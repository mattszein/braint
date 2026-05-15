use braint_core::Clock;
use braint_daemon::{
    config::{DaemonConfig, load_or_create_device_id},
    server::{self, state::DaemonState},
    storage::Storage,
};
use interprocess::local_socket::tokio::prelude::*;
use interprocess::local_socket::{GenericFilePath, ListenerOptions};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let config = DaemonConfig::from_env();

    std::fs::create_dir_all(&config.data_dir)?;
    // Remove stale socket from a previous run.
    let _ = std::fs::remove_file(&config.socket_path);

    let device_id = load_or_create_device_id(&config.device_id_path)?;
    let storage = Storage::open(&config.db_path)?;
    let clock = Clock::new(device_id);
    let state = DaemonState::new(storage, clock, device_id, config.clone());

    let socket_str = config.socket_path.to_string_lossy().to_string();
    let name = socket_str.to_fs_name::<GenericFilePath>()?;
    let listener = ListenerOptions::new().name(name).create_tokio()?;

    tracing::info!("daemon listening on {:?}", config.socket_path);
    server::run(listener, state).await
}
