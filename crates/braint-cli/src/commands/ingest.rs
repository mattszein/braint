use braint_client::Client;
use braint_proto::{IngestRequest, JsonRpcRequest, METHOD_INGEST, Source};

pub async fn run(text: String) -> crate::error::Result<()> {
    let socket_path = std::env::var_os("XDG_RUNTIME_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("braint.sock")
        .to_string_lossy()
        .to_string();

    let mut client = Client::connect(&socket_path)
        .await
        .map_err(|e| crate::error::CliError::Daemon(e.to_string()))?;

    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: 1,
        method: METHOD_INGEST.to_string(),
        params: IngestRequest {
            text,
            source: Source::Cli,
        },
    };

    let resp: braint_proto::JsonRpcResponse<braint_proto::IngestResponse> = client
        .send(&req)
        .await
        .map_err(|e| crate::error::CliError::Daemon(e.to_string()))?;

    match resp.result {
        Some(r) => {
            println!("{}", r.entry_id);
            Ok(())
        }
        None => {
            let msg = resp.error.map(|e| e.message).unwrap_or_default();
            Err(crate::error::CliError::Daemon(msg))
        }
    }
}
