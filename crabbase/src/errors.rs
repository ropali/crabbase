use std::fmt;

#[derive(Debug)]
pub enum AppError {
    Database(String),
    Config(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Database(msg) => write!(f, "Database error: {}", msg),
            AppError::Config(msg) => write!(f, "Config error: {}", msg),
        }
    }
}

impl std::error::Error for AppError {}
