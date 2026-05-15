//! Confirm and cancel handlers for voice-sourced pending entries.

use braint_proto::{
    CancelRequest, CancelResponse, ConfirmRequest, ConfirmResponse, EntryChange, JsonRpcError,
    ERR_NOT_FOUND, ERR_STORAGE, ERR_TTL_EXPIRED,
};
use crate::server::state::DaemonState;

/// Handle a confirm request: move the pending entry into durable storage.
///
/// Returns [`ERR_NOT_FOUND`] if the id was never registered,
/// and [`ERR_TTL_EXPIRED`] if the entry existed but its TTL has elapsed.
pub async fn handle_confirm(
    state: &DaemonState,
    req: ConfirmRequest,
) -> Result<ConfirmResponse, JsonRpcError> {
    let mut pending = state.pending.lock().await;
    // Check if it existed at all (even if expired).
    if !pending.contains(&req.pending_id) {
        return Err(JsonRpcError::new(ERR_NOT_FOUND, "pending entry not found"));
    }
    let entry = pending.take(req.pending_id).ok_or_else(|| {
        JsonRpcError::new(ERR_TTL_EXPIRED, "pending entry expired")
    })?;
    drop(pending);

    let entry_id = entry.id;
    state
        .storage
        .lock()
        .await
        .save(&entry)
        .map_err(|e| JsonRpcError::new(ERR_STORAGE, format!("storage error: {e}")))?;

    state.subs.publish(EntryChange::Created, &entry).await;

    Ok(ConfirmResponse { entry_id })
}

/// Handle a cancel request: discard a pending voice entry.
///
/// Silently succeeds even if the id is unknown or already expired.
pub async fn handle_cancel(
    state: &DaemonState,
    req: CancelRequest,
) -> Result<CancelResponse, JsonRpcError> {
    state.pending.lock().await.take(req.pending_id);
    Ok(CancelResponse {})
}
