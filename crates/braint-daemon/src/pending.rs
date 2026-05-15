//! Pending voice confirmations with TTL-based expiry.
//!
//! Voice-sourced entries are held here until the user confirms or cancels.
//! Entries that are not acted on within the TTL are silently dropped.

use braint_proto::{Entry, PendingId};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// In-memory store of pending voice-sourced entries awaiting confirmation.
pub struct PendingMap {
    entries: HashMap<PendingId, (Entry, Instant)>,
    ttl: Duration,
}

impl PendingMap {
    /// Create a new `PendingMap` with the given TTL in seconds.
    pub fn new(ttl_secs: u64) -> Self {
        Self {
            entries: HashMap::new(),
            ttl: Duration::from_secs(ttl_secs),
        }
    }

    /// Insert a pending entry under the given id.
    pub fn insert(&mut self, id: PendingId, entry: Entry) {
        self.entries.insert(id, (entry, Instant::now()));
    }

    /// Returns the entry if found and not expired, removing it from the map.
    ///
    /// Returns `None` if the id is unknown or the TTL has elapsed.
    pub fn take(&mut self, id: PendingId) -> Option<Entry> {
        match self.entries.remove(&id) {
            Some((entry, inserted_at)) if inserted_at.elapsed() < self.ttl => Some(entry),
            Some(_) => None, // expired
            None => None,
        }
    }

    /// Returns `true` if the id exists in the map (even if expired).
    pub fn contains(&self, id: &PendingId) -> bool {
        self.entries.contains_key(id)
    }

    /// Remove all entries older than the TTL. Should be called periodically.
    pub fn sweep(&mut self) {
        let ttl = self.ttl;
        self.entries
            .retain(|_, (_, inserted_at)| inserted_at.elapsed() < ttl);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use braint_proto::{DeviceId, Entry, EntryId, EntryKind, HybridLogicalClock, PendingId};

    fn make_entry() -> Entry {
        let device = DeviceId::generate();
        let hlc = HybridLogicalClock { physical_ms: 1, logical: 0, device_id: device };
        Entry {
            id: EntryId::generate(),
            kind: EntryKind::Idea,
            body: "pending".to_string(),
            created_at: hlc,
            created_on_device: device,
            last_modified_at: hlc,
            last_modified_on_device: device,
            project: None,
            tags: Default::default(),
        }
    }

    #[test]
    fn insert_and_take_happy_path() {
        let mut map = PendingMap::new(60);
        let id = PendingId::generate();
        let entry = make_entry();
        let body = entry.body.clone();
        map.insert(id, entry);
        let taken = map.take(id).expect("should be present");
        assert_eq!(taken.body, body);
        // second take returns None
        assert!(map.take(id).is_none());
    }

    #[test]
    fn take_unknown_id_returns_none() {
        let mut map = PendingMap::new(60);
        assert!(map.take(PendingId::generate()).is_none());
    }

    #[test]
    fn take_expired_entry_returns_none() {
        let mut map = PendingMap::new(0); // 0s TTL → always expired
        let id = PendingId::generate();
        map.insert(id, make_entry());
        // Even 0s TTL expires immediately
        assert!(map.take(id).is_none());
    }

    #[test]
    fn contains_present_vs_absent() {
        let mut map = PendingMap::new(60);
        let id = PendingId::generate();
        assert!(!map.contains(&id));
        map.insert(id, make_entry());
        assert!(map.contains(&id));
    }

    #[test]
    fn sweep_removes_expired() {
        let mut map = PendingMap::new(0); // 0s TTL
        let id1 = PendingId::generate();
        let id2 = PendingId::generate();
        map.insert(id1, make_entry());
        map.insert(id2, make_entry());
        map.sweep();
        assert!(!map.contains(&id1));
        assert!(!map.contains(&id2));
    }

    #[test]
    fn sweep_keeps_fresh_entries() {
        let mut map = PendingMap::new(60);
        let id = PendingId::generate();
        map.insert(id, make_entry());
        map.sweep();
        assert!(map.contains(&id));
    }
}
