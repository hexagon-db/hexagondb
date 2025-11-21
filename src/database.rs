use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub enum DataType {
    String(String),
    List(Vec<String>),
}

#[derive(Clone)]
struct Entry {
    value: DataType,
    expires_at: Option<Instant>,
}

pub struct DB{
    items: HashMap<String, Entry>
}

impl DB {
    pub fn new() -> Self {
        DB {
            items: HashMap::new(),
        }
    }

    fn check_expiration(&mut self, key: &str) -> bool {
        if let Some(entry) = self.items.get(key) {
            if let Some(expires_at) = entry.expires_at {
                if Instant::now() > expires_at {
                    self.items.remove(key);
                    return false;
                }
            }
            return true;
        }
        false
    }

    pub fn get(&mut self, item: String) -> Result<Option<String>, String> {
        if !self.check_expiration(&item) {
            return Ok(None);
        }

        if let Some(entry) = self.items.get(&item) {
            match &entry.value {
                DataType::String(s) => Ok(Some(s.clone())),
                _ => Err("WRONGTYPE Operation against a key holding the wrong kind of value".to_string()),
            }
        } else {
            Ok(None)
        }
    }

    pub fn set(&mut self, item: String, value: String) {
        self.items.insert(item, Entry {
            value: DataType::String(value),
            expires_at: None,
        });
    }

    pub fn del(&mut self, item: String) {
        self.items.remove(&item);
    }

    pub fn exists(&self, item: String) -> bool {
        if let Some(entry) = self.items.get(&item) {
            if let Some(expires_at) = entry.expires_at {
                if Instant::now() > expires_at {
                    return false;
                }
            }
            return true;
        }
        false
    }

    pub fn keys(&self, pattern: String) -> Vec<String> {
        let now = Instant::now();
        let valid_keys = self.items.iter()
            .filter(|(_, entry)| {
                if let Some(expires_at) = entry.expires_at {
                    now <= expires_at
                } else {
                    true
                }
            });

        if pattern == "*" {
            valid_keys.map(|(k, _)| k.clone()).collect()
        } else if pattern.contains('*') {
            let prefix = pattern.trim_end_matches('*');
            valid_keys
                .filter(|(k, _)| k.starts_with(prefix))
                .map(|(k, _)| k.clone())
                .collect()
        } else {
            if self.exists(pattern.clone()) {
                vec![pattern]
            } else {
                vec![]
            }
        }
    }

    pub fn incr(&mut self, key: String) -> Result<i64, String> {
        if !self.check_expiration(&key) {
            // Key expired or didn't exist
        }

        let current_val = if let Some(entry) = self.items.get(&key) {
            match &entry.value {
                DataType::String(s) => s.clone(),
                _ => return Err("WRONGTYPE Operation against a key holding the wrong kind of value".to_string()),
            }
        } else {
            "0".to_string()
        };
        
        match current_val.parse::<i64>() {
            Ok(num) => {
                let new_val = num + 1;
                let expires_at = self.items.get(&key).and_then(|e| e.expires_at);
                self.items.insert(key, Entry {
                    value: DataType::String(new_val.to_string()),
                    expires_at,
                });
                Ok(new_val)
            }
            Err(_) => Err(String::from("value is not an integer or out of range"))
        }
    }

    pub fn decr(&mut self, key: String) -> Result<i64, String> {
        if !self.check_expiration(&key) {
            // Key expired or didn't exist
        }

        let current_val = if let Some(entry) = self.items.get(&key) {
            match &entry.value {
                DataType::String(s) => s.clone(),
                _ => return Err("WRONGTYPE Operation against a key holding the wrong kind of value".to_string()),
            }
        } else {
            "0".to_string()
        };
        
        match current_val.parse::<i64>() {
            Ok(num) => {
                let new_val = num - 1;
                let expires_at = self.items.get(&key).and_then(|e| e.expires_at);
                self.items.insert(key, Entry {
                    value: DataType::String(new_val.to_string()),
                    expires_at,
                });
                Ok(new_val)
            }
            Err(_) => Err(String::from("value is not an integer or out of range"))
        }
    }

    // List Operations
    pub fn lpush(&mut self, key: String, values: Vec<String>) -> usize {
        self.check_expiration(&key);
        
        let entry = self.items.entry(key).or_insert(Entry {
            value: DataType::List(Vec::new()),
            expires_at: None,
        });

        match &mut entry.value {
            DataType::List(list) => {
                for v in values {
                    list.insert(0, v);
                }
                list.len()
            },
            _ => 0, // Should return error but for simplicity returning 0 (or panic?) Redis returns error.
            // We should change signature to Result
        }
    }
    
    // Helper for proper error handling in List ops
    pub fn lpush_safe(&mut self, key: String, values: Vec<String>) -> Result<usize, String> {
        self.check_expiration(&key);
        
        // We can't use entry() because we need to check type first and return error
        if let Some(entry) = self.items.get_mut(&key) {
            match &mut entry.value {
                DataType::List(list) => {
                    for v in values {
                        list.insert(0, v);
                    }
                    return Ok(list.len());
                },
                _ => return Err("WRONGTYPE Operation against a key holding the wrong kind of value".to_string()),
            }
        }
        
        // Create new list
        let mut list = Vec::new();
        for v in values {
            list.insert(0, v);
        }
        let len = list.len();
        self.items.insert(key, Entry {
            value: DataType::List(list),
            expires_at: None,
        });
        Ok(len)
    }

    pub fn rpush(&mut self, key: String, values: Vec<String>) -> Result<usize, String> {
        self.check_expiration(&key);
        
        if let Some(entry) = self.items.get_mut(&key) {
            match &mut entry.value {
                DataType::List(list) => {
                    list.extend(values);
                    return Ok(list.len());
                },
                _ => return Err("WRONGTYPE Operation against a key holding the wrong kind of value".to_string()),
            }
        }
        
        self.items.insert(key, Entry {
            value: DataType::List(values.clone()),
            expires_at: None,
        });
        Ok(values.len())
    }

    pub fn lpop(&mut self, key: String) -> Result<Option<String>, String> {
        self.check_expiration(&key);
        
        if let Some(entry) = self.items.get_mut(&key) {
            match &mut entry.value {
                DataType::List(list) => {
                    let val = list.first().cloned();
                    if val.is_some() {
                        list.remove(0);
                    }
                    // If empty, should we remove key? Redis does.
                    if list.is_empty() {
                        self.items.remove(&key);
                    }
                    Ok(val)
                },
                _ => Err("WRONGTYPE Operation against a key holding the wrong kind of value".to_string()),
            }
        } else {
            Ok(None)
        }
    }

    pub fn rpop(&mut self, key: String) -> Result<Option<String>, String> {
        self.check_expiration(&key);
        
        if let Some(entry) = self.items.get_mut(&key) {
            match &mut entry.value {
                DataType::List(list) => {
                    let val = list.pop();
                    if list.is_empty() {
                        self.items.remove(&key);
                    }
                    Ok(val)
                },
                _ => Err("WRONGTYPE Operation against a key holding the wrong kind of value".to_string()),
            }
        } else {
            Ok(None)
        }
    }

    pub fn llen(&mut self, key: String) -> Result<usize, String> {
        self.check_expiration(&key);
        
        if let Some(entry) = self.items.get(&key) {
            match &entry.value {
                DataType::List(list) => Ok(list.len()),
                _ => Err("WRONGTYPE Operation against a key holding the wrong kind of value".to_string()),
            }
        } else {
            Ok(0)
        }
    }

    pub fn lrange(&mut self, key: String, start: i64, stop: i64) -> Result<Vec<String>, String> {
        self.check_expiration(&key);
        
        if let Some(entry) = self.items.get(&key) {
            match &entry.value {
                DataType::List(list) => {
                    let len = list.len() as i64;
                    if len == 0 {
                        return Ok(Vec::new());
                    }

                    let mut start_idx = if start < 0 { len + start } else { start };
                    let mut stop_idx = if stop < 0 { len + stop } else { stop };

                    if start_idx < 0 { start_idx = 0; }
                    if stop_idx < 0 { stop_idx = 0; }
                    if start_idx >= len { return Ok(Vec::new()); }
                    if stop_idx >= len { stop_idx = len - 1; }
                    if start_idx > stop_idx { return Ok(Vec::new()); }

                    Ok(list[start_idx as usize..=stop_idx as usize].to_vec())
                },
                _ => Err("WRONGTYPE Operation against a key holding the wrong kind of value".to_string()),
            }
        } else {
            Ok(Vec::new())
        }
    }

    pub fn expire(&mut self, key: String, seconds: u64) -> bool {
        if let Some(entry) = self.items.get_mut(&key) {
            entry.expires_at = Some(Instant::now() + Duration::from_secs(seconds));
            true
        } else {
            false
        }
    }

    pub fn ttl(&mut self, key: String) -> i64 {
        if let Some(entry) = self.items.get(&key) {
            if let Some(expires_at) = entry.expires_at {
                let now = Instant::now();
                if now > expires_at {
                    self.items.remove(&key);
                    return -2; // Key does not exist (expired)
                }
                return expires_at.duration_since(now).as_secs() as i64;
            } else {
                return -1; // Key exists but has no associated expire
            }
        }
        -2 // Key does not exist
    }

    pub fn persist(&mut self, key: String) -> bool {
        if let Some(entry) = self.items.get_mut(&key) {
            if entry.expires_at.is_some() {
                entry.expires_at = None;
                return true;
            }
        }
        false
    }
}