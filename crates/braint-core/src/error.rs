use thiserror::Error;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("parse error: {0}")]
    Parse(String),
}

pub type Result<T> = std::result::Result<T, CoreError>;
