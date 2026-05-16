use thiserror::Error;

/// All errors that can be produced by the daemon.
#[derive(Error, Debug)]
pub enum DaemonError {
    /// A rusqlite (SQLite) error from the storage layer.
    #[error("storage error: {0}")]
    Storage(#[from] rusqlite::Error),
    /// An I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// A JSON-RPC protocol error.
    #[error("JSON-RPC error: {0}")]
    JsonRpc(String),
    /// A configuration error.
    #[error("config error: {0}")]
    Config(String),
    /// A JSON serialization / deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    /// The plugin binary returned an invalid or missing manifest.
    #[error("plugin manifest error: {0}")]
    PluginManifestError(String),
    /// A plugin governance rule was violated (e.g., duplicate verb, wrong api_version).
    #[error("plugin governance violation: {0}")]
    PluginGovernance(String),
    /// The plugin process has died and can no longer handle requests.
    #[error("plugin dead: {0}")]
    PluginDead(String),
    /// The plugin returned an application-level error.
    #[error("plugin error: {0}")]
    PluginError(String),
}

/// Convenience result alias for the daemon.
pub type Result<T> = std::result::Result<T, DaemonError>;
