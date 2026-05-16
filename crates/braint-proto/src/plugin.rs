//! Plugin protocol types — manifests, verb requests, and responses.

use crate::{Entry, EntryKind, ProjectId, TagSet};
use serde::{Deserialize, Serialize};

/// Protocol version this daemon supports. Plugins must declare a matching value.
pub const PLUGIN_API_VERSION: u32 = 1;

/// Describes a single verb contributed by a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerbManifest {
    /// Verb name (lowercase, no punctuation). Must be unique across all loaded plugins.
    pub name: String,
    /// Human-readable description shown in help text.
    pub description: String,
    /// If `true`, the daemon parses the body as an `EntryId`, fetches the entry,
    /// and populates `PluginVerbRequest::current_entry`.
    pub takes_entry_id: bool,
}

/// Static metadata a plugin declares at load time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Unique plugin name; used as the event-topic prefix.
    pub name: String,
    /// Semver version string (informational only).
    pub version: String,
    /// Must equal [`PLUGIN_API_VERSION`] for the daemon to load the plugin.
    pub api_version: u32,
    /// All verbs this plugin contributes.
    pub verbs: Vec<VerbManifest>,
    /// Event topic glob patterns this plugin subscribes to (Phase 7).
    pub events_subscribed: Vec<String>,
    /// Entry kinds this plugin owns (informational; Phase 6 enforcement).
    pub kinds_owned: Vec<EntryKind>,
}

/// Daemon → Plugin: a forwarded verb invocation over the plugin transport.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginVerbRequest {
    /// The verb being invoked.
    pub verb: String,
    /// Free-form body text extracted by the parser.
    pub body: String,
    /// Optional project extracted from the header tokens.
    pub project: Option<ProjectId>,
    /// Structured and free tags extracted from the header tokens.
    pub tags: TagSet,
    /// Populated when the verb's [`VerbManifest::takes_entry_id`] is `true`.
    pub current_entry: Option<Entry>,
}

/// Plugin → Daemon: the plugin's response to a verb invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum PluginVerbResponse {
    /// Create and persist a new entry.
    Create { entry: Entry },
    /// Overwrite an existing entry.
    Update { entry: Entry },
    /// Nothing to persist.
    Noop,
}
