use crate::output::OutputMode;
use braint_client::Client;
use braint_proto::{ConfirmRequest, ConfirmResponse, METHOD_CONFIRM, PendingId};
use uuid::Uuid;

pub async fn run(
    pending_id_str: &str,
    socket: &str,
    mode: &OutputMode,
) -> crate::error::Result<()> {
    let uuid = Uuid::parse_str(pending_id_str)
        .map_err(|e| crate::error::CliError::Daemon(format!("invalid pending id: {e}")))?;
    let pending_id = PendingId(uuid);

    let client = Client::connect(socket)
        .await
        .map_err(|e| crate::error::CliError::Daemon(e.to_string()))?;

    let req = ConfirmRequest { pending_id };
    let resp: ConfirmResponse = client
        .send(METHOD_CONFIRM, &req)
        .await
        .map_err(|e| crate::error::CliError::Daemon(e.to_string()))?;

    crate::output::print_id("committed", &resp.entry_id.to_string(), mode);
    Ok(())
}
