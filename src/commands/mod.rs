use crate::config::Config;
use crate::db::pubsub::PubSub;
use crate::db::DB;
use crate::db::{GenericOps, HashOps, ListOps, SetOps, StringOps};
use crate::network::resp::RespValue;
use crate::observability::metrics::{METRIC_COMMANDS_TOTAL, METRIC_COMMAND_LATENCY};
use crate::persistence::aof::Aof;
use crate::server_info::ServerInfo;
use metrics::{counter, histogram};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::error;

/// İstemciden gelen komutları işleyen birim.
/// Her bağlantı için bir Interpreter oluşturulur.
pub struct Interpreter {
    db: Arc<RwLock<DB>>,
    aof: Arc<RwLock<Aof>>,
    server_info: Arc<ServerInfo>,
    #[allow(dead_code)]
    config: Arc<RwLock<Config>>,
    pubsub: Arc<PubSub>,
}

use tokio::sync::broadcast;

pub enum ExecutionResult {
    Response(RespValue),
    Subscribe(String, broadcast::Receiver<String>),
}

struct LatencyGuard {
    start: std::time::Instant,
}

impl Drop for LatencyGuard {
    fn drop(&mut self) {
        let duration = self.start.elapsed();
        histogram!(METRIC_COMMAND_LATENCY).record(duration.as_secs_f64());
    }
}

impl Interpreter {
    /// Yeni bir yorumlayıcı oluşturur.
    /// Veritabanı ve AOF (Persistence) modüllerine erişimi vardır.
    pub fn new(
        db: Arc<RwLock<DB>>,
        aof: Arc<RwLock<Aof>>,
        server_info: Arc<ServerInfo>,
        config: Arc<RwLock<Config>>,
        pubsub: Arc<PubSub>,
    ) -> Self {
        Interpreter {
            db,
            aof,
            server_info,
            config,
            pubsub,
        }
    }

    /// İstemciden gelen komutu işler ve cevabı döndürür.
    #[tracing::instrument(skip(self, request), fields(cmd, key))]
    pub async fn execute(&mut self, request: RespValue) -> ExecutionResult {
        counter!(METRIC_COMMANDS_TOTAL).increment(1);
        let _guard = LatencyGuard {
            start: std::time::Instant::now(),
        };

        match request {
            RespValue::Array(Some(tokens)) => {
                if tokens.is_empty() {
                    return ExecutionResult::Response(RespValue::Error(
                        "empty command".to_string(),
                    ));
                }

                // İlk eleman komut ismidir (SET, GET vs.)
                let cmd_string = match &tokens[0] {
                    RespValue::BulkString(Some(s)) => s.clone(),
                    RespValue::SimpleString(s) => s.clone(),
                    _ => {
                        return ExecutionResult::Response(RespValue::Error(
                            "invalid command format".to_string(),
                        ));
                    }
                };

                let cmd_upper = cmd_string.to_uppercase();
                tracing::Span::current().record("cmd", &cmd_upper);
                tracing::info!("Processing command");
                let args: Vec<String> = tokens
                    .iter()
                    .skip(1)
                    .filter_map(|t| match t {
                        RespValue::BulkString(Some(s)) => Some(s.clone()),
                        _ => None,
                    })
                    .collect();

                // Komut argümanlarını string listesine çevir (AOF için lazım)
                let mut full_cmd_args = vec![cmd_string.clone()];
                full_cmd_args.extend(args.clone());

                // --- Komutları İşle ---

                if cmd_upper == "PING" {
                    return ExecutionResult::Response(RespValue::SimpleString("PONG".to_string()));
                }

                if cmd_upper == "ECHO" {
                    if let Some(arg) = args.get(0) {
                        return ExecutionResult::Response(RespValue::BulkString(Some(arg.clone())));
                    } else {
                        return ExecutionResult::Response(RespValue::Error(
                            "wrong number of arguments for 'ECHO' command".to_string(),
                        ));
                    }
                }

                if cmd_upper == "INFO" {
                    let db_guard = self.db.read().await;
                    let db_size = db_guard.items.len();
                    drop(db_guard);

                    let info_str = self.server_info.generate_info(db_size);
                    return ExecutionResult::Response(RespValue::BulkString(Some(info_str)));
                }

                // Anahtar gerektiren komutlar için kontrol
                let key = if let Some(k) = args.get(0) {
                    k.clone()
                } else {
                    // Bazı komutlar anahtar istemez (PING, ECHO, KEYS *)
                    // Ama aşağıdakiler ister.
                    if [
                        "GET",
                        "SET",
                        "DEL",
                        "EXISTS",
                        "INCR",
                        "DECR",
                        "EXPIRE",
                        "TTL",
                        "PERSIST",
                        "LPUSH",
                        "RPUSH",
                        "LPOP",
                        "RPOP",
                        "LLEN",
                        "LRANGE",
                        "HSET",
                        "HGET",
                        "HGETALL",
                        "HDEL",
                        "SADD",
                        "SREM",
                        "SMEMBERS",
                        "SISMEMBER",
                        "SCARD",
                    ]
                    .contains(&cmd_upper.as_str())
                    {
                        return ExecutionResult::Response(RespValue::Error(format!(
                            "wrong number of arguments for '{}' command",
                            cmd_upper
                        )));
                    }
                    "".to_string()
                };

                if cmd_upper == "GET" {
                    let mut db = self.db.write().await;
                    return match db.get(key) {
                        Ok(Some(value)) => {
                            ExecutionResult::Response(RespValue::BulkString(Some(value)))
                        }
                        Ok(None) => ExecutionResult::Response(RespValue::BulkString(None)),
                        Err(e) => ExecutionResult::Response(RespValue::Error(e)),
                    };
                } else if cmd_upper == "SET" {
                    if let Some(value) = args.get(1) {
                        let mut db = self.db.write().await;
                        db.set(key, value.clone());

                        // AOF'a kaydet (Kalıcılık)
                        let mut aof = self.aof.write().await;
                        if let Err(e) = aof.append(full_cmd_args) {
                            error!("AOF write error: {}", e);
                        }

                        return ExecutionResult::Response(RespValue::SimpleString(
                            "OK".to_string(),
                        ));
                    } else {
                        return ExecutionResult::Response(RespValue::Error(
                            "wrong number of arguments for 'SET' command".to_string(),
                        ));
                    }
                } else if cmd_upper == "DEL" {
                    let mut db = self.db.write().await;
                    db.del(key);

                    let mut aof = self.aof.write().await;
                    if let Err(e) = aof.append(full_cmd_args) {
                        error!("AOF write error: {}", e);
                    }

                    return ExecutionResult::Response(RespValue::Integer(1));
                } else if cmd_upper == "EXISTS" {
                    let db = self.db.read().await;
                    let exists = db.exists(key);
                    return ExecutionResult::Response(RespValue::Integer(if exists {
                        1
                    } else {
                        0
                    }));
                } else if cmd_upper == "KEYS" {
                    if let Some(pattern) = args.get(0) {
                        let db = self.db.read().await;
                        let keys = db.keys(pattern.clone());
                        let resp_keys: Vec<RespValue> = keys
                            .into_iter()
                            .map(|k| RespValue::BulkString(Some(k)))
                            .collect();
                        return ExecutionResult::Response(RespValue::Array(Some(resp_keys)));
                    } else {
                        return ExecutionResult::Response(RespValue::Error(
                            "wrong number of arguments for 'KEYS' command".to_string(),
                        ));
                    }
                } else if cmd_upper == "INCR" {
                    let mut db = self.db.write().await;
                    match db.incr(key) {
                        Ok(val) => {
                            let mut aof = self.aof.write().await;
                            if let Err(e) = aof.append(full_cmd_args) {
                                error!("AOF write error: {}", e);
                            }
                            return ExecutionResult::Response(RespValue::Integer(val));
                        }
                        Err(e) => return ExecutionResult::Response(RespValue::Error(e)),
                    }
                } else if cmd_upper == "DECR" {
                    let mut db = self.db.write().await;
                    match db.decr(key) {
                        Ok(val) => {
                            let mut aof = self.aof.write().await;
                            if let Err(e) = aof.append(full_cmd_args) {
                                error!("AOF write error: {}", e);
                            }
                            return ExecutionResult::Response(RespValue::Integer(val));
                        }
                        Err(e) => return ExecutionResult::Response(RespValue::Error(e)),
                    }
                } else if cmd_upper == "LPUSH" || cmd_upper == "RPUSH" {
                    if args.len() < 2 {
                        return ExecutionResult::Response(RespValue::Error(format!(
                            "wrong number of arguments for '{}' command",
                            cmd_upper
                        )));
                    }
                    let values = args[1..].to_vec();
                    let mut db = self.db.write().await;

                    let result = if cmd_upper == "LPUSH" {
                        db.lpush_safe(key, values)
                    } else {
                        db.rpush(key, values)
                    };

                    match result {
                        Ok(len) => {
                            let mut aof = self.aof.write().await;
                            if let Err(e) = aof.append(full_cmd_args) {
                                error!("AOF write error: {}", e);
                            }
                            return ExecutionResult::Response(RespValue::Integer(len as i64));
                        }
                        Err(e) => return ExecutionResult::Response(RespValue::Error(e)),
                    }
                } else if cmd_upper == "LPOP" || cmd_upper == "RPOP" {
                    let mut db = self.db.write().await;
                    let result = if cmd_upper == "LPOP" {
                        db.lpop(key)
                    } else {
                        db.rpop(key)
                    };

                    match result {
                        Ok(Some(val)) => {
                            let mut aof = self.aof.write().await;
                            if let Err(e) = aof.append(full_cmd_args) {
                                error!("AOF write error: {}", e);
                            }
                            return ExecutionResult::Response(RespValue::BulkString(Some(val)));
                        }
                        Ok(None) => return ExecutionResult::Response(RespValue::BulkString(None)),
                        Err(e) => return ExecutionResult::Response(RespValue::Error(e)),
                    }
                } else if cmd_upper == "LLEN" {
                    let mut db = self.db.write().await;
                    match db.llen(key) {
                        Ok(len) => {
                            return ExecutionResult::Response(RespValue::Integer(len as i64))
                        }
                        Err(e) => return ExecutionResult::Response(RespValue::Error(e)),
                    }
                } else if cmd_upper == "LRANGE" {
                    if args.len() != 3 {
                        return ExecutionResult::Response(RespValue::Error(
                            "wrong number of arguments for 'LRANGE' command".to_string(),
                        ));
                    }
                    let start_str = &args[1];
                    let stop_str = &args[2];

                    match (start_str.parse::<i64>(), stop_str.parse::<i64>()) {
                        (Ok(start), Ok(stop)) => {
                            let mut db = self.db.write().await;
                            match db.lrange(key, start, stop) {
                                Ok(values) => {
                                    let resp_values: Vec<RespValue> = values
                                        .into_iter()
                                        .map(|s| RespValue::BulkString(Some(s)))
                                        .collect();
                                    return ExecutionResult::Response(RespValue::Array(Some(
                                        resp_values,
                                    )));
                                }
                                Err(e) => return ExecutionResult::Response(RespValue::Error(e)),
                            }
                        }
                        _ => {
                            return ExecutionResult::Response(RespValue::Error(
                                "value is not an integer or out of range".to_string(),
                            ));
                        }
                    }
                } else if cmd_upper == "HSET" {
                    if args.len() != 3 {
                        return ExecutionResult::Response(RespValue::Error(
                            "wrong number of arguments for 'HSET' command".to_string(),
                        ));
                    }
                    let field = args[1].clone();
                    let value = args[2].clone();

                    let mut db = self.db.write().await;
                    match db.hset(key, field, value) {
                        Ok(val) => {
                            let mut aof = self.aof.write().await;
                            if let Err(e) = aof.append(full_cmd_args) {
                                error!("AOF write error: {}", e);
                            }
                            return ExecutionResult::Response(RespValue::Integer(val as i64));
                        }
                        Err(e) => return ExecutionResult::Response(RespValue::Error(e)),
                    }
                } else if cmd_upper == "HGET" {
                    if args.len() != 2 {
                        return ExecutionResult::Response(RespValue::Error(
                            "wrong number of arguments for 'HGET' command".to_string(),
                        ));
                    }
                    let field = args[1].clone();

                    let mut db = self.db.write().await;
                    match db.hget(key, field) {
                        Ok(Some(val)) => {
                            return ExecutionResult::Response(RespValue::BulkString(Some(val)))
                        }
                        Ok(None) => return ExecutionResult::Response(RespValue::BulkString(None)),
                        Err(e) => return ExecutionResult::Response(RespValue::Error(e)),
                    }
                } else if cmd_upper == "HGETALL" {
                    let mut db = self.db.write().await;
                    match db.hgetall(key) {
                        Ok(values) => {
                            let resp_values: Vec<RespValue> = values
                                .into_iter()
                                .map(|s| RespValue::BulkString(Some(s)))
                                .collect();
                            return ExecutionResult::Response(RespValue::Array(Some(resp_values)));
                        }
                        Err(e) => return ExecutionResult::Response(RespValue::Error(e)),
                    }
                } else if cmd_upper == "HDEL" {
                    if args.len() != 2 {
                        return ExecutionResult::Response(RespValue::Error(
                            "wrong number of arguments for 'HDEL' command".to_string(),
                        ));
                    }
                    let field = args[1].clone();

                    let mut db = self.db.write().await;
                    match db.hdel(key, field) {
                        Ok(val) => {
                            let mut aof = self.aof.write().await;
                            if let Err(e) = aof.append(full_cmd_args) {
                                error!("AOF write error: {}", e);
                            }
                            return ExecutionResult::Response(RespValue::Integer(val as i64));
                        }
                        Err(e) => return ExecutionResult::Response(RespValue::Error(e)),
                    }
                } else if cmd_upper == "EXPIRE" {
                    if let Some(seconds_str) = args.get(1) {
                        if let Ok(seconds) = seconds_str.parse::<u64>() {
                            let mut db = self.db.write().await;
                            let result = db.expire(key, seconds);

                            if result {
                                let mut aof = self.aof.write().await;
                                if let Err(e) = aof.append(full_cmd_args) {
                                    error!("AOF write error: {}", e);
                                }
                            }

                            return ExecutionResult::Response(RespValue::Integer(if result {
                                1
                            } else {
                                0
                            }));
                        } else {
                            return ExecutionResult::Response(RespValue::Error(
                                "value is not an integer or out of range".to_string(),
                            ));
                        }
                    } else {
                        return ExecutionResult::Response(RespValue::Error(
                            "wrong number of arguments for 'EXPIRE' command".to_string(),
                        ));
                    }
                } else if cmd_upper == "TTL" {
                    let mut db = self.db.write().await;
                    let ttl = db.ttl(key);
                    return ExecutionResult::Response(RespValue::Integer(ttl));
                } else if cmd_upper == "PERSIST" {
                    let mut db = self.db.write().await;
                    let result = db.persist(key);

                    if result {
                        let mut aof = self.aof.write().await;
                        if let Err(e) = aof.append(full_cmd_args) {
                            error!("AOF write error: {}", e);
                        }
                    }

                    return ExecutionResult::Response(RespValue::Integer(if result {
                        1
                    } else {
                        0
                    }));
                } else if cmd_upper == "SADD" {
                    if args.len() < 2 {
                        return ExecutionResult::Response(RespValue::Error(
                            "wrong number of arguments for 'SADD' command".to_string(),
                        ));
                    }
                    let members = args[1..].to_vec();
                    let mut db = self.db.write().await;
                    match db.sadd(key, members) {
                        Ok(added) => {
                            let mut aof = self.aof.write().await;
                            if let Err(e) = aof.append(full_cmd_args) {
                                error!("AOF write error: {}", e);
                            }
                            return ExecutionResult::Response(RespValue::Integer(added as i64));
                        }
                        Err(e) => return ExecutionResult::Response(RespValue::Error(e)),
                    }
                } else if cmd_upper == "SREM" {
                    if args.len() != 2 {
                        return ExecutionResult::Response(RespValue::Error(
                            "wrong number of arguments for 'SREM' command".to_string(),
                        ));
                    }
                    let member = args[1].clone();
                    let mut db = self.db.write().await;
                    match db.srem(key, member) {
                        Ok(removed) => {
                            let mut aof = self.aof.write().await;
                            if let Err(e) = aof.append(full_cmd_args) {
                                error!("AOF write error: {}", e);
                            }
                            return ExecutionResult::Response(RespValue::Integer(removed as i64));
                        }
                        Err(e) => return ExecutionResult::Response(RespValue::Error(e)),
                    }
                } else if cmd_upper == "SMEMBERS" {
                    let mut db = self.db.write().await;
                    match db.smembers(key) {
                        Ok(members) => {
                            let resp_members: Vec<RespValue> = members
                                .into_iter()
                                .map(|m| RespValue::BulkString(Some(m)))
                                .collect();
                            return ExecutionResult::Response(RespValue::Array(Some(resp_members)));
                        }
                        Err(e) => return ExecutionResult::Response(RespValue::Error(e)),
                    }
                } else if cmd_upper == "SISMEMBER" {
                    if args.len() != 2 {
                        return ExecutionResult::Response(RespValue::Error(
                            "wrong number of arguments for 'SISMEMBER' command".to_string(),
                        ));
                    }
                    let member = args[1].clone();
                    let mut db = self.db.write().await;
                    match db.sismember(key, member) {
                        Ok(exists) => {
                            return ExecutionResult::Response(RespValue::Integer(if exists {
                                1
                            } else {
                                0
                            }))
                        }
                        Err(e) => return ExecutionResult::Response(RespValue::Error(e)),
                    }
                } else if cmd_upper == "SCARD" {
                    let mut db = self.db.write().await;
                    match db.scard(key) {
                        Ok(count) => {
                            return ExecutionResult::Response(RespValue::Integer(count as i64))
                        }
                        Err(e) => return ExecutionResult::Response(RespValue::Error(e)),
                    }
                } else if cmd_upper == "PUBLISH" {
                    if tokens.len() != 3 {
                        return ExecutionResult::Response(RespValue::Error(
                            "ERR wrong number of arguments for 'publish' command".to_string(),
                        ));
                    }

                    let channel = match &tokens[1] {
                        RespValue::BulkString(Some(s)) => s.clone(),
                        RespValue::SimpleString(s) => s.clone(),
                        _ => {
                            return ExecutionResult::Response(RespValue::Error(
                                "ERR channel name must be a string".to_string(),
                            ));
                        }
                    };

                    let message = match &tokens[2] {
                        RespValue::BulkString(Some(s)) => s.clone(),
                        RespValue::SimpleString(s) => s.clone(),
                        _ => {
                            return ExecutionResult::Response(RespValue::Error(
                                "ERR message must be a string".to_string(),
                            ))
                        }
                    };

                    let count = self.pubsub.publish(&channel, &message).await;
                    return ExecutionResult::Response(RespValue::Integer(count as i64));
                } else if cmd_upper == "SUBSCRIBE" {
                    // SUBSCRIBE komutu özeldir: Bağlantıyı bloklar ve mesajları dinler.
                    // İstemci "abone" moduna geçer ve sadece pub/sub komutlarını gönderebilir.
                    // ExecutionResult::Subscribe döndürerek bağlantı yöneticisinin (connection handler)
                    // yayın akışı (streaming) moduna geçmesini sağlarız.

                    let channel_name = match &tokens[1] {
                        RespValue::BulkString(Some(s)) => s.clone(),
                        RespValue::SimpleString(s) => s.clone(),
                        _ => {
                            return ExecutionResult::Response(RespValue::Error(
                                "ERR channel name must be a string".to_string(),
                            ))
                        }
                    };

                    let receiver = self.pubsub.subscribe(&channel_name).await;
                    return ExecutionResult::Subscribe(channel_name, receiver);
                } else if cmd_upper == "SAVE" {
                    // Synchronous snapshot save
                    use crate::persistence::snapshot;
                    match snapshot::save("dump.rdb", &self.db).await {
                        Ok(_) => {
                            return ExecutionResult::Response(RespValue::SimpleString(
                                "OK".to_string(),
                            ))
                        }
                        Err(e) => {
                            return ExecutionResult::Response(RespValue::Error(format!(
                                "Failed to save snapshot: {}",
                                e
                            )))
                        }
                    }
                } else if cmd_upper == "BGSAVE" {
                    // Background snapshot save
                    let db_clone = Arc::clone(&self.db);

                    tokio::spawn(async move {
                        use crate::persistence::snapshot;
                        use tracing::{error, info};
                        if let Err(e) = snapshot::save("dump.rdb", &db_clone).await {
                            error!("Background save failed: {}", e);
                        } else {
                            info!("Background save completed successfully");
                        }
                    });

                    return ExecutionResult::Response(RespValue::SimpleString(
                        "Background saving started".to_string(),
                    ));
                } else {
                    return ExecutionResult::Response(RespValue::Error(format!(
                        "unknown command '{}'",
                        cmd_string
                    )));
                }
            }
            _ => ExecutionResult::Response(RespValue::Error("invalid command format".to_string())),
        }
    }
}
