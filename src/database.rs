use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Clone)]
struct Entry {
    value: String,
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

    pub fn get(&mut self, item: String) -> Option<String> {
        if let Some(entry) = self.items.get(&item) {
            if let Some(expires_at) = entry.expires_at {
                if Instant::now() > expires_at {
                    self.items.remove(&item);
                    return None;
                }
            }
            return Some(entry.value.clone());
        }
        None
    }

    pub fn set(&mut self, item: String, value: String) {
        self.items.insert(item, Entry {
            value,
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
        // Check expiration first (lazy expire)
        if !self.exists(key.clone()) {
            self.items.remove(&key);
        }

        let current_entry = self.items.get(&key);
        let current_val = match current_entry {
            Some(entry) => entry.value.as_str(),
            None => "0",
        };
        
        match current_val.parse::<i64>() {
            Ok(num) => {
                let new_val = num + 1;
                // Preserve expiration if exists
                let expires_at = current_entry.and_then(|e| e.expires_at);
                self.items.insert(key, Entry {
                    value: new_val.to_string(),
                    expires_at,
                });
                Ok(new_val)
            }
            Err(_) => Err(String::from("value is not an integer or out of range"))
        }
    }

    pub fn decr(&mut self, key: String) -> Result<i64, String> {
        // Check expiration first
        if !self.exists(key.clone()) {
            self.items.remove(&key);
        }

        let current_entry = self.items.get(&key);
        let current_val = match current_entry {
            Some(entry) => entry.value.as_str(),
            None => "0",
        };
        
        match current_val.parse::<i64>() {
            Ok(num) => {
                let new_val = num - 1;
                let expires_at = current_entry.and_then(|e| e.expires_at);
                self.items.insert(key, Entry {
                    value: new_val.to_string(),
                    expires_at,
                });
                Ok(new_val)
            }
            Err(_) => Err(String::from("value is not an integer or out of range"))
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