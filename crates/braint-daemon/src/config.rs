//! Daemon configuration — paths, defaults, persistent device ID.

use braint_proto::DeviceId;
use directories::ProjectDirs;
use std::path::PathBuf;

/// All runtime-configuration for the daemon process.
///
/// Constructed from environment variables and XDG base directories with sensible defaults.
#[derive(Debug, Clone)]
pub struct DaemonConfig {
    /// Path to the Unix domain socket the daemon listens on.
    pub socket_path: PathBuf,
    /// Path to the SQLite database file.
    pub db_path: PathBuf,
    /// Base data directory (XDG data home / braint).
    pub data_dir: PathBuf,
    /// Path where the persistent device UUID is stored.
    pub device_id_path: PathBuf,
    /// How long a pending voice confirmation lives before it expires (seconds).
    pub pending_ttl_secs: u64,
    /// Maximum number of active subscriptions allowed per connection.
    pub max_subs_per_conn: usize,
    /// Directories to scan for plugin executables. Defaults to empty (no plugins).
    pub plugin_dirs: Vec<PathBuf>,
}

impl DaemonConfig {
    /// Build config from environment / XDG dirs with sensible defaults.
    pub fn from_env() -> Self {
        let dirs = ProjectDirs::from("", "", "braint");

        let data_dir = dirs
            .as_ref()
            .map(|d| d.data_dir().to_path_buf())
            .unwrap_or_else(|| std::env::temp_dir().join("braint"));

        let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);

        Self {
            socket_path: runtime_dir.join("braint.sock"),
            db_path: data_dir.join("braint.db"),
            device_id_path: data_dir.join("device_id"),
            data_dir,
            pending_ttl_secs: 60,
            max_subs_per_conn: 32,
            plugin_dirs: Vec::new(),
        }
    }
}

/// Load or generate a persistent [`DeviceId`].
///
/// Reads the UUID from `path`; generates and writes a fresh UUIDv7 if the file is absent.
pub fn load_or_create_device_id(path: &std::path::Path) -> crate::Result<DeviceId> {
    if path.exists() {
        let s = std::fs::read_to_string(path).map_err(crate::DaemonError::Io)?;
        let uuid = uuid::Uuid::parse_str(s.trim())
            .map_err(|e| crate::DaemonError::Config(format!("invalid device id: {e}")))?;
        Ok(DeviceId(uuid))
    } else {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(crate::DaemonError::Io)?;
        }
        let id = DeviceId::generate();
        std::fs::write(path, id.0.to_string()).map_err(crate::DaemonError::Io)?;
        Ok(id)
    }
}
