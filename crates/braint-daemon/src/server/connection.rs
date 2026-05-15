//! Per-connection handler: reads frames, dispatches methods, writes responses and notifications.

use braint_client::framing::{read_frame, write_frame};
use braint_proto::{
    CancelRequest, ConfirmRequest, IngestRequest, JsonRpcError, JsonRpcResponse, ListRequest,
    SubscribeRequest, UnsubscribeRequest, METHOD_CANCEL, METHOD_CONFIRM, METHOD_INGEST,
    METHOD_LIST, METHOD_SUBSCRIBE, METHOD_UNSUBSCRIBE,
};
use crate::{handler, server::state::DaemonState, subscription::ConnectionId};
use interprocess::local_socket::tokio::prelude::*;
use serde_json::Value;
use tokio::sync::mpsc;

/// Drive a single client connection to completion.
///
/// Reads length-prefixed JSON-RPC frames, dispatches to the appropriate handler,
/// and writes responses. Outbound subscription notifications are multiplexed via
/// an internal channel and written between request/response cycles.
pub async fn handle_connection(
    stream: LocalSocketStream,
    state: DaemonState,
) -> anyhow::Result<()> {
    let connection_id = ConnectionId::generate();
    let (mut reader, mut writer) = stream.split();

    // Channel for outbound notifications from subscriptions to this connection's writer.
    let (notify_tx, mut notify_rx) = mpsc::channel::<Vec<u8>>(1024);

    loop {
        tokio::select! {
            frame_result = read_frame(&mut reader) => {
                match frame_result {
                    Ok(frame) => {
                        let response_bytes = dispatch(&frame, &state, connection_id, &notify_tx).await;
                        write_frame(&mut writer, &response_bytes).await?;
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                    Err(e) => return Err(e.into()),
                }
            }
            Some(notif_bytes) = notify_rx.recv() => {
                write_frame(&mut writer, &notif_bytes).await?;
            }
        }
    }

    state.subs.unregister_connection(connection_id).await;
    Ok(())
}

/// Deserialise and route a single JSON-RPC frame to its handler. Always returns serialised bytes.
async fn dispatch(
    frame: &[u8],
    state: &DaemonState,
    connection_id: ConnectionId,
    notify_tx: &mpsc::Sender<Vec<u8>>,
) -> Vec<u8> {
    let request: braint_proto::JsonRpcRequest<Value> = match serde_json::from_slice(frame) {
        Ok(r) => r,
        Err(e) => {
            let err = JsonRpcResponse::<Value>::err(
                0,
                JsonRpcError::new(-32700, format!("parse error: {e}")),
            );
            return serde_json::to_vec(&err).unwrap_or_default();
        }
    };

    let id = request.id;
    let method = request.method.as_str();

    macro_rules! parse_params {
        ($T:ty) => {
            match serde_json::from_value::<$T>(request.params.clone()) {
                Ok(p) => p,
                Err(e) => {
                    let err = JsonRpcResponse::<Value>::err(
                        id,
                        JsonRpcError::new(-32602, format!("invalid params: {e}")),
                    );
                    return serde_json::to_vec(&err).unwrap_or_default();
                }
            }
        };
    }

    macro_rules! respond {
        ($result:expr) => {
            match $result {
                Ok(r) => serde_json::to_vec(&JsonRpcResponse::ok(id, r)).unwrap_or_default(),
                Err(e) => {
                    serde_json::to_vec(&JsonRpcResponse::<Value>::err(id, e)).unwrap_or_default()
                }
            }
        };
    }

    match method {
        METHOD_INGEST => {
            let req = parse_params!(IngestRequest);
            respond!(handler::ingest::handle(state, req).await)
        }
        METHOD_CONFIRM => {
            let req = parse_params!(ConfirmRequest);
            respond!(handler::confirm::handle_confirm(state, req).await)
        }
        METHOD_CANCEL => {
            let req = parse_params!(CancelRequest);
            respond!(handler::confirm::handle_cancel(state, req).await)
        }
        METHOD_SUBSCRIBE => {
            let req = parse_params!(SubscribeRequest);
            respond!(
                handler::subscribe::handle_subscribe(state, req, connection_id, notify_tx).await
            )
        }
        METHOD_UNSUBSCRIBE => {
            let req = parse_params!(UnsubscribeRequest);
            respond!(handler::subscribe::handle_unsubscribe(state, req).await)
        }
        METHOD_LIST => {
            let req = parse_params!(ListRequest);
            respond!(handler::list::handle(state, req).await)
        }
        _ => {
            let err = JsonRpcResponse::<Value>::err(
                id,
                JsonRpcError::new(-32601, format!("method not found: {method}")),
            );
            serde_json::to_vec(&err).unwrap_or_default()
        }
    }
}
