use thiserror::Error;

// NOTE(debt-3): Minimal for Phase 1. Phase 2 will distinguish daemon unreachable
// from daemon returned an error, and surface structured error codes.
#[derive(Error, Debug)]
pub enum CliError {
    #[error("daemon error: {0}")]
    Daemon(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, CliError>;
