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
}

/// Source of the ingest request. Used in Phase 2+ for confirmation policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    Cli,
    Voice,
}
