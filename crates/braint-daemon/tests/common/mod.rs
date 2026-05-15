use braint_core::Clock;
use braint_daemon::{
    config::DaemonConfig,
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
    pub _task: JoinHandle<()>,
    pub _tempdir: tempfile::TempDir,
}

pub async fn spawn_test_daemon() -> TestDaemonHandle {
    // Use short prefix to stay under UDS path limits (108 bytes on Linux)
    let tempdir = tempfile::Builder::new().prefix("b").tempdir().unwrap();
    let socket_path = tempdir.path().join("s.sock");
    let db_path = tempdir.path().join("test.db");

    // Build a minimal config pointing at the temp paths.
    let config = DaemonConfig {
        socket_path: socket_path.clone(),
        db_path: db_path.clone(),
        data_dir: tempdir.path().to_path_buf(),
        device_id_path: tempdir.path().join("device_id"),
        pending_ttl_secs: 60,
        max_subs_per_conn: 32,
    };

    spawn_test_daemon_with_config(config, tempdir).await
}

/// Spawn a test daemon using an explicit config. The caller must supply a
/// `TempDir` whose lifetime will be tied to the returned handle.
pub async fn spawn_test_daemon_with_config(
    config: DaemonConfig,
    tempdir: tempfile::TempDir,
) -> TestDaemonHandle {
    let socket_path = config.socket_path.clone();
    let db_path = config.db_path.clone();

    let storage = Storage::open(&db_path).unwrap();
    let device_id = DeviceId::generate();
    let clock = Clock::new(device_id);

    let state = DaemonState::new(storage, clock, device_id, config);

    let socket_str = socket_path.to_string_lossy().to_string();
    let name = socket_str.to_fs_name::<GenericFilePath>().unwrap();
    let listener = ListenerOptions::new().name(name).create_tokio().unwrap();

    let task = tokio::spawn(async move {
        let _ = server::run(listener, state).await;
    });

    // Give the server a moment to start listening.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    TestDaemonHandle {
        socket_path,
        db_path,
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
