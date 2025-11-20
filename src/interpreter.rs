use crate::{database::DB, parse_query};

pub struct Interpreter {
    db: DB
}
impl Interpreter {
    pub fn new(db: DB) -> Self {
        Interpreter {
            db: db,
        }
    }

    pub fn exec(&mut self, query: String) -> String {
        let tokens: Vec<String> = parse_query::parse_query(query.to_string());

        if let Some(cmd) = tokens.get(0).cloned() {
            if let Some(item) = tokens.get(1).cloned() {
                if cmd.to_uppercase() == "GET" {
                    return self.db.get(item);
                } else if cmd.to_uppercase() == "SET" {
                    if let Some(value) = tokens.get(2).cloned() {
                        self.db.set(item, value);
                        return String::from("+OK");
                    } else {
                        return String::from("-ERR wrong number of arguments for 'SET' command");
                    }
                } else if cmd.to_uppercase() == "DEL" {
                    self.db.del(item);
                    return String::from(":1");
                }
            }
        }

        
        String::from("-ERR unknown command")
    }
}

