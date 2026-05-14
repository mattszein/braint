//! Socket server — accepts connections and dispatches to handlers.

pub mod connection;
pub mod state;

use interprocess::local_socket::tokio::prelude::*;

pub async fn run(
    listener: LocalSocketListener,
    mut handler: crate::handler::IngestHandler,
) -> anyhow::Result<()> {
    loop {
        let stream = listener.accept().await?;
        // NOTE(debt-5): Phase 1 uses sequential handling.
        // Phase 2+ will spawn per-connection tasks with Arc<Mutex<Storage>>.
        if let Err(e) = connection::handle_connection(stream, &mut handler).await {
            tracing::warn!("connection error: {e}");
        }
    }
}
