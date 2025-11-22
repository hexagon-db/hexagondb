use crate::commands::{ExecutionResult, Interpreter};
use crate::network::resp::{RespHandler, RespValue};
use crate::observability::metrics::{METRIC_ACTIVE_CONNECTIONS, METRIC_CONNECTIONS_TOTAL};
use metrics::{counter, gauge};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::{debug, error, info, instrument, Instrument};
use uuid::Uuid;

struct ConnectionGuard;

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        gauge!(METRIC_ACTIVE_CONNECTIONS).decrement(1.0);
    }
}

/// Her bir istemci bağlantısını işler.
/// Gelen veriyi buffer'a alır, RESP formatında parse eder, komutu işler ve cevap gönderir.
#[instrument(skip(stream, client), fields(connection_id = %Uuid::new_v4()))]
pub async fn handle_client(mut stream: TcpStream, client: &mut Interpreter) {
    counter!(METRIC_CONNECTIONS_TOTAL).increment(1);
    gauge!(METRIC_ACTIVE_CONNECTIONS).increment(1.0);
    let _guard = ConnectionGuard;

    info!("New connection established");

    // Sabit buffer yerine dinamik bir buffer kullanıyoruz.
    // Bu sayede parça parça gelen verileri birleştirebiliriz.
    let mut buffer = Vec::new();
    let mut temp_buf = [0u8; 1024]; // Ağdan okuma yapmak için geçici buffer

    loop {
        match stream.read(&mut temp_buf).await {
            Ok(bytes_read) => {
                if bytes_read == 0 {
                    debug!("Client closed the connection");
                    return;
                }

                // Okunan veriyi ana buffer'a ekle
                buffer.extend_from_slice(&temp_buf[..bytes_read]);

                // Pipelining desteği: Tüm mevcut komutları işle
                let mut responses = Vec::new();

                loop {
                    // Buffer boşsa döngüden çık, yeni veri bekle
                    if buffer.is_empty() {
                        break;
                    }

                    // Gelen veriyi RESP formatında parse etmeye çalış
                    match RespHandler::parse_request(&buffer) {
                        Ok(Some((request, len))) => {
                            // Başarılı bir şekilde tam bir komut parse edildi

                            // Komutu çalıştır
                            let request_id = Uuid::new_v4();
                            let span = tracing::info_span!("request", %request_id);

                            match client.execute(request).instrument(span).await {
                                ExecutionResult::Response(response) => {
                                    // Cevabı topla (pipelining için)
                                    responses.push(response);
                                }
                                ExecutionResult::Subscribe(channel, mut receiver) => {
                                    // Abonelik moduna geç
                                    // İlk olarak abonelik onayını gönder
                                    let success_resp = RespValue::Array(Some(vec![
                                        RespValue::BulkString(Some("subscribe".to_string())),
                                        RespValue::BulkString(Some(channel.clone())),
                                        RespValue::Integer(1),
                                    ]));

                                    let response_bytes = success_resp.serialize();
                                    if let Err(e) =
                                        stream.write_all(response_bytes.as_bytes()).await
                                    {
                                        error!("Failed to send subscribe response: {}", e);
                                        return;
                                    }

                                    // Abonelik döngüsü
                                    // Hem kanaldan gelen mesajları hem de istemciden gelen komutları dinliyoruz.
                                    loop {
                                        tokio::select! {
                                            // 1. Kanaldan gelen mesajlar
                                            msg = receiver.recv() => {
                                                match msg {
                                                    Ok(msg_content) => {
                                                        let push_msg = RespValue::Array(Some(vec![
                                                            RespValue::BulkString(Some("message".to_string())),
                                                            RespValue::BulkString(Some(channel.clone())),
                                                            RespValue::BulkString(Some(msg_content)),
                                                        ]));

                                                        let push_bytes = push_msg.serialize();
                                                        if let Err(e) = stream.write_all(push_bytes.as_bytes()).await {
                                                            error!("Failed to send push message: {}", e);
                                                            break;
                                                        }
                                                    }
                                                    Err(e) => {
                                                        // Kanal kapandı veya hata oluştu
                                                        error!("Broadcast receive error: {}", e);
                                                        break;
                                                    }
                                                }
                                            }

                                            // 2. İstemciden gelen veriler (UNSUBSCRIBE, QUIT vb.)
                                            read_result = stream.read(&mut temp_buf) => {
                                                match read_result {
                                                    Ok(0) => {
                                                        // Bağlantı koptu
                                                        debug!("Client closed connection during subscribe");
                                                        break;
                                                    }
                                                    Ok(n) => {
                                                        // Veriyi buffer'a ekle
                                                        buffer.extend_from_slice(&temp_buf[..n]);

                                                        // Buffer'daki komutları işle
                                                        // Not: Basitlik için burada sadece buffer'ın başındaki komuta bakıyoruz.
                                                        // Gerçek bir implementasyonda döngü içinde tüm komutları işlemeliyiz.
                                                        match RespHandler::parse_request(&buffer) {
                                                            Ok(Some((request, len))) => {
                                                                // Buffer'dan işlenen kısmı sil
                                                                buffer.drain(0..len);

                                                                if let RespValue::Array(Some(tokens)) = &request {
                                                                    if !tokens.is_empty() {
                                                                        if let RespValue::BulkString(Some(cmd)) = &tokens[0] {
                                                                            let cmd_upper = cmd.to_uppercase();
                                                                            if cmd_upper == "UNSUBSCRIBE" || cmd_upper == "QUIT" {
                                                                                // Döngüden çık, normal moda dön veya bağlantıyı kapat
                                                                                // UNSUBSCRIBE durumunda normal moda dönmek gerekebilir ama şimdilik çıkıyoruz.
                                                                                break;
                                                                            } else if cmd_upper == "PING" {
                                                                                // PONG gönder
                                                                                let pong = RespValue::SimpleString("PONG".to_string());
                                                                                if let Err(e) = stream.write_all(pong.serialize().as_bytes()).await {
                                                                                     error!("Failed to send PONG: {}", e);
                                                                                     break;
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                            Ok(None) => {
                                                                // Veri eksik, devam et
                                                            }
                                                            Err(e) => {
                                                                error!("Failed to parse request in subscribe mode: {}", e);
                                                                break;
                                                            }
                                                        }
                                                    }
                                                    Err(e) => {
                                                        error!("Failed to read from socket in subscribe mode: {}", e);
                                                        break;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    // Döngüden çıkınca fonksiyon bitiyor ve bağlantı kapanıyor.
                                    // Normalde UNSUBSCRIBE sonrası normal moda dönmek gerekir (recursive call veya loop yapısı değişikliği ile).
                                    return;
                                }
                            }

                            // İşlenen kısmı buffer'dan sil (drain)
                            buffer.drain(0..len);
                        }
                        Ok(None) => {
                            // Veri eksik, daha fazla veri bekle
                            break;
                        }
                        Err(e) => {
                            error!("Failed to parse request: {}", e);
                            // Hatalı veriyi temizle veya bağlantıyı kapat
                            return;
                        }
                    }
                }

                // Pipelining: Tüm cevapları birlikte gönder
                if !responses.is_empty() {
                    for response in responses {
                        let response_bytes = response.serialize();
                        if let Err(e) = stream.write_all(response_bytes.as_bytes()).await {
                            error!("Failed to send pipelined response: {}", e);
                            return;
                        }
                    }
                }
            }
            Err(e) => {
                error!("Failed to read from socket: {}", e);
                return;
            }
        }
    }
}
