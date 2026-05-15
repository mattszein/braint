//! List handler — query existing entries with optional filter.

use braint_proto::{JsonRpcError, ListRequest, ListResponse, ERR_STORAGE};
use crate::server::state::DaemonState;
use crate::subscription::filter::filter_matches;

pub async fn handle(state: &DaemonState, req: ListRequest) -> Result<ListResponse, JsonRpcError> {
    let entries = state
        .storage
        .lock()
        .await
        .list(req.limit)
        .map_err(|e| JsonRpcError::new(ERR_STORAGE, format!("storage error: {e}")))?;

    // Apply filter client-side (storage returns all; filter is applied in memory for Phase 2).
    // Phase 4+: push filter into SQL for performance.
    let filtered: Vec<_> = entries
        .into_iter()
        .filter(|e| filter_matches(&req.filter, e))
        .collect();

    Ok(ListResponse { entries: filtered })
}
