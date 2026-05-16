use braint_client::Client;
use braint_core::Clock;
use braint_daemon::{
    config::DaemonConfig,
    plugin::PluginManager,
    server::{self, state::DaemonState},
    storage::Storage,
};
use braint_proto::DeviceId;
use interprocess::local_socket::tokio::prelude::*;
use interprocess::local_socket::{GenericFilePath, ListenerOptions};
use std::path::PathBuf;
use tokio::task::JoinHandle;

pub struct TestDaemonHandle {
    pub socket_path: PathBuf,
    pub db_path: PathBuf,
    pub client: Client,
    pub _task: JoinHandle<()>,
    pub _tempdir: tempfile::TempDir,
}

pub async fn spawn_test_daemon() -> TestDaemonHandle {
    spawn_test_daemon_with_plugins(vec![]).await
}

/// Spawn a test daemon that loads plugins from the given directories.
pub async fn spawn_test_daemon_with_plugins(plugin_dirs: Vec<PathBuf>) -> TestDaemonHandle {
    // Use short prefix to stay under UDS path limits (108 bytes on Linux)
    let tempdir = tempfile::Builder::new().prefix("b").tempdir().unwrap();
    let socket_path = tempdir.path().join("s.sock");
    let db_path = tempdir.path().join("test.db");

    let storage = Storage::open(&db_path).unwrap();
    let device_id = DeviceId::generate();
    let clock = Clock::new(device_id);

    let plugins = PluginManager::load(&plugin_dirs).await.unwrap_or_else(|e| {
        eprintln!("warn: plugin load error: {e}");
        PluginManager::empty()
    });

    let config = DaemonConfig {
        socket_path: socket_path.clone(),
        db_path: db_path.clone(),
        data_dir: tempdir.path().to_path_buf(),
        device_id_path: tempdir.path().join("device_id"),
        pending_ttl_secs: 60,
        max_subs_per_conn: 32,
        plugin_dirs,
    };

    let state = DaemonState::new(storage, clock, device_id, config, plugins);

    let socket_str = socket_path.to_string_lossy().to_string();
    let name = socket_str.to_fs_name::<GenericFilePath>().unwrap();
    let listener = ListenerOptions::new().name(name).create_tokio().unwrap();

    let task = tokio::spawn(async move {
        let _ = server::run(listener, state).await;
    });

    // Give the server a moment to start listening.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let client = Client::connect(socket_path.to_str().unwrap())
        .await
        .unwrap();

    TestDaemonHandle {
        socket_path,
        db_path,
        client,
        _task: task,
        _tempdir: tempdir,
    }
}

/// Open the SQLite database at `db_path` and count how many rows in the
/// `entries` table have the given `entry_id` (stored as raw UUID bytes).
pub fn query_count(db_path: &PathBuf, entry_id: braint_proto::EntryId) -> i64 {
    let conn = rusqlite::Connection::open(db_path).unwrap();
    conn.query_row(
        "SELECT COUNT(*) FROM entries WHERE id = ?1",
        [entry_id.0.as_bytes()],
        |row| row.get(0),
    )
    .unwrap()
}

/// Returns the path to the `braint-plugin-hello` binary from the cargo target directory.
pub fn hello_plugin_path() -> PathBuf {
    // CARGO_MANIFEST_DIR = .../crates/braint-daemon
    // Workspace root is two levels up.
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.parent().unwrap().parent().unwrap();
    workspace_root
        .join("target")
        .join("debug")
        .join("braint-plugin-hello")
}
