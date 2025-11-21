use std::fs::{File, OpenOptions};
use std::io::{self, BufReader, Write, Read};
use std::path::Path;
use std::sync::Arc;
use parking_lot::Mutex;
use tracing::{info, error};
use crate::database::DB;
use crate::resp::RespValue;

pub struct Aof {
    file: File,
}

impl Aof {
    pub fn new(path: impl AsRef<Path>) -> io::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(path)?;
        
        Ok(Aof { file })
    }

    pub fn append(&mut self, command: Vec<String>) -> io::Result<()> {
        // Convert command to RESP format
        let resp_args: Vec<RespValue> = command.into_iter()
            .map(|s| RespValue::BulkString(Some(s)))
            .collect();
        
        let resp = RespValue::Array(Some(resp_args));
        let serialized = resp.serialize();
        
        self.file.write_all(serialized.as_bytes())?;
        self.file.flush()?; // Ensure it's written to disk
        Ok(())
    }

    pub fn load(path: impl AsRef<Path>, db: &Arc<Mutex<DB>>) -> io::Result<()> {
        if !path.as_ref().exists() {
            return Ok(());
        }

        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut buffer = Vec::new();
        
        // This is a simplified loader. 
        // In a real implementation, we would use RespHandler to parse the stream properly.
        // But since we write line-based RESP (mostly), we can try to parse it.
        // Actually, using RespHandler is better.
        
        // Let's read the whole file into memory for simplicity (not ideal for large AOF)
        reader.read_to_end(&mut buffer)?;
        
        let mut current_pos = 0;
        let mut count = 0;
        
        use crate::resp::RespHandler;
        
        while current_pos < buffer.len() {
            match RespHandler::parse_request(&buffer[current_pos..]) {
                Ok(Some((value, len))) => {
                    current_pos += len;
                    
                    // Convert RESP value to arguments
                    let args = match value {
                        RespValue::Array(Some(items)) => {
                            items.into_iter().filter_map(|item| {
                                match item {
                                    RespValue::BulkString(Some(s)) => Some(s),
                                    RespValue::SimpleString(s) => Some(s),
                                    _ => None,
                                }
                            }).collect::<Vec<String>>()
                        },
                        _ => Vec::new(),
                    };

                    if !args.is_empty() {
                        // Replay command
                        // We need to create a temporary interpreter or just call DB methods directly.
                        // Calling DB methods directly is safer/faster for replay.
                        
                        let cmd = args[0].to_uppercase();
                        let mut db_guard = db.lock();
                        
                        if cmd == "SET" && args.len() >= 3 {
                            db_guard.set(args[1].clone(), args[2].clone());
                        } else if cmd == "DEL" && args.len() >= 2 {
                            db_guard.del(args[1].clone());
                        } else if cmd == "INCR" && args.len() >= 2 {
                            let _ = db_guard.incr(args[1].clone());
                        } else if cmd == "DECR" && args.len() >= 2 {
                            let _ = db_guard.decr(args[1].clone());
                        } else if cmd == "EXPIRE" && args.len() >= 3 {
                            if let Ok(secs) = args[2].parse::<u64>() {
                                db_guard.expire(args[1].clone(), secs);
                            }
                        } else if cmd == "PERSIST" && args.len() >= 2 {
                            db_guard.persist(args[1].clone());
                        } else if (cmd == "LPUSH" || cmd == "RPUSH") && args.len() >= 3 {
                            let values = args[2..].to_vec();
                            if cmd == "LPUSH" {
                                let _ = db_guard.lpush_safe(args[1].clone(), values);
                            } else {
                                let _ = db_guard.rpush(args[1].clone(), values);
                            }
                        } else if (cmd == "LPOP" || cmd == "RPOP") && args.len() >= 2 {
                            if cmd == "LPOP" {
                                let _ = db_guard.lpop(args[1].clone());
                            } else {
                                let _ = db_guard.rpop(args[1].clone());
                            }
                        }
                        // Note: We don't replay read commands like GET, KEYS, etc.
                        
                        count += 1;
                    }
                },
                Ok(None) => break, // Incomplete or end
                Err(e) => {
                    error!("Error parsing AOF: {}", e);
                    break;
                }
            }
        }
        
        info!("Loaded {} commands from AOF", count);
        Ok(())
    }
}
