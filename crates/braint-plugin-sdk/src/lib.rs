//! braint-plugin-sdk — helpers for writing braint plugins.
//!
//! Plugins are simple single-threaded binaries that communicate with the daemon
//! over stdin/stdout using length-prefixed JSON-RPC frames.
//!
//! # Quick start
//!
//! ```no_run
//! use braint_plugin_sdk::Plugin;
//! use braint_proto::plugin::PluginVerbResponse;
//!
//! fn main() {
//!     Plugin::new("myplugin", "0.1.0")
//!         .verb("myverb", "Does something useful", false, |req| {
//!             Ok(PluginVerbResponse::Noop)
//!         })
//!         .run();
//! }
//! ```

pub mod error;
pub mod transport;

use braint_proto::{
    JsonRpcError, JsonRpcRequest, JsonRpcResponse,
    METHOD_PLUGIN_VERB,
    PluginVerbRequest, PluginVerbResponse,
    plugin::{PluginManifest, VerbManifest},
};
use std::collections::HashMap;
use std::io::{self, BufReader, BufWriter};

type HandlerFn =
    Box<dyn Fn(PluginVerbRequest) -> std::result::Result<PluginVerbResponse, String> + Send + Sync>;

/// A plugin descriptor that carries verb handlers and runs the dispatch loop.
pub struct Plugin {
    name: String,
    version: String,
    verb_manifests: Vec<VerbManifest>,
    handlers: HashMap<String, HandlerFn>,
}

impl Plugin {
    /// Create a new plugin with the given name and semver version string.
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            verb_manifests: Vec::new(),
            handlers: HashMap::new(),
        }
    }

    /// Register a verb handler.
    ///
    /// - `name`: verb name (lowercase, no punctuation).
    /// - `description`: human-readable description for help text.
    /// - `takes_entry_id`: if `true`, the daemon parses the body as an `EntryId`,
    ///   fetches the entry, and populates `PluginVerbRequest::current_entry`.
    /// - `handler`: called for each invocation; returns a [`PluginVerbResponse`]
    ///   or an error message string.
    pub fn verb<F>(
        mut self,
        name: &str,
        description: &str,
        takes_entry_id: bool,
        handler: F,
    ) -> Self
    where
        F: Fn(PluginVerbRequest) -> std::result::Result<PluginVerbResponse, String>
            + Send
            + Sync
            + 'static,
    {
        self.verb_manifests.push(VerbManifest {
            name: name.to_string(),
            description: description.to_string(),
            takes_entry_id,
        });
        self.handlers.insert(name.to_string(), Box::new(handler));
        self
    }

    /// Build the [`PluginManifest`] from the registered verbs.
    fn manifest(&self) -> PluginManifest {
        use braint_proto::plugin::PLUGIN_API_VERSION;
        PluginManifest {
            name: self.name.clone(),
            version: self.version.clone(),
            api_version: PLUGIN_API_VERSION,
            verbs: self.verb_manifests.clone(),
            events_subscribed: Vec::new(),
            kinds_owned: Vec::new(),
        }
    }

    /// Run the plugin's dispatch loop.
    ///
    /// - If `--manifest` appears anywhere in `argv`, prints the manifest as JSON and exits.
    /// - Otherwise, reads length-prefixed JSON-RPC frames from stdin until EOF.
    pub fn run(self) {
        // --manifest short-circuit
        if std::env::args().any(|a| a == "--manifest") {
            let manifest = self.manifest();
            println!(
                "{}",
                serde_json::to_string(&manifest).expect("manifest serialization failed")
            );
            return;
        }

        let stdin = io::stdin();
        let stdout = io::stdout();
        let mut reader = BufReader::new(stdin.lock());
        let mut writer = BufWriter::new(stdout.lock());

        loop {
            let frame = match transport::read_frame(&mut reader) {
                Ok(f) => f,
                Err(_) => break, // stdin closed → exit cleanly
            };

            let request: JsonRpcRequest<serde_json::Value> =
                match serde_json::from_slice(&frame) {
                    Ok(r) => r,
                    Err(e) => {
                        let resp = JsonRpcResponse::<serde_json::Value>::err(
                            0,
                            JsonRpcError::new(-32700, format!("parse error: {e}")),
                        );
                        let _ = transport::write_frame(
                            &mut writer,
                            &serde_json::to_vec(&resp).unwrap(),
                        );
                        continue;
                    }
                };

            let id = request.id;
            let resp: JsonRpcResponse<serde_json::Value> = self.dispatch(id, request);

            let bytes = serde_json::to_vec(&resp).unwrap();
            if transport::write_frame(&mut writer, &bytes).is_err() {
                break; // stdout closed
            }
        }
    }

    /// Dispatch a single JSON-RPC request to the appropriate verb handler.
    fn dispatch(
        &self,
        id: i64,
        request: JsonRpcRequest<serde_json::Value>,
    ) -> JsonRpcResponse<serde_json::Value> {
        if request.method != METHOD_PLUGIN_VERB {
            return JsonRpcResponse::err(
                id,
                JsonRpcError::new(-32601, format!("method not found: {}", request.method)),
            );
        }

        let plugin_req: PluginVerbRequest = match serde_json::from_value(request.params) {
            Ok(r) => r,
            Err(e) => {
                return JsonRpcResponse::err(
                    id,
                    JsonRpcError::new(-32602, format!("invalid params: {e}")),
                );
            }
        };

        let verb_name = plugin_req.verb.clone();
        match self.handlers.get(&verb_name) {
            Some(handler) => match handler(plugin_req) {
                Ok(result) => {
                    JsonRpcResponse::ok(id, serde_json::to_value(result).unwrap())
                }
                Err(msg) => JsonRpcResponse::err(id, JsonRpcError::new(-32000, msg)),
            },
            None => JsonRpcResponse::err(
                id,
                JsonRpcError::new(-32601, format!("no handler for verb: {verb_name}")),
            ),
        }
    }
}
