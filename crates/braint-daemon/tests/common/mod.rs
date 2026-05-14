use braint_core::Clock;
use braint_daemon::{handler::IngestHandler, server, storage::Storage};
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

    let storage = Storage::open(&db_path).unwrap();
    let device_id = DeviceId::generate();
    let clock = Clock::new(device_id);
    let handler = IngestHandler::new(storage, clock, device_id);

    let socket_str = socket_path.to_string_lossy().to_string();
    let name = socket_str.to_fs_name::<GenericFilePath>().unwrap();
    let listener = ListenerOptions::new().name(name).create_tokio().unwrap();

    let task = tokio::spawn(async move {
        let _ = server::run(listener, handler).await;
    });

    // Give the server a moment to start listening
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    TestDaemonHandle {
        socket_path,
        db_path,
        _task: task,
        _tempdir: tempdir,
    }
}
