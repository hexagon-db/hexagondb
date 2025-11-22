use std::fmt;

/// HexagonDB error type
#[derive(Debug, Clone)]
pub struct HexagonDBError {
    message: String,
}

impl HexagonDBError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for HexagonDBError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for HexagonDBError {}

impl From<std::io::Error> for HexagonDBError {
    fn from(err: std::io::Error) -> Self {
        HexagonDBError::new(err.to_string())
    }
}
