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
