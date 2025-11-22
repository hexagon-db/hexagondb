use crate::db::core::DB;
use crate::db::ops::generic::GenericOps;
use crate::db::types::{DataType, Entry};

/// String veri tipi operasyonları
pub trait StringOps {
    fn get(&mut self, item: String) -> Result<Option<String>, String>;
    fn set(&mut self, item: String, value: String);
    fn incr(&mut self, key: String) -> Result<i64, String>;
    fn decr(&mut self, key: String) -> Result<i64, String>;
}

impl StringOps for DB {
    /// String tipindeki bir değeri getirir.
    /// Eğer tip uyuşmazlığı varsa hata döner.
    fn get(&mut self, item: String) -> Result<Option<String>, String> {
        if !self.check_expiration(&item) {
            return Ok(None);
        }

        if let Some(entry) = self.items.get(&item) {
            match &entry.value {
                DataType::String(s) => Ok(Some(s.clone())),
                _ => Err(
                    "WRONGTYPE Operation against a key holding the wrong kind of value".to_string(),
                ),
            }
        } else {
            Ok(None)
        }
    }

    /// Basit bir String değeri kaydeder.
    fn set(&mut self, item: String, value: String) {
        self.items.insert(
            item,
            Entry {
                value: DataType::String(value),
                expires_at: None,
            },
        );
        self.increment_changes();
    }

    /// Sayısal değeri 1 artırır.
    /// Eğer anahtar yoksa 0 kabul edip artırır.
    fn incr(&mut self, key: String) -> Result<i64, String> {
        if !self.check_expiration(&key) {
            // Süresi dolmuşsa silindi zaten
        }

        let current_val = if let Some(entry) = self.items.get(&key) {
            match &entry.value {
                DataType::String(s) => s.clone(),
                _ => {
                    return Err(
                        "WRONGTYPE Operation against a key holding the wrong kind of value"
                            .to_string(),
                    )
                }
            }
        } else {
            "0".to_string()
        };

        match current_val.parse::<i64>() {
            Ok(num) => {
                let new_val = num + 1;
                let expires_at = self.items.get(&key).and_then(|e| e.expires_at);
                self.items.insert(
                    key,
                    Entry {
                        value: DataType::String(new_val.to_string()),
                        expires_at,
                    },
                );
                self.increment_changes();
                Ok(new_val)
            }
            Err(_) => Err(String::from("value is not an integer or out of range")),
        }
    }

    /// Sayısal değeri 1 azaltır.
    fn decr(&mut self, key: String) -> Result<i64, String> {
        if !self.check_expiration(&key) {
            // Expired
        }

        let current_val = if let Some(entry) = self.items.get(&key) {
            match &entry.value {
                DataType::String(s) => s.clone(),
                _ => {
                    return Err(
                        "WRONGTYPE Operation against a key holding the wrong kind of value"
                            .to_string(),
                    )
                }
            }
        } else {
            "0".to_string()
        };

        match current_val.parse::<i64>() {
            Ok(num) => {
                let new_val = num - 1;
                let expires_at = self.items.get(&key).and_then(|e| e.expires_at);
                self.items.insert(
                    key,
                    Entry {
                        value: DataType::String(new_val.to_string()),
                        expires_at,
                    },
                );
                self.increment_changes();
                Ok(new_val)
            }
            Err(_) => Err(String::from("value is not an integer or out of range")),
        }
    }
}
