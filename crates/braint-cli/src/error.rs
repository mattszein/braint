use thiserror::Error;

#[derive(Error, Debug)]
pub enum CliError {
    #[error("daemon error: {0}")]
    Daemon(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, CliError>;
