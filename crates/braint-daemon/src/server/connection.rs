use braint_proto::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};
use interprocess::local_socket::tokio::prelude::*;
use serde_json::Value;

pub async fn handle_connection(
    mut stream: LocalSocketStream,
    handler: &mut crate::handler::IngestHandler,
) -> anyhow::Result<()> {
    loop {
        let frame = match braint_client::framing::read_frame(&mut stream).await {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e.into()),
        };

        let request: JsonRpcRequest<Value> = match serde_json::from_slice(&frame) {
            Ok(r) => r,
            Err(e) => {
                let err_resp = JsonRpcResponse::<Value>::err(
                    0,
                    JsonRpcError::new(-32700, format!("parse error: {e}")),
                );
                let bytes = serde_json::to_vec(&err_resp)?;
                braint_client::framing::write_frame(&mut stream, &bytes).await?;
                continue;
            }
        };

        // NOTE: match-based routing is fine for one method. Revisit when plugins arrive.
        // TODO(phase-4a): Evaluate dynamic routing when plugins introduce runtime verbs.
        let response: JsonRpcResponse<Value> = match request.method.as_str() {
            braint_proto::METHOD_INGEST => {
                let params: braint_proto::IngestRequest =
                    match serde_json::from_value(request.params) {
                        Ok(p) => p,
                        Err(e) => {
                            let resp: JsonRpcResponse<serde_json::Value> = JsonRpcResponse::err(
                                request.id,
                                JsonRpcError::new(-32602, format!("invalid params: {e}")),
                            );
                            let bytes = serde_json::to_vec(&resp)?;
                            braint_client::framing::write_frame(&mut stream, &bytes).await?;
                            continue;
                        }
                    };
                match handler.handle(params) {
                    Ok(result) => JsonRpcResponse::ok(request.id, serde_json::to_value(result)?),
                    Err(e) => JsonRpcResponse::err(request.id, e),
                }
            }
            _ => JsonRpcResponse::err(
                request.id,
                JsonRpcError::new(-32601, format!("method not found: {}", request.method)),
            ),
        };

        let bytes = serde_json::to_vec(&response)?;
        braint_client::framing::write_frame(&mut stream, &bytes).await?;
    }

    Ok(())
}
