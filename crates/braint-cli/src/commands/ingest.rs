use crate::output::OutputMode;
use braint_client::Client;
use braint_proto::{EntryKind, IngestRequest, IngestResponse, METHOD_INGEST, Source};

pub async fn run(
    verb_prefix: &str,
    text: &str,
    source: Source,
    socket: &str,
    mode: &OutputMode,
) -> crate::error::Result<()> {
    let full_text = if verb_prefix.is_empty() {
        text.to_string()
    } else {
        format!("{verb_prefix} {text}")
    };

    let client = Client::connect(socket)
        .await
        .map_err(|e| crate::error::CliError::Daemon(e.to_string()))?;

    let req = IngestRequest {
        text: full_text,
        source,
    };
    let resp: IngestResponse = client
        .send(METHOD_INGEST, &req)
        .await
        .map_err(|e| crate::error::CliError::Daemon(e.to_string()))?;

    match resp {
        IngestResponse::Committed {
            entry_id,
            kind,
            body,
        } => {
            if source == Source::Voice {
                notify_committed(kind, &body);
            } else {
                crate::output::print_id("committed", &entry_id.to_string(), mode);
            }
            Ok(())
        }
        IngestResponse::Pending { pending_id, .. } => {
            crate::output::print_id("pending", &pending_id.to_string(), mode);
            Ok(())
        }
    }
}

fn kind_action(kind: EntryKind) -> &'static str {
    match kind {
        EntryKind::Idea => "idea added",
        EntryKind::Todo => "todo added",
        EntryKind::Note => "note added",
        EntryKind::Capture => "captured",
    }
}

/// Fire-and-forget informational notification after a voice ingest commits.
fn notify_committed(kind: EntryKind, body: &str) {
    let action = kind_action(kind);
    let preview = if body.len() > 80 {
        format!("{}…", &body[..80])
    } else {
        body.to_string()
    };
    let message = format!("{action}: {preview}");

    let _ = std::process::Command::new("notify-send")
        .args(["--expire-time=4000", "braint", &message])
        .spawn();
}
