//! Storage module — SQLite persistence for the daemon.
//!
//! Starts as a module inside daemon. Promote to its own crate later if it earns it.
//! This is the ONLY place in the daemon that knows about rusqlite.

mod connection;
pub mod entry;
pub mod migrations;
mod query;

use braint_proto::{Entry, EntryId};
use rusqlite::Connection;
use std::path::Path;

pub use connection::open as open_connection;

pub struct Storage {
    conn: Connection,
}

impl Storage {
    pub fn open(path: &Path) -> crate::error::Result<Self> {
        let mut conn = connection::open(path)?;
        migrations::run(&mut conn)?;
        Ok(Self { conn })
    }

    pub fn save(&mut self, entry: &Entry) -> crate::error::Result<()> {
        let params = entry::bind_entry(entry);
        self.conn.execute(
            "INSERT INTO entries
             (id, kind, body,
              created_at_physical_ms, created_at_logical, created_on_device,
              last_modified_at_physical_ms, last_modified_at_logical, last_modified_on_device)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            ",
            params,
        )?;
        Ok(())
    }

    pub fn get(&self, id: EntryId) -> crate::error::Result<Option<Entry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, kind, body,
                    created_at_physical_ms, created_at_logical, created_on_device,
                    last_modified_at_physical_ms, last_modified_at_logical, last_modified_on_device
             FROM entries WHERE id = ?1",
        )?;
        let mut rows = stmt.query([id.0.as_bytes()])?;
        match rows.next()? {
            Some(row) => Ok(Some(entry::row_to_entry(row)?)),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use braint_proto::{DeviceId, Entry, EntryId, EntryKind, HybridLogicalClock};

    fn test_entry() -> Entry {
        let device = DeviceId::generate();
        let hlc = HybridLogicalClock {
            physical_ms: 1,
            logical: 0,
            device_id: device,
        };
        Entry {
            id: EntryId::generate(),
            kind: EntryKind::Idea,
            body: "test".to_string(),
            created_at: hlc,
            created_on_device: device,
            last_modified_at: hlc,
            last_modified_on_device: device,
        }
    }

    #[test]
    fn save_and_get_roundtrip() {
        let tempdir = tempfile::tempdir().unwrap();
        let mut storage = Storage::open(&tempdir.path().join("test.db")).unwrap();
        let entry = test_entry();
        storage.save(&entry).unwrap();
        let fetched = storage.get(entry.id).unwrap().expect("row should exist");
        assert_eq!(fetched.body, entry.body);
        assert_eq!(fetched.kind, entry.kind);
        assert_eq!(fetched.id.0, entry.id.0);
    }
}
