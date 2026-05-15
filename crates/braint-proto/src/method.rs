use crate::{Entry, EntryId, EntryKind, PrincipalTag, ProjectId};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const METHOD_INGEST: &str = "ingest";
pub const METHOD_CONFIRM: &str = "confirm";
pub const METHOD_CANCEL: &str = "cancel";
pub const METHOD_SUBSCRIBE: &str = "subscribe";
pub const METHOD_UNSUBSCRIBE: &str = "unsubscribe";
pub const METHOD_LIST: &str = "list";
pub const METHOD_NOTIFY_ENTRY_CHANGED: &str = "notify.entry_changed";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestRequest {
    pub text: String,
    pub source: crate::Source,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(transparent)]
pub struct PendingId(pub Uuid);

impl PendingId {
    pub fn generate() -> Self { Self(Uuid::now_v7()) }
}

impl std::fmt::Display for PendingId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(transparent)]
pub struct SubscriptionId(pub Uuid);

impl SubscriptionId {
    pub fn generate() -> Self { Self(Uuid::now_v7()) }
}

impl std::fmt::Display for SubscriptionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum IngestResponse {
    Committed { entry_id: EntryId },
    Pending { pending_id: PendingId, preview: Entry },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmRequest { pub pending_id: PendingId }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmResponse { pub entry_id: EntryId }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelRequest { pub pending_id: PendingId }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelResponse {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionTopic {
    Scratch,
    RecentActivity,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EntryFilter {
    pub kind: Option<EntryKind>,
    pub project: Option<ProjectId>,
    pub free_tags: Vec<String>,
    pub principal_match: Vec<PrincipalTag>,
    pub untriaged: bool,
    pub since_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscribeRequest {
    pub topic: SubscriptionTopic,
    pub filter: EntryFilter,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscribeResponse { pub subscription_id: SubscriptionId }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnsubscribeRequest { pub subscription_id: SubscriptionId }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnsubscribeResponse {}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ListRequest {
    pub filter: EntryFilter,
    pub limit: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResponse {
    pub entries: Vec<Entry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryChange { Created, Updated, Deleted }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryChangeNotification {
    pub subscription_id: SubscriptionId,
    pub change: EntryChange,
    pub entry: Entry,
}
