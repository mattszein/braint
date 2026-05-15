//! Parse module — turns raw text into structured domain types.
//!
//! Phase 1: `parse_ingest` creates a full `Entry` from text (no verb parsing).
//! Phase 2: `parse_verb` extracts structured `VerbInvocation` from verb grammar.

pub mod verb;

pub use verb::{parse_verb, VerbInvocation};

use crate::error::Result;
use braint_proto::{DeviceId, Entry, EntryId, EntryKind, HybridLogicalClock};

/// Parse free-form text into an Entry.
/// Phase 1: always returns EntryKind::Idea, no project, no tags.
pub fn parse_ingest(text: &str, device_id: DeviceId, hlc: HybridLogicalClock) -> Result<Entry> {
    Ok(Entry {
        id: EntryId::generate(),
        kind: EntryKind::Idea,
        body: text.to_string(),
        created_at: hlc,
        created_on_device: device_id,
        last_modified_at: hlc,
        last_modified_on_device: device_id,
        project: None,
        tags: Default::default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_idea() {
        let device = DeviceId::generate();
        let hlc = HybridLogicalClock {
            physical_ms: 1,
            logical: 0,
            device_id: device,
        };
        let entry = parse_ingest("explore CRDTs", device, hlc).unwrap();
        assert_eq!(entry.kind, EntryKind::Idea);
        assert_eq!(entry.body, "explore CRDTs");
        assert_eq!(entry.created_on_device, device);
    }
}
