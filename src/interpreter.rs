use std::sync::Arc;
use parking_lot::Mutex;
use tracing::{debug, warn};
use crate::{database::DB, parse_query, resp::RespValue};

pub struct Interpreter {
    db: Arc<Mutex<DB>>
}
impl Interpreter {
    pub fn new(db: Arc<Mutex<DB>>) -> Self {
        Interpreter {
            db,
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
            // Handle KEYS command
            if cmd.to_uppercase() == "KEYS" {
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
                if cmd.to_uppercase() == "GET" {
                    let mut db = self.db.lock();
                    return match db.get(item) {
                        Some(value) => RespValue::BulkString(Some(value)),
                        None => RespValue::BulkString(None),
                    };
                } else if cmd.to_uppercase() == "SET" {
                    if let Some(value) = tokens.get(2).cloned() {
                        let mut db = self.db.lock();
                        db.set(item, value);
                        return RespValue::SimpleString("OK".to_string());
                    } else {
                        return RespValue::Error("wrong number of arguments for 'SET' command".to_string());
                    }
                } else if cmd.to_uppercase() == "DEL" {
                    let mut db = self.db.lock();
                    db.del(item);
                    return RespValue::Integer(1);
                } else if cmd.to_uppercase() == "EXISTS" {
                    let db = self.db.lock();
                    let exists = db.exists(item);
                    return RespValue::Integer(if exists { 1 } else { 0 });
                } else if cmd.to_uppercase() == "INCR" {
                    let mut db = self.db.lock();
                    match db.incr(item) {
                        Ok(val) => return RespValue::Integer(val),
                        Err(e) => return RespValue::Error(e),
                    }
                } else if cmd.to_uppercase() == "DECR" {
                    let mut db = self.db.lock();
                    match db.decr(item) {
                        Ok(val) => return RespValue::Integer(val),
                        Err(e) => return RespValue::Error(e),
                    }
                } else if cmd.to_uppercase() == "EXPIRE" {
                    if let Some(seconds_str) = tokens.get(2).cloned() {
                        if let Ok(seconds) = seconds_str.parse::<u64>() {
                            let mut db = self.db.lock();
                            let result = db.expire(item, seconds);
                            return RespValue::Integer(if result { 1 } else { 0 });
                        } else {
                            return RespValue::Error("value is not an integer or out of range".to_string());
                        }
                    } else {
                        return RespValue::Error("wrong number of arguments for 'EXPIRE' command".to_string());
                    }
                } else if cmd.to_uppercase() == "TTL" {
                    let mut db = self.db.lock();
                    let ttl = db.ttl(item);
                    return RespValue::Integer(ttl);
                } else if cmd.to_uppercase() == "PERSIST" {
                    let mut db = self.db.lock();
                    let result = db.persist(item);
                    return RespValue::Integer(if result { 1 } else { 0 });
                }
            }
        }

        warn!("Unknown command: {:?}", tokens);
        RespValue::Error("unknown command".to_string())
    }
}

