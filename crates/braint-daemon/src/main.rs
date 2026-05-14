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

    // NOTE(debt-2): unwrap_or_else is acceptable for Phase 1 but should be
    // a proper error variant in Phase 2 when config paths are validated.
    let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let socket_path = runtime_dir.join("braint.sock");
    let db_path = runtime_dir.join("braint.db");

    // Cleanup stale socket. NOTE(debt-4): Add SIGTERM/SIGINT handler in Phase 2
    // so the socket is removed on graceful exit, not just startup.
    let _ = std::fs::remove_file(&socket_path);

    let storage = Storage::open(&db_path)?;
    let device_id = DeviceId::generate(); // Phase 1: ephemeral; Phase 2: persisted
    let clock = Clock::new(device_id);
    let handler = IngestHandler::new(storage, clock, device_id);

    let socket_str = socket_path.to_string_lossy().to_string();
    let name = socket_str.to_fs_name::<GenericFilePath>()?;
    let listener = ListenerOptions::new().name(name).create_tokio()?;
    tracing::info!("daemon listening on {:?}", socket_path);

    // NOTE(debt-5): server::run is sequential (one connection at a time).
    // Phase 2 TUI + CLI simultaneous use requires per-connection tasks.
    server::run(listener, handler).await
}
