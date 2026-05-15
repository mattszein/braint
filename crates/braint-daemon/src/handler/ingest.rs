//! Ingest handler — parse verb and commit immediately for all sources.

use crate::server::state::DaemonState;
use braint_core::parse_verb;
use braint_proto::{
    ERR_PARSE, ERR_STORAGE, EntryChange, EntryId, IngestRequest, IngestResponse, JsonRpcError,
};

/// Handle an ingest request: parse the verb and commit to storage.
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

    let id: EntryId = entry.id;
    let kind = entry.kind;
    let body = entry.body.clone();

    state
        .storage
        .lock()
        .await
        .save(&entry)
        .map_err(|e| JsonRpcError::new(ERR_STORAGE, format!("storage error: {e}")))?;

    state.subs.publish(EntryChange::Created, &entry).await;

    Ok(IngestResponse::Committed {
        entry_id: id,
        kind,
        body,
    })
}
