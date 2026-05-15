use thiserror::Error;

#[derive(Error, Debug)]
pub enum DaemonError {
    #[error("storage error: {0}")]
    Storage(#[from] rusqlite::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON-RPC error: {0}")]
    JsonRpc(String),
    #[error("config error: {0}")]
    Config(String),
}

pub type Result<T> = std::result::Result<T, DaemonError>;
