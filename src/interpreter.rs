use std::sync::Arc;
use parking_lot::Mutex;
use tracing::{debug, warn, error};
use crate::{database::DB, parse_query, resp::RespValue, aof::Aof};

pub struct Interpreter {
    db: Arc<Mutex<DB>>,
    aof: Arc<Mutex<Aof>>,
}
impl Interpreter {
    pub fn new(db: Arc<Mutex<DB>>, aof: Arc<Mutex<Aof>>) -> Self {
        Interpreter {
            db,
            aof,
        }
    }

    pub fn exec(&mut self, query: String) -> String {
        let tokens: Vec<String> = parse_query::parse_query(query.to_string());
        let result = self.exec_args(tokens);
        result.serialize()
    }

    pub fn exec_args(&mut self, tokens: Vec<String>) -> RespValue {
        debug!("Executing command: {:?}", tokens);

        if let Some(cmd) = tokens.get(0).cloned() {
            let cmd_upper = cmd.to_uppercase();
            
            // Handle KEYS command
            if cmd_upper == "KEYS" {
                if let Some(pattern) = tokens.get(1).cloned() {
                    let db = self.db.lock();
                    let keys = db.keys(pattern);
                    // Convert Vec<String> to RespValue::Array
                    let resp_keys: Vec<RespValue> = keys.into_iter()
                        .map(|k| RespValue::BulkString(Some(k)))
                        .collect();
                    return RespValue::Array(Some(resp_keys));
                } else {
                    return RespValue::Error("wrong number of arguments for 'KEYS' command".to_string());
                }
            }
            
            if let Some(item) = tokens.get(1).cloned() {
                if cmd_upper == "GET" {
                    let mut db = self.db.lock();
                    return match db.get(item) {
                        Ok(Some(value)) => RespValue::BulkString(Some(value)),
                        Ok(None) => RespValue::BulkString(None),
                        Err(e) => RespValue::Error(e),
                    };
                } else if cmd_upper == "SET" {
                    if let Some(value) = tokens.get(2).cloned() {
                        let mut db = self.db.lock();
                        db.set(item, value);
                        
                        // Persist to AOF
                        let mut aof = self.aof.lock();
                        if let Err(e) = aof.append(tokens.clone()) {
                            error!("Failed to append to AOF: {}", e);
                        }
                        
                        return RespValue::SimpleString("OK".to_string());
                    } else {
                        return RespValue::Error("wrong number of arguments for 'SET' command".to_string());
                    }
                } else if cmd_upper == "DEL" {
                    let mut db = self.db.lock();
                    db.del(item);
                    
                    // Persist to AOF
                    let mut aof = self.aof.lock();
                    if let Err(e) = aof.append(tokens.clone()) {
                        error!("Failed to append to AOF: {}", e);
                    }
                    
                    return RespValue::Integer(1);
                } else if cmd_upper == "EXISTS" {
                    let db = self.db.lock();
                    let exists = db.exists(item);
                    return RespValue::Integer(if exists { 1 } else { 0 });
                } else if cmd_upper == "INCR" {
                    let mut db = self.db.lock();
                    match db.incr(item) {
                        Ok(val) => {
                            // Persist to AOF
                            let mut aof = self.aof.lock();
                            if let Err(e) = aof.append(tokens.clone()) {
                                error!("Failed to append to AOF: {}", e);
                            }
                            return RespValue::Integer(val);
                        },
                        Err(e) => return RespValue::Error(e),
                    }
                } else if cmd_upper == "DECR" {
                    let mut db = self.db.lock();
                    match db.decr(item) {
                        Ok(val) => {
                            // Persist to AOF
                            let mut aof = self.aof.lock();
                            if let Err(e) = aof.append(tokens.clone()) {
                                error!("Failed to append to AOF: {}", e);
                            }
                            return RespValue::Integer(val);
                        },
                        Err(e) => return RespValue::Error(e),
                    }
                } else if cmd_upper == "LPUSH" || cmd_upper == "RPUSH" {
                    if tokens.len() < 3 {
                        return RespValue::Error(format!("wrong number of arguments for '{}' command", cmd_upper));
                    }
                    let values = tokens[2..].to_vec();
                    let mut db = self.db.lock();
                    
                    let result = if cmd_upper == "LPUSH" {
                        db.lpush_safe(item, values)
                    } else {
                        db.rpush(item, values)
                    };

                    match result {
                        Ok(len) => {
                            // Persist to AOF
                            let mut aof = self.aof.lock();
                            if let Err(e) = aof.append(tokens.clone()) {
                                error!("Failed to append to AOF: {}", e);
                            }
                            return RespValue::Integer(len as i64);
                        },
                        Err(e) => return RespValue::Error(e),
                    }
                } else if cmd_upper == "LPOP" || cmd_upper == "RPOP" {
                    let mut db = self.db.lock();
                    let result = if cmd_upper == "LPOP" {
                        db.lpop(item)
                    } else {
                        db.rpop(item)
                    };

                    match result {
                        Ok(Some(val)) => {
                            // Persist to AOF
                            let mut aof = self.aof.lock();
                            if let Err(e) = aof.append(tokens.clone()) {
                                error!("Failed to append to AOF: {}", e);
                            }
                            return RespValue::BulkString(Some(val));
                        },
                        Ok(None) => return RespValue::BulkString(None),
                        Err(e) => return RespValue::Error(e),
                    }
                } else if cmd_upper == "LLEN" {
                    let mut db = self.db.lock();
                    match db.llen(item) {
                        Ok(len) => return RespValue::Integer(len as i64),
                        Err(e) => return RespValue::Error(e),
                    }
                } else if cmd_upper == "LRANGE" {
                    if tokens.len() != 4 {
                        return RespValue::Error("wrong number of arguments for 'LRANGE' command".to_string());
                    }
                    let start_str = &tokens[2];
                    let stop_str = &tokens[3];
                    
                    match (start_str.parse::<i64>(), stop_str.parse::<i64>()) {
                        (Ok(start), Ok(stop)) => {
                            let mut db = self.db.lock();
                            match db.lrange(item, start, stop) {
                                Ok(values) => {
                                    let resp_values: Vec<RespValue> = values.into_iter()
                                        .map(|s| RespValue::BulkString(Some(s)))
                                        .collect();
                                    return RespValue::Array(Some(resp_values));
                                },
                                Err(e) => return RespValue::Error(e),
                            }
                        },
                        _ => return RespValue::Error("value is not an integer or out of range".to_string()),
                    }
                } else if cmd_upper == "EXPIRE" {
                    if let Some(seconds_str) = tokens.get(2).cloned() {
                        if let Ok(seconds) = seconds_str.parse::<u64>() {
                            let mut db = self.db.lock();
                            let result = db.expire(item, seconds);
                            
                            if result {
                                // Persist to AOF
                                let mut aof = self.aof.lock();
                                if let Err(e) = aof.append(tokens.clone()) {
                                    error!("Failed to append to AOF: {}", e);
                                }
                            }
                            
                            return RespValue::Integer(if result { 1 } else { 0 });
                        } else {
                            return RespValue::Error("value is not an integer or out of range".to_string());
                        }
                    } else {
                        return RespValue::Error("wrong number of arguments for 'EXPIRE' command".to_string());
                    }
                } else if cmd_upper == "TTL" {
                    let mut db = self.db.lock();
                    let ttl = db.ttl(item);
                    return RespValue::Integer(ttl);
                } else if cmd_upper == "PERSIST" {
                    let mut db = self.db.lock();
                    let result = db.persist(item);
                    
                    if result {
                        // Persist to AOF
                        let mut aof = self.aof.lock();
                        if let Err(e) = aof.append(tokens.clone()) {
                            error!("Failed to append to AOF: {}", e);
                        }
                    }
                    
                    return RespValue::Integer(if result { 1 } else { 0 });
                }
            }
        }

        warn!("Unknown command: {:?}", tokens);
        RespValue::Error("unknown command".to_string())
    }
}

