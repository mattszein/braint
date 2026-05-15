use braint_client::Client;
use braint_proto::{IngestRequest, IngestResponse, METHOD_INGEST, Source};
use crate::output::OutputMode;

pub async fn run(
    verb_prefix: &str,
    text: &str,
    source: Source,
    socket: &str,
    mode: &OutputMode,
) -> crate::error::Result<()> {
    let full_text = format!("{verb_prefix} {text}");

    let client = Client::connect(socket)
        .await
        .map_err(|e| crate::error::CliError::Daemon(e.to_string()))?;

    let req = IngestRequest { text: full_text, source };
    let resp: IngestResponse = client
        .send(METHOD_INGEST, &req)
        .await
        .map_err(|e| crate::error::CliError::Daemon(e.to_string()))?;

    match resp {
        IngestResponse::Committed { entry_id } => {
            crate::output::print_id("committed", &entry_id.to_string(), mode);
            Ok(())
        }
        IngestResponse::Pending { pending_id, .. } => {
            crate::output::print_id("pending", &pending_id.to_string(), mode);
            Ok(())
        }
    }
}
