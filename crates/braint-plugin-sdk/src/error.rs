//! Error types for the plugin SDK.

use thiserror::Error;

/// Errors that can occur in the plugin SDK transport or dispatch loop.
#[derive(Debug, Error)]
pub enum PluginSdkError {
    /// An underlying I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// A JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    /// A handler returned an error string.
    #[error("handler error: {0}")]
    Handler(String),
}

/// Convenience result alias for the plugin SDK.
pub type Result<T> = std::result::Result<T, PluginSdkError>;
