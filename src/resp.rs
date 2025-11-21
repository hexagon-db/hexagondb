

#[derive(Debug, Clone, PartialEq)]
pub enum RespValue {
    SimpleString(String),
    Error(String),
    Integer(i64),
    BulkString(Option<String>),
    Array(Option<Vec<RespValue>>),
}

impl RespValue {
    pub fn serialize(&self) -> String {
        match self {
            RespValue::SimpleString(s) => format!("+{}\r\n", s),
            RespValue::Error(msg) => format!("-{}\r\n", msg),
            RespValue::Integer(i) => format!(":{}\r\n", i),
            RespValue::BulkString(val) => match val {
                Some(s) => format!("${}\r\n{}\r\n", s.len(), s),
                None => "$-1\r\n".to_string(),
            },
            RespValue::Array(val) => match val {
                Some(arr) => {
                    let mut res = format!("*{}\r\n", arr.len());
                    for v in arr {
                        res.push_str(&v.serialize());
                    }
                    res
                }
                None => "*-1\r\n".to_string(),
            },
        }
    }
}

pub struct RespHandler {
    // We might need internal buffer state later for partial reads
}

impl RespHandler {
    pub fn new() -> Self {
        RespHandler {}
    }

    // Helper to read a line ending with CRLF
    fn read_line(buffer: &[u8]) -> Option<(String, usize)> {
        let mut i = 0;
        while i < buffer.len() - 1 {
            if buffer[i] == b'\r' && buffer[i + 1] == b'\n' {
                let line = String::from_utf8_lossy(&buffer[0..i]).to_string();
                return Some((line, i + 2));
            }
            i += 1;
        }
        None
    }

    // Helper to parse an integer from a line
    fn parse_int(buffer: &[u8]) -> Option<(i64, usize)> {
        if let Some((line, len)) = Self::read_line(buffer) {
            if let Ok(val) = line.parse::<i64>() {
                return Some((val, len));
            }
        }
        None
    }

    pub fn parse_request(buffer: &[u8]) -> Result<Option<(RespValue, usize)>, String> {
        if buffer.is_empty() {
            return Ok(None);
        }

        match buffer[0] {
            b'+' => {
                if let Some((line, len)) = Self::read_line(&buffer[1..]) {
                    Ok(Some((RespValue::SimpleString(line), len + 1)))
                } else {
                    Ok(None) // Incomplete
                }
            }
            b'-' => {
                if let Some((line, len)) = Self::read_line(&buffer[1..]) {
                    Ok(Some((RespValue::Error(line), len + 1)))
                } else {
                    Ok(None)
                }
            }
            b':' => {
                if let Some((val, len)) = Self::parse_int(&buffer[1..]) {
                    Ok(Some((RespValue::Integer(val), len + 1)))
                } else {
                    Ok(None)
                }
            }
            b'$' => {
                if let Some((len_val, len_bytes)) = Self::parse_int(&buffer[1..]) {
                    let start = 1 + len_bytes;
                    if len_val == -1 {
                        return Ok(Some((RespValue::BulkString(None), start)));
                    }
                    let str_len = len_val as usize;
                    if buffer.len() >= start + str_len + 2 {
                        let str_val = String::from_utf8_lossy(&buffer[start..start + str_len]).to_string();
                        Ok(Some((RespValue::BulkString(Some(str_val)), start + str_len + 2)))
                    } else {
                        Ok(None) // Incomplete
                    }
                } else {
                    Ok(None)
                }
            }
            b'*' => {
                if let Some((count, len_bytes)) = Self::parse_int(&buffer[1..]) {
                    let mut current_pos = 1 + len_bytes;
                    if count == -1 {
                        return Ok(Some((RespValue::Array(None), current_pos)));
                    }
                    
                    let mut items = Vec::new();
                    for _ in 0..count {
                        if let Ok(Some((item, len))) = Self::parse_request(&buffer[current_pos..]) {
                            items.push(item);
                            current_pos += len;
                        } else {
                            return Ok(None); // Incomplete
                        }
                    }
                    Ok(Some((RespValue::Array(Some(items)), current_pos)))
                } else {
                    Ok(None)
                }
            }
            _ => {
                // Inline command (simple space-separated like "GET key")
                // This is for backward compatibility and simple telnet usage
                if let Some((line, len)) = Self::read_line(buffer) {
                    let parts: Vec<String> = line.split_whitespace().map(|s| s.to_string()).collect();
                    let args: Vec<RespValue> = parts.into_iter()
                        .map(|s| RespValue::BulkString(Some(s)))
                        .collect();
                    Ok(Some((RespValue::Array(Some(args)), len)))
                } else {
                    Ok(None)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_simple_string() {
        let val = RespValue::SimpleString("OK".to_string());
        assert_eq!(val.serialize(), "+OK\r\n");
    }

    #[test]
    fn test_serialize_error() {
        let val = RespValue::Error("Error message".to_string());
        assert_eq!(val.serialize(), "-Error message\r\n");
    }

    #[test]
    fn test_serialize_integer() {
        let val = RespValue::Integer(1000);
        assert_eq!(val.serialize(), ":1000\r\n");
    }

    #[test]
    fn test_serialize_bulk_string() {
        let val = RespValue::BulkString(Some("hello".to_string()));
        assert_eq!(val.serialize(), "$5\r\nhello\r\n");
        
        let null_val = RespValue::BulkString(None);
        assert_eq!(null_val.serialize(), "$-1\r\n");
    }

    #[test]
    fn test_serialize_array() {
        let val = RespValue::Array(Some(vec![
            RespValue::BulkString(Some("hello".to_string())),
            RespValue::BulkString(Some("world".to_string()))
        ]));
        assert_eq!(val.serialize(), "*2\r\n$5\r\nhello\r\n$5\r\nworld\r\n");
        
        let null_arr = RespValue::Array(None);
        assert_eq!(null_arr.serialize(), "*-1\r\n");
    }

    #[test]
    fn test_parse_array() {
        let data = b"*2\r\n$5\r\nhello\r\n$5\r\nworld\r\n";
        let (val, len) = RespHandler::parse_request(data).unwrap().unwrap();
        
        assert_eq!(len, data.len());
        match val {
            RespValue::Array(Some(items)) => {
                assert_eq!(items.len(), 2);
                assert_eq!(items[0], RespValue::BulkString(Some("hello".to_string())));
                assert_eq!(items[1], RespValue::BulkString(Some("world".to_string())));
            },
            _ => panic!("Expected Array"),
        }
    }
    
    #[test]
    fn test_parse_inline() {
        let data = b"SET key value\r\n";
        let (val, len) = RespHandler::parse_request(data).unwrap().unwrap();
        
        assert_eq!(len, data.len());
        match val {
            RespValue::Array(Some(items)) => {
                assert_eq!(items.len(), 3);
                assert_eq!(items[0], RespValue::BulkString(Some("SET".to_string())));
                assert_eq!(items[1], RespValue::BulkString(Some("key".to_string())));
                assert_eq!(items[2], RespValue::BulkString(Some("value".to_string())));
            },
            _ => panic!("Expected Array"),
        }
    }
}
