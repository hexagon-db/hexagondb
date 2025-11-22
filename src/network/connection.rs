use crate::{
    interpreter,
    resp::{RespHandler, RespValue},
};
use std::io::{Read, Write};
use std::net::TcpStream;
use tracing::{debug, error};

pub fn handle_client(mut stream: TcpStream, interpreter: &mut interpreter::Interpreter) {
    let mut buffer = [0u8; 4096];

    loop {
        let n = match stream.read(&mut buffer) {
            Ok(0) => return,
            Ok(n) => n,
            Err(e) => {
                error!("Error reading from stream: {}", e);
                return;
            }
        };

        let mut current_pos = 0;
        while current_pos < n {
            match RespHandler::parse_request(&buffer[current_pos..n]) {
                Ok(Some((value, len))) => {
                    current_pos += len;

                    // Convert RESP value to arguments
                    let args = match value {
                        RespValue::Array(Some(items)) => items
                            .into_iter()
                            .filter_map(|item| match item {
                                RespValue::BulkString(Some(s)) => Some(s),
                                RespValue::SimpleString(s) => Some(s),
                                _ => None,
                            })
                            .collect(),
                        _ => Vec::new(), // Should not happen with valid commands
                    };

                    if args.is_empty() {
                        continue;
                    }

                    let response = interpreter.exec_args(args);
                    let response_bytes = response.serialize();

                    if let Err(e) = stream.write_all(response_bytes.as_bytes()) {
                        error!("Error sending response: {}", e);
                        return;
                    }
                }
                Ok(None) => {
                    // Incomplete command - waiting for more data from client
                    // This is normal when commands span multiple TCP packets
                    debug!("Incomplete command received, waiting for more data");
                    break;
                }
                Err(e) => {
                    error!("Protocol error: {}", e);
                    let err_resp = RespValue::Error(format!("Protocol error: {}", e));
                    let _ = stream.write_all(err_resp.serialize().as_bytes());
                    return;
                }
            }
        }
    }
}
