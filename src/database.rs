use std::collections::HashMap;

pub struct DB{
    items: HashMap<String,String>
}

impl DB {
    pub fn new() -> Self {
        DB {
            items: HashMap::new(),
        }
    }

    pub fn get(&self, item: String) -> Option<String> {
        self.items.get(&item).cloned()
    }

    pub fn set(&mut self,item: String,value: String) {
        self.items.insert(item,value);
    }

    pub fn del(&mut self,item: String) {
        self.items.remove(&item);
    }

    pub fn exists(&self, item: String) -> bool {
        self.items.contains_key(&item)
    }

    pub fn keys(&self, pattern: String) -> Vec<String> {
        if pattern == "*" {
            // Return all keys
            self.items.keys().cloned().collect()
        } else if pattern.contains('*') {
            // Simple wildcard matching
            let prefix = pattern.trim_end_matches('*');
            self.items.keys()
                .filter(|k| k.starts_with(prefix))
                .cloned()
                .collect()
        } else {
            // Exact match
            if self.items.contains_key(&pattern) {
                vec![pattern]
            } else {
                vec![]
            }
        }
    }

    pub fn incr(&mut self, key: String) -> Result<i64, String> {
        let current = self.items.get(&key)
            .map(|v| v.as_str())
            .unwrap_or("0");
        
        match current.parse::<i64>() {
            Ok(num) => {
                let new_val = num + 1;
                self.items.insert(key, new_val.to_string());
                Ok(new_val)
            }
            Err(_) => Err(String::from("value is not an integer or out of range"))
        }
    }

    pub fn decr(&mut self, key: String) -> Result<i64, String> {
        let current = self.items.get(&key)
            .map(|v| v.as_str())
            .unwrap_or("0");
        
        match current.parse::<i64>() {
            Ok(num) => {
                let new_val = num - 1;
                self.items.insert(key, new_val.to_string());
                Ok(new_val)
            }
            Err(_) => Err(String::from("value is not an integer or out of range"))
        }
    }
}