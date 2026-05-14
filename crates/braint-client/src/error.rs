use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("daemon unreachable: {0}")]
    DaemonUnreachable(String),
}

pub type Result<T> = std::result::Result<T, ClientError>;
