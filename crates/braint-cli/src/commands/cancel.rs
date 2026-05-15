use braint_client::Client;
use braint_proto::{CancelRequest, PendingId, METHOD_CANCEL};
use uuid::Uuid;
use crate::output::OutputMode;

pub async fn run(pending_id_str: &str, socket: &str, mode: &OutputMode) -> crate::error::Result<()> {
    let uuid = Uuid::parse_str(pending_id_str)
        .map_err(|e| crate::error::CliError::Daemon(format!("invalid pending id: {e}")))?;
    let pending_id = PendingId(uuid);

    let client = Client::connect(socket)
        .await
        .map_err(|e| crate::error::CliError::Daemon(e.to_string()))?;

    let req = CancelRequest { pending_id };
    let _resp: braint_proto::CancelResponse = client
        .send(METHOD_CANCEL, &req)
        .await
        .map_err(|e| crate::error::CliError::Daemon(e.to_string()))?;

    match mode {
        crate::output::OutputMode::Human => println!("cancelled"),
        crate::output::OutputMode::Ndjson => {
            let v = serde_json::json!({ "type": "cancelled", "pending_id": pending_id_str });
            println!("{v}");
        }
    }
    Ok(())
}
