use std::fmt;
use std::io;

#[derive(Debug)]
pub enum HexagonError {
    /// Database-related errors
    KeyNotFound(String),
    InvalidValue(String),
    
    /// Command errors
    UnknownCommand(String),
    WrongNumberOfArguments { command: String, expected: usize, got: usize },
    InvalidArgument(String),
    
    /// I/O errors
    IoError(io::Error),
    
    /// Connection errors
    ConnectionClosed,
}

impl fmt::Display for HexagonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HexagonError::KeyNotFound(key) => write!(f, "Key not found: {}", key),
            HexagonError::InvalidValue(msg) => write!(f, "Invalid value: {}", msg),
            HexagonError::UnknownCommand(cmd) => write!(f, "Unknown command: {}", cmd),
            HexagonError::WrongNumberOfArguments { command, expected, got } => {
                write!(f, "Wrong number of arguments for '{}' command (expected {}, got {})", 
                       command, expected, got)
            }
            HexagonError::InvalidArgument(msg) => write!(f, "Invalid argument: {}", msg),
            HexagonError::IoError(err) => write!(f, "I/O error: {}", err),
            HexagonError::ConnectionClosed => write!(f, "Connection closed"),
        }
    }
}

impl std::error::Error for HexagonError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            HexagonError::IoError(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for HexagonError {
    fn from(err: io::Error) -> Self {
        HexagonError::IoError(err)
    }
}

pub type Result<T> = std::result::Result<T, HexagonError>;
