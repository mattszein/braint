use braint_core::Clock;
use braint_daemon::{handler::IngestHandler, server, storage::Storage};
use braint_proto::DeviceId;
use interprocess::local_socket::tokio::prelude::*;
use interprocess::local_socket::{GenericFilePath, ListenerOptions};
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let socket_path = runtime_dir.join("braint.sock");
    let db_path = runtime_dir.join("braint.db");

    // Cleanup stale socket
    let _ = std::fs::remove_file(&socket_path);

    let storage = Storage::open(&db_path)?;
    let device_id = DeviceId::generate(); // Phase 1: ephemeral; Phase 2: persisted
    let clock = Clock::new(device_id);
    let handler = IngestHandler::new(storage, clock, device_id);

    let socket_str = socket_path.to_string_lossy().to_string();
    let name = socket_str.to_fs_name::<GenericFilePath>()?;
    let listener = ListenerOptions::new().name(name).create_tokio()?;
    tracing::info!("daemon listening on {:?}", socket_path);

    server::run(listener, handler).await
}
