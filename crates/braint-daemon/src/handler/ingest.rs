//! Ingest handler — parse verb and commit immediately for all sources.

use crate::server::state::DaemonState;
use braint_core::parse_verb;
use braint_proto::{
    ERR_PARSE, ERR_STORAGE, EntryChange, EntryId, IngestRequest, IngestResponse, JsonRpcError,
    PluginVerbRequest, PluginVerbResponse,
};

/// Handle an ingest request: parse the verb, route to plugin or built-in handler.
pub async fn handle(
    state: &DaemonState,
    req: IngestRequest,
) -> Result<IngestResponse, JsonRpcError> {
    let invocation = parse_verb(&req.text)
        .map_err(|e| JsonRpcError::new(ERR_PARSE, format!("parse error: {e}")))?;

    // If a plugin owns this verb, route there.
    if state.plugins.owns_verb(&invocation.verb) {
        // For `takes_entry_id` verbs, parse the first body token as a UUID and fetch the entry.
        let current_entry = if state.plugins.verb_takes_entry_id(&invocation.verb) {
            let entry_id = parse_entry_id_from_body(&invocation.body)
                .map_err(|e| JsonRpcError::new(ERR_PARSE, format!("expected entry ID: {e}")))?;
            state
                .storage
                .lock()
                .await
                .get(entry_id)
                .map_err(|e| JsonRpcError::new(ERR_STORAGE, format!("storage error: {e}")))?
        } else {
            None
        };

        let plugin_req = PluginVerbRequest {
            verb: invocation.verb.clone(),
            body: invocation.body.clone(),
            project: invocation.project.clone(),
            tags: invocation.tags.clone(),
            current_entry,
        };

        let plugin_resp = state
            .plugins
            .route_verb(&invocation.verb, plugin_req)
            .await
            .map_err(|e| JsonRpcError::new(-32000, format!("plugin error: {e}")))?;

        return handle_plugin_response(state, plugin_resp).await;
    }

    // Built-in verb: kind must be Some.
    let kind = invocation.kind.ok_or_else(|| {
        JsonRpcError::new(ERR_PARSE, format!("unknown verb: {}", invocation.verb))
    })?;

    let hlc = state.clock.now();
    let entry = braint_proto::Entry {
        id: EntryId::generate(),
        kind,
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

/// Parse the first whitespace-separated token of `body` as a UUID [`EntryId`].
fn parse_entry_id_from_body(body: &str) -> Result<braint_proto::EntryId, String> {
    let first_token = body.split_whitespace().next().ok_or("empty body")?;
    let uuid = uuid::Uuid::parse_str(first_token).map_err(|e| e.to_string())?;
    Ok(braint_proto::EntryId(uuid))
}

/// Persist a [`PluginVerbResponse`] to storage and notify subscribers.
async fn handle_plugin_response(
    state: &DaemonState,
    resp: PluginVerbResponse,
) -> Result<IngestResponse, JsonRpcError> {
    match resp {
        PluginVerbResponse::Create { entry } => {
            let id = entry.id;
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
        PluginVerbResponse::Update { entry } => {
            let id = entry.id;
            let kind = entry.kind;
            let body = entry.body.clone();
            state
                .storage
                .lock()
                .await
                .update(&entry)
                .map_err(|e| JsonRpcError::new(ERR_STORAGE, format!("storage error: {e}")))?;
            state.subs.publish(EntryChange::Updated, &entry).await;
            Ok(IngestResponse::Committed {
                entry_id: id,
                kind,
                body,
            })
        }
        PluginVerbResponse::Noop => {
            // Return a synthetic "committed" with a nil UUID to signal no-op.
            Ok(IngestResponse::Committed {
                entry_id: EntryId(uuid::Uuid::nil()),
                kind: braint_proto::EntryKind::Capture,
                body: String::new(),
            })
        }
    }
}
