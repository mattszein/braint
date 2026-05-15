//! Ingest handler — parse verb, persist or pend based on source.

use braint_core::parse_verb;
use braint_proto::{
    EntryChange, EntryId, IngestRequest, IngestResponse, JsonRpcError, PendingId, Source,
    ERR_PARSE, ERR_STORAGE,
};
use crate::server::state::DaemonState;

/// Handle an ingest request: parse the verb, then either commit immediately or hold pending.
///
/// Voice-sourced entries are placed in the pending map and returned as [`IngestResponse::Pending`].
/// All other sources are committed directly to storage.
pub async fn handle(
    state: &DaemonState,
    req: IngestRequest,
) -> Result<IngestResponse, JsonRpcError> {
    let invocation = parse_verb(&req.text)
        .map_err(|e| JsonRpcError::new(ERR_PARSE, format!("parse error: {e}")))?;

    let hlc = state.clock.now();
    let entry = braint_proto::Entry {
        id: braint_proto::EntryId::generate(),
        kind: invocation.kind,
        body: invocation.body,
        project: invocation.project,
        tags: invocation.tags,
        created_at: hlc,
        created_on_device: state.device_id,
        last_modified_at: hlc,
        last_modified_on_device: state.device_id,
    };

    if req.source == Source::Voice {
        let pending_id = PendingId::generate();
        let preview = entry.clone();
        state.pending.lock().await.insert(pending_id, entry);
        return Ok(IngestResponse::Pending { pending_id, preview });
    }

    let id: EntryId = entry.id;
    state
        .storage
        .lock()
        .await
        .save(&entry)
        .map_err(|e| JsonRpcError::new(ERR_STORAGE, format!("storage error: {e}")))?;

    state.subs.publish(EntryChange::Created, &entry).await;

    Ok(IngestResponse::Committed { entry_id: id })
}
