use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Newtype wrapper around UUIDv7. Always use this, never raw Uuid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(transparent)]
pub struct EntryId(pub Uuid);

impl EntryId {
    pub fn generate() -> Self {
        Self(Uuid::now_v7())
    }
}

impl std::fmt::Display for EntryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryKind {
    Idea,
    Todo,
    Note,
    Capture,
}

/// Hybrid Logical Clock: (physical_ms, logical, device_id).
/// Lexicographic compare gives total ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HybridLogicalClock {
    pub physical_ms: u64,
    pub logical: u32,
    pub device_id: DeviceId,
}

/// Stable identifier for this daemon instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(transparent)]
pub struct DeviceId(pub Uuid);

impl DeviceId {
    pub fn generate() -> Self {
        Self(Uuid::now_v7())
    }
}

/// An entry is the unit of capture in the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub id: EntryId,
    pub kind: EntryKind,
    pub body: String,
    pub created_at: HybridLogicalClock,
    pub created_on_device: DeviceId,
    pub last_modified_at: HybridLogicalClock,
    pub last_modified_on_device: DeviceId,
    pub project: Option<ProjectId>,
    pub tags: TagSet,
}

/// Free-form string project identifier. v2 may switch to Uuid.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(transparent)]
pub struct ProjectId(pub String);

impl std::fmt::Display for ProjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// A recognized structural tag. Serialized as "prefix:value" strings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "k", content = "v")]
pub enum PrincipalTag {
    Status(String),
    Priority(String),
    When(String),
    Due(String),
    Scope(String),
    Repeat(String),
    Type(String),
}

impl PrincipalTag {
    pub fn prefix(&self) -> &'static str {
        match self {
            Self::Status(_) => "status",
            Self::Priority(_) => "priority",
            Self::When(_) => "when",
            Self::Due(_) => "due",
            Self::Scope(_) => "scope",
            Self::Repeat(_) => "repeat",
            Self::Type(_) => "type",
        }
    }

    pub fn value(&self) -> &str {
        match self {
            Self::Status(v) | Self::Priority(v) | Self::When(v) | Self::Due(v)
            | Self::Scope(v) | Self::Repeat(v) | Self::Type(v) => v,
        }
    }
}

impl std::fmt::Display for PrincipalTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.prefix(), self.value())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagSet {
    pub principal: Vec<PrincipalTag>,
    pub free: Vec<String>,
}

/// Source of the ingest request. Used in Phase 2+ for confirmation policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    Cli,
    Voice,
}
