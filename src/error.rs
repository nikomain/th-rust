use thiserror::Error;

#[derive(Error, Debug)]
pub enum ThError {
    #[error("Teleport authentication failed: {0}")]
    AuthFailed(String),
    
    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),
    
    #[error("Process execution failed: {0}")]
    Process(String),
    
    #[error("Resource not found: {0}")]
    NotFound(String),
    
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),
    
    #[error("Timeout waiting for operation: {0}")]
    Timeout(String),
    
    #[error("Shell integration error: {0}")]
    Shell(String),
    
    #[error("Teleport proxy error: {0}")]
    Proxy(String),
}