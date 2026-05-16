//! `PluginHandle` — a live handle to a running plugin child process.

use braint_proto::{
    JsonRpcRequest, JsonRpcResponse, PluginVerbRequest, PluginVerbResponse, METHOD_PLUGIN_VERB,
    plugin::PluginManifest,
};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use tokio::process::Child;
use tokio::sync::{Mutex, mpsc, oneshot};

/// Global monotonic counter for plugin request IDs.
static NEXT_ID: AtomicI64 = AtomicI64::new(1);

/// A live, connected plugin process.
///
/// All fields are `Arc`-wrapped so the handle is cheaply cloneable and
/// `Send + Sync` for use in `Arc<PluginManager>`.
pub struct PluginHandle {
    /// The manifest the plugin declared at load time.
    pub manifest: PluginManifest,
    /// Send raw (unframed payload) bytes to the writer task which frames and forwards them to plugin stdin.
    pub frame_tx: mpsc::Sender<Vec<u8>>,
    /// Pending in-flight RPC requests: request id → response channel.
    pub pending: Arc<Mutex<HashMap<i64, oneshot::Sender<JsonRpcResponse<Value>>>>>,
    /// The child process (kept alive for shutdown).
    pub child: Arc<Mutex<Child>>,
}

impl PluginHandle {
    /// Send a plugin verb request and await the response.
    ///
    /// Encodes the request as a JSON-RPC call, sends it over the frame channel,
    /// and awaits until the plugin sends back a matching response.
    pub async fn call_verb(
        &self,
        req: PluginVerbRequest,
    ) -> Result<PluginVerbResponse, crate::error::DaemonError> {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let rpc_req = JsonRpcRequest::<Value> {
            jsonrpc: "2.0".into(),
            id,
            method: METHOD_PLUGIN_VERB.into(),
            params: serde_json::to_value(&req)?,
        };
        let frame = serde_json::to_vec(&rpc_req)?;

        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(id, tx);

        self.frame_tx
            .send(frame)
            .await
            .map_err(|_| crate::error::DaemonError::PluginDead(self.manifest.name.clone()))?;

        let response = rx
            .await
            .map_err(|_| crate::error::DaemonError::PluginDead(self.manifest.name.clone()))?;

        if let Some(err) = response.error {
            return Err(crate::error::DaemonError::PluginError(err.message));
        }

        let result = response.result.unwrap_or(Value::Null);
        let plugin_resp: PluginVerbResponse = serde_json::from_value(result)?;
        Ok(plugin_resp)
    }
}
