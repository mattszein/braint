//! Socket server — accepts connections and spawns per-connection tasks.

pub mod connection;
pub mod state;

use crate::server::state::DaemonState;
use interprocess::local_socket::tokio::prelude::*;

/// Run the daemon server loop: accept connections, spawn tasks, handle Ctrl-C.
///
/// Each accepted connection is handled in its own Tokio task so the daemon
/// can serve multiple clients (CLI + TUI) concurrently.
pub async fn run(listener: LocalSocketListener, state: DaemonState) -> anyhow::Result<()> {
    loop {
        tokio::select! {
            accept_result = listener.accept() => {
                let stream = accept_result?;
                let state = state.clone();
                tokio::spawn(async move {
                    if let Err(e) = connection::handle_connection(stream, state).await {
                        tracing::warn!("connection error: {e}");
                    }
                });
            }
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("shutdown signal received");
                break;
            }
        }
    }
    Ok(())
}
