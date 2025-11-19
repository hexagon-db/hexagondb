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

    pub fn get(&self,item: &str) -> String{
        if let Some(value) = self.items.get(&item.to_string()){
            return value.to_string();
        } else {
            return String::from("");
        }
    }

    pub fn set(&mut self,item: &str,value: &str) {
        self.items.insert(item.to_string(),value.to_string());
    }

    pub fn del(&mut self,item: &str) {
        self.items.insert(item.to_string(),"".to_string());
    }
}