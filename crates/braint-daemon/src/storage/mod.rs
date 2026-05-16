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

/// SQLite-backed entry store.
pub struct Storage {
    conn: Connection,
}

impl Storage {
    /// Open (or create) the database at `path`, running any pending migrations.
    pub fn open(path: &Path) -> crate::error::Result<Self> {
        let mut conn = connection::open(path)?;
        migrations::run(&mut conn)?;
        Ok(Self { conn })
    }

    /// Persist `entry` to the database.
    pub fn save(&mut self, entry: &Entry) -> crate::error::Result<()> {
        let params = entry::bind_entry(entry);
        self.conn.execute(
            "INSERT INTO entries
             (id, kind, body,
              created_at_physical_ms, created_at_logical, created_on_device,
              last_modified_at_physical_ms, last_modified_at_logical, last_modified_on_device,
              project, principal_tags, free_tags)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            ",
            params,
        )?;
        Ok(())
    }

    /// List entries ordered newest-first. Requires migration 0002 (project/tags columns).
    pub fn list(&self, limit: Option<u64>) -> crate::error::Result<Vec<Entry>> {
        let limit_clause = limit.map(|l| format!("LIMIT {l}")).unwrap_or_default();
        let sql = format!(
            "SELECT id, kind, body,
                    created_at_physical_ms, created_at_logical, created_on_device,
                    last_modified_at_physical_ms, last_modified_at_logical, last_modified_on_device,
                    project, principal_tags, free_tags
             FROM entries ORDER BY created_at_physical_ms DESC, created_at_logical DESC {limit_clause}"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map([], entry::row_to_entry)?;
        let mut entries = Vec::new();
        for row in rows {
            entries.push(row?);
        }
        Ok(entries)
    }

    /// Update an existing entry in the database.
    ///
    /// Overwrites all mutable fields for the row matching `entry.id`.
    pub fn update(&mut self, entry: &Entry) -> crate::error::Result<()> {
        let params = entry::bind_entry(entry);
        self.conn.execute(
            "UPDATE entries SET
               kind = ?2,
               body = ?3,
               created_at_physical_ms = ?4,
               created_at_logical = ?5,
               created_on_device = ?6,
               last_modified_at_physical_ms = ?7,
               last_modified_at_logical = ?8,
               last_modified_on_device = ?9,
               project = ?10,
               principal_tags = ?11,
               free_tags = ?12
             WHERE id = ?1",
            params,
        )?;
        Ok(())
    }

    /// Retrieve an entry by id, or `None` if it does not exist.
    pub fn get(&self, id: EntryId) -> crate::error::Result<Option<Entry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, kind, body,
                    created_at_physical_ms, created_at_logical, created_on_device,
                    last_modified_at_physical_ms, last_modified_at_logical, last_modified_on_device,
                    project, principal_tags, free_tags
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
            project: None,
            tags: Default::default(),
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

    #[test]
    fn list_returns_all_newest_first() {
        let tempdir = tempfile::tempdir().unwrap();
        let mut storage = Storage::open(&tempdir.path().join("test.db")).unwrap();

        let device = DeviceId::generate();
        let make = |ms: u64, body: &str| {
            let hlc = HybridLogicalClock {
                physical_ms: ms,
                logical: 0,
                device_id: device,
            };
            Entry {
                id: EntryId::generate(),
                kind: EntryKind::Idea,
                body: body.to_string(),
                created_at: hlc,
                created_on_device: device,
                last_modified_at: hlc,
                last_modified_on_device: device,
                project: None,
                tags: Default::default(),
            }
        };

        let e1 = make(1000, "oldest");
        let e2 = make(2000, "middle");
        let e3 = make(3000, "newest");
        storage.save(&e1).unwrap();
        storage.save(&e2).unwrap();
        storage.save(&e3).unwrap();

        let all = storage.list(None).unwrap();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].body, "newest");
        assert_eq!(all[1].body, "middle");
        assert_eq!(all[2].body, "oldest");
    }

    #[test]
    fn list_with_limit() {
        let tempdir = tempfile::tempdir().unwrap();
        let mut storage = Storage::open(&tempdir.path().join("test.db")).unwrap();

        let device = DeviceId::generate();
        for ms in [1000u64, 2000, 3000] {
            let hlc = HybridLogicalClock {
                physical_ms: ms,
                logical: 0,
                device_id: device,
            };
            storage
                .save(&Entry {
                    id: EntryId::generate(),
                    kind: EntryKind::Idea,
                    body: format!("entry-{ms}"),
                    created_at: hlc,
                    created_on_device: device,
                    last_modified_at: hlc,
                    last_modified_on_device: device,
                    project: None,
                    tags: Default::default(),
                })
                .unwrap();
        }

        let limited = storage.list(Some(1)).unwrap();
        assert_eq!(limited.len(), 1);
        assert_eq!(limited[0].body, "entry-3000");
    }

    #[test]
    fn list_empty_db() {
        let tempdir = tempfile::tempdir().unwrap();
        let storage = Storage::open(&tempdir.path().join("test.db")).unwrap();
        let entries = storage.list(None).unwrap();
        assert!(entries.is_empty());
    }
}
