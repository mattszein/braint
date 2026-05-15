use thiserror::Error;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("parse error: {0}")]
    Parse(String),
    #[error("unknown verb: {0}")]
    Verb(String),
    #[error("malformed tag: {0}")]
    MalformedTag(String),
}

pub type Result<T> = std::result::Result<T, CoreError>;
