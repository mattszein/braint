use thiserror::Error;

// NOTE(debt-3): This error enum is intentionally minimal for Phase 1.
// Phase 2 will add structured variants (Parse, Config, Timeout, NotFound)
// and map each to a specific JSON-RPC error code range.
#[derive(Error, Debug)]
pub enum DaemonError {
    #[error("storage error: {0}")]
    Storage(#[from] rusqlite::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON-RPC error: {0}")]
    JsonRpc(String),
}

pub type Result<T> = std::result::Result<T, DaemonError>;
