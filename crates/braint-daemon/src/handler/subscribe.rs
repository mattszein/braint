//! Subscribe / unsubscribe handlers.

use crate::{server::state::DaemonState, subscription::ConnectionId};
use braint_proto::{
    JsonRpcError, SubscribeRequest, SubscribeResponse, UnsubscribeRequest, UnsubscribeResponse,
};
use tokio::sync::mpsc;

/// Handle a subscribe request: register the filter and spawn a forwarding task.
///
/// Notifications from the subscription are serialised and forwarded to `notify_tx`,
/// which is the write channel for this connection's outbound byte stream.
pub async fn handle_subscribe(
    state: &DaemonState,
    req: SubscribeRequest,
    connection_id: ConnectionId,
    notify_tx: &mpsc::Sender<Vec<u8>>,
) -> Result<SubscribeResponse, JsonRpcError> {
    let (sub_id, mut rx) = state.subs.subscribe(req.filter, connection_id).await;

    // Spawn a task that forwards notifications from this subscription to the connection's writer.
    let tx = notify_tx.clone();
    tokio::spawn(async move {
        while let Some(notif) = rx.recv().await {
            match serde_json::to_vec(&notif) {
                Ok(bytes) => {
                    if tx.send(bytes).await.is_err() {
                        break; // connection closed
                    }
                }
                Err(e) => {
                    tracing::warn!("failed to serialize notification: {e}");
                }
            }
        }
    });

    Ok(SubscribeResponse {
        subscription_id: sub_id,
    })
}

/// Handle an unsubscribe request: remove the given subscription.
pub async fn handle_unsubscribe(
    state: &DaemonState,
    req: UnsubscribeRequest,
) -> Result<UnsubscribeResponse, JsonRpcError> {
    state.subs.unsubscribe(req.subscription_id).await;
    Ok(UnsubscribeResponse {})
}
