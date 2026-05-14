# Phase 1 Implementation Plan — Skeleton End-to-End

> Derived from `personal-daemon-build-plan.md` §Phase 1 and `personal-daemon-phase-details.md` §Phase 1.
> Phase 0 (workspace skeleton) is complete. This plan bridges the high-level spec to concrete files, signatures, and execution order.
> **All crates follow the conventions in `docs/ARCHITECTURE.md`.**

---

## Goal

A CLI command sends a JSON-RPC `ingest` request over a local socket; the daemon receives it, persists an `Entry` to SQLite via `rusqlite`, and responds with the entry's `EntryId`. The simplest possible pipe, working end-to-end.

**Demo:** `cargo run --bin braintd` in one terminal. In another: `cargo run --bin braint ingest "explore CRDTs for sync"`. Get an ID back. Inspect SQLite directly and see the row.

---

## Crate Graph (unchanged from Phase 0)

```
         ┌─────────────┐
         │  braint-cli │
         └──────┬──────┘
                │ depends on
    ┌───────────┼───────────┐
    ▼           ▼           ▼
braint-client  braint-core  braint-proto
    │              │
    └──────────────┘
                │
         ┌──────▼──────┐
         │braint-daemon│
         │  + storage  │
         └─────────────┘
```

Rules enforced:
- `braint-proto` depends on nothing workspace-local (only `serde`, `uuid`).
- `braint-core` depends only on `braint-proto`.
- `braint-client` depends only on `braint-proto`.
- `braint-daemon` depends on `braint-proto`, `braint-core`, `rusqlite`, `tokio`, `interprocess`, `tracing`.
- `braint-cli` depends on `braint-proto`, `braint-core`, `braint-client`, `clap`, `tokio`.
- `braint-plugin-sdk` is untouched this phase (still a placeholder).

---

## Step 1 — `braint-proto`: wire types and JSON-RPC envelope

**Layout:**

```
crates/braint-proto/src/
├── lib.rs              # Table of contents: re-export modules, version constant
├── entry.rs            # Entry, EntryId, EntryKind, HLC, DeviceId, Source
├── jsonrpc.rs          # JsonRpcRequest, JsonRpcResponse, JsonRpcError
└── method.rs           # METHOD_INGEST + IngestRequest, IngestResponse
```

### 1.1 Entry types (`src/entry.rs`)

```rust
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
```

### 1.2 JSON-RPC envelope (`src/jsonrpc.rs`)

Match JSON-RPC 2.0 spec. Use `i64` request IDs (daemon uses positive; we reserve negative for Phase 2+ bidirectional RPC).

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest<T> {
    pub jsonrpc: String,
    pub id: i64,
    pub method: String,
    pub params: T,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse<T> {
    pub jsonrpc: String,
    pub id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl JsonRpcError {
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }
}

impl<T> JsonRpcResponse<T> {
    pub fn ok(id: i64, result: T) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn err(id: i64, error: JsonRpcError) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(error),
        }
    }
}
```

### 1.3 Method constants and request/response types (`src/method.rs`)

```rust
use serde::{Deserialize, Serialize};
use crate::{EntryId, Source};

pub const METHOD_INGEST: &str = "ingest";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestRequest {
    pub text: String,
    pub source: Source,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestResponse {
    pub entry_id: EntryId,
}
```

### 1.4 `src/lib.rs`

```rust
//! braint-proto — wire types and protocol contracts.
//!
//! This crate contains everything that crosses a crate boundary.
//! No logic, no I/O, no async.

pub mod entry;
pub mod jsonrpc;
pub mod method;

pub use entry::*;
pub use jsonrpc::*;
pub use method::*;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
```

**Key decisions locked:**
- `serde_json` for serialization. No `bincode` until a measured bottleneck appears (per tech doc).
- `EntryId` is a newtype, not a bare `Uuid`. Prevents mixing ID types later.
- `Source` included now even though Phase 1 only uses `Cli`. Prevents a breaking proto change in Phase 2.
- `lib.rs` is a pure table of contents. No logic.

---

## Step 2 — `braint-core`: parse and ID generation

**Layout:**

```
crates/braint-core/src/
├── lib.rs              # Re-export submodules
├── error.rs            # CoreError (thiserror)
├── parse.rs            # parse_ingest, parse_verb (Phase 2)
└── clock.rs            # Clock, HLC generation
```

### 2.1 Error type (`src/error.rs`)

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("parse error: {0}")]
    Parse(String),
}

pub type Result<T> = std::result::Result<T, CoreError>;
```

### 2.2 Ingest parser (`src/parse.rs`)

Pure function. No async, no I/O.

```rust
use braint_proto::{Entry, EntryId, EntryKind, DeviceId, HybridLogicalClock};
use crate::error::Result;

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
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use braint_proto::{EntryKind, HybridLogicalClock, DeviceId};

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
```

### 2.3 HLC and device ID (`src/clock.rs`)

```rust
use braint_proto::{DeviceId, HybridLogicalClock};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Clock state for this daemon instance.
pub struct Clock {
    device_id: DeviceId,
    logical: AtomicU32,
}

impl Clock {
    pub fn new(device_id: DeviceId) -> Self {
        Self {
            device_id,
            logical: AtomicU32::new(0),
        }
    }

    pub fn now(&self) -> HybridLogicalClock {
        let physical_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before 1970")
            .as_millis() as u64;
        let logical = self.logical.fetch_add(1, Ordering::SeqCst);
        HybridLogicalClock {
            physical_ms,
            logical,
            device_id: self.device_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_increments_logical() {
        let device = DeviceId::generate();
        let clock = Clock::new(device);
        let a = clock.now();
        let b = clock.now();
        assert_eq!(b.logical, a.logical + 1);
        assert!(b.physical_ms >= a.physical_ms);
    }
}
```

### 2.4 `src/lib.rs`

```rust
//! braint-core — pure domain logic.
//!
//! No SQLite, no sockets, no tokio. Parse, validate, filter, clock.

pub mod clock;
pub mod error;
pub mod parse;

pub use clock::Clock;
pub use error::{CoreError, Result};
pub use parse::parse_ingest;
```

**Notes:**
- HLC update rule from phase-details: `new = max(current_hlc + (0,1,_), (now_ms, 0, device))`. Phase 1 simplifies because we're single-writer per process; the `AtomicU32` is sufficient. When sync arrives (v2), we'll need to merge remote HLCs.
- `DeviceId` generation: load from a config file, or generate a fresh UUIDv7 and persist it. For Phase 1, `daemon::main` generates one in-memory on each start. Persistent device ID lands in Phase 2 when config files exist.

---

## Step 3 — `braint-daemon::storage`: SQLite persistence

**Layout:**

```
crates/braint-daemon/src/
├── lib.rs              # Re-export public API for tests
├── main.rs             # Minimal: init tracing, build deps, call daemon::run()
├── error.rs            # DaemonError (thiserror)
├── server/
│   ├── mod.rs          # run(listener, state) → accept loop
│   ├── connection.rs   # One connection: read frame → dispatch → write frame
│   └── state.rs        # Shared daemon state (Phase 2+)
├── handler/
│   ├── mod.rs          # Dispatch table: method name → handler fn
│   └── ingest.rs       # handle_ingest: core::parse + storage::save
└── storage/
    ├── mod.rs          # Storage struct, public API
    ├── connection.rs   # Connection open, WAL, pragmas
    ├── migrations/
    │   ├── mod.rs      # Migration runner
    │   └── 0001_entries.sql
    ├── entry.rs        # Entry CRUD: save, get
    └── query.rs        # Query builders (Phase 4+)
```

### 3.1 Error type (`src/error.rs`)

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DaemonError {
    #[error("storage error: {0}")]
    Storage(#[from] rusqlite::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON-RPC error: {0}")]
    JsonRpc(String),
}

pub type Result<T> = std::result::Result<T, DaemonError>;
```

### 3.2 Migration SQL (`src/storage/migrations/0001_entries.sql`)

```sql
CREATE TABLE IF NOT EXISTS entries (
    id BLOB PRIMARY KEY NOT NULL,
    kind TEXT NOT NULL,
    body TEXT NOT NULL,
    created_at_physical_ms INTEGER NOT NULL,
    created_at_logical INTEGER NOT NULL,
    created_on_device BLOB NOT NULL,
    last_modified_at_physical_ms INTEGER NOT NULL,
    last_modified_at_logical INTEGER NOT NULL,
    last_modified_on_device BLOB NOT NULL
) STRICT;
```

Use `BLOB` for UUIDs (compact, index-friendly). `TEXT` for `kind` (extensible).

### 3.3 Migration runner (`src/storage/migrations/mod.rs`)

Small hand-rolled runner (no `refinery` yet — we have one migration). When we hit 3+ migrations, pull in `rusqlite_migration` or `refinery`.

```rust
use rusqlite::Connection;

const MIGRATIONS: &[(&str, &str)] = &[("0001_entries", include_str!("0001_entries.sql"))];

pub fn run(conn: &mut Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS __migrations (name TEXT PRIMARY KEY);",
    )?;
    for (name, sql) in MIGRATIONS {
        let exists: bool = conn
            .query_row(
                "SELECT 1 FROM __migrations WHERE name = ?1",
                [name],
                |_| Ok(true),
            )
            .unwrap_or(false);
        if !exists {
            conn.execute_batch(sql)?;
            conn.execute(
                "INSERT INTO __migrations (name) VALUES (?1)",
                [name],
            )?;
        }
    }
    Ok(())
}
```

### 3.4 Connection setup (`src/storage/connection.rs`)

```rust
use rusqlite::Connection;
use std::path::Path;

pub fn open(path: &Path) -> rusqlite::Result<Connection> {
    let mut conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA journal_mode = WAL;")?;
    Ok(conn)
}
```

### 3.5 Entry serialization helpers (`src/storage/entry.rs`)

```rust
use braint_proto::{Entry, EntryId, EntryKind, HybridLogicalClock, DeviceId};
use rusqlite::{params, Row};

pub fn encode_kind(kind: EntryKind) -> String {
    match kind {
        EntryKind::Idea => "idea".to_string(),
    }
}

pub fn decode_kind(s: &str) -> Option<EntryKind> {
    match s {
        "idea" => Some(EntryKind::Idea),
        _ => None,
    }
}

pub fn bind_entry(entry: &Entry) -> impl rusqlite::Params + '_ {
    params![
        entry.id.0.as_bytes(),
        encode_kind(entry.kind),
        &entry.body,
        entry.created_at.physical_ms as i64,
        entry.created_at.logical as i64,
        entry.created_on_device.0.as_bytes(),
        entry.last_modified_at.physical_ms as i64,
        entry.last_modified_at.logical as i64,
        entry.last_modified_on_device.0.as_bytes(),
    ]
}

pub fn row_to_entry(row: &Row) -> rusqlite::Result<Entry> {
    let id_bytes: Vec<u8> = row.get(0)?;
    let kind_str: String = row.get(1)?;
    let body: String = row.get(2)?;
    let created_at_physical_ms: i64 = row.get(3)?;
    let created_at_logical: i64 = row.get(4)?;
    let created_on_device_bytes: Vec<u8> = row.get(5)?;
    let last_modified_at_physical_ms: i64 = row.get(6)?;
    let last_modified_at_logical: i64 = row.get(7)?;
    let last_modified_on_device_bytes: Vec<u8> = row.get(8)?;

    let id = EntryId(
        uuid::Uuid::from_slice(&id_bytes)
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Blob,
                Box::new(e),
            ))?,
    );
    let kind = decode_kind(&kind_str)
        .ok_or_else(|| rusqlite::Error::FromSqlConversionFailure(
            1,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("unknown kind: {kind_str}"),
            )),
        ))?;
    let device_from_bytes = |bytes: Vec<u8>| -> rusqlite::Result<DeviceId> {
        uuid::Uuid::from_slice(&bytes)
            .map(DeviceId)
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Blob,
                Box::new(e),
            ))
    };

    Ok(Entry {
        id,
        kind,
        body,
        created_at: HybridLogicalClock {
            physical_ms: created_at_physical_ms as u64,
            logical: created_at_logical as u32,
            device_id: device_from_bytes(created_on_device_bytes)?,
        },
        created_on_device: device_from_bytes(created_on_device_bytes)?,
        last_modified_at: HybridLogicalClock {
            physical_ms: last_modified_at_physical_ms as u64,
            logical: last_modified_at_logical as u32,
            device_id: device_from_bytes(last_modified_on_device_bytes.clone())?,
        },
        last_modified_on_device: device_from_bytes(last_modified_on_device_bytes)?,
    })
}
```

### 3.6 Storage API (`src/storage/mod.rs`)

```rust
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
        self.conn.execute(
            "INSERT INTO entries
             (id, kind, body,
              created_at_physical_ms, created_at_logical, created_on_device,
              last_modified_at_physical_ms, last_modified_at_logical, last_modified_on_device)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            ",
            entry::bind_entry(entry),
        )?;
        Ok(())
    }

    pub fn get(&self, id: EntryId) -> crate::error::Result<Option<Entry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, kind, body,
                    created_at_physical_ms, created_at_logical, created_on_device,
                    last_modified_at_physical_ms, last_modified_at_logical, last_modified_on_device
             FROM entries WHERE id = ?1"
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
    use braint_proto::{Entry, EntryId, EntryKind, HybridLogicalClock, DeviceId};

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
        let mut storage = Storage::open(tempdir.path().join("test.db")).unwrap();
        let entry = test_entry();
        storage.save(&entry).unwrap();
        let fetched = storage.get(entry.id).unwrap().expect("row should exist");
        assert_eq!(fetched.body, entry.body);
        assert_eq!(fetched.kind, entry.kind);
        assert_eq!(fetched.id.0, entry.id.0);
    }
}
```

**Gotcha:** WAL mode creates `-wal` and `-shm` sidecar files. Tests must point at a writable temp directory (use `tempfile` crate).

---

## Step 4 — `braint-client`: IPC connect-and-send

**Layout:**

```
crates/braint-client/src/
├── lib.rs              # Re-export Client, framing
├── error.rs            # ClientError (thiserror)
├── framing.rs          # Length-prefixed JSON
└── client.rs           # Client struct
```

### 4.1 Error type (`src/error.rs`)

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("daemon unreachable: {0}")]
    DaemonUnreachable(String),
}

pub type Result<T> = std::result::Result<T, ClientError>;
```

### 4.2 Length-prefixed framing (`src/framing.rs`)

The tech doc and phase-details both mandate **length-prefixed JSON** (4-byte big-endian length, then JSON bytes). Do NOT use newline-delimited JSON over a streaming socket.

```rust
use tokio::io::{AsyncReadExt, AsyncWriteExt};

// NOTE: Manual framing is 20 lines and zero extra deps.
// TODO(phase-2): Evaluate tokio-util::codec::LengthDelimitedCodec when subscriptions arrive.
pub async fn write_frame<W: AsyncWriteExt + Unpin>(
    writer: &mut W,
    payload: &[u8],
) -> std::io::Result<()> {
    let len = payload.len() as u32;
    writer.write_all(&len.to_be_bytes()).await?;
    writer.write_all(payload).await?;
    writer.flush().await?;
    Ok(())
}

pub async fn read_frame<R: AsyncReadExt + Unpin>(
    reader: &mut R,
) -> std::io::Result<Vec<u8>> {
    let mut len_buf = [0u8; 4];
    reader.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf).await?;
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn roundtrip_frame() {
        let mut buf: Vec<u8> = Vec::new();
        write_frame(&mut buf, b"hello").await.unwrap();
        let mut cursor = std::io::Cursor::new(buf);
        let read = read_frame(&mut cursor).await.unwrap();
        assert_eq!(read, b"hello");
    }
}
```

### 4.3 Client API (`src/client.rs`)

```rust
use braint_proto::{JsonRpcRequest, JsonRpcResponse};
use interprocess::local_socket::tokio::LocalSocketStream;
use serde::{de::DeserializeOwned, Serialize};

pub struct Client {
    stream: LocalSocketStream,
}

impl Client {
    pub async fn connect(path: &str) -> crate::error::Result<Self> {
        let stream = LocalSocketStream::connect(path)
            .await
            .map_err(|e| crate::error::ClientError::DaemonUnreachable(e.to_string()))?;
        Ok(Self { stream })
    }

    pub async fn send<Req, Resp>(
        &mut self,
        request: &JsonRpcRequest<Req>,
    ) -> crate::error::Result<JsonRpcResponse<Resp>>
    where
        Req: Serialize,
        Resp: DeserializeOwned,
    {
        let payload = serde_json::to_vec(request)?;
        crate::framing::write_frame(&mut self.stream, &payload).await?;

        let response_bytes = crate::framing::read_frame(&mut self.stream).await?;
        let response: JsonRpcResponse<Resp> = serde_json::from_slice(&response_bytes)?;
        Ok(response)
    }
}
```

**Decision:** `Client` owns the `LocalSocketStream`. Each request reuses the connection (no reconnect per call). Phase 1 only needs one request/response per CLI invocation.

### 4.4 `src/lib.rs`

```rust
//! braint-client — IPC client for talking to the daemon.
//!
//! Handles connection, length-prefixed JSON-RPC framing, and request/response.

pub mod client;
pub mod error;
pub mod framing;

pub use client::Client;
pub use error::{ClientError, Result};
```

---

## Step 5 — `braint-daemon`: socket server + JSON-RPC dispatch

### 5.1 Handler (`src/handler/ingest.rs`)

Sync function from request → response. Async only for I/O (socket); business logic is sync.

```rust
use braint_core::{parse_ingest, Clock};
use braint_proto::{
    DeviceId, HybridLogicalClock, IngestRequest, IngestResponse, JsonRpcError, Source,
};
use crate::storage::Storage;

pub struct IngestHandler {
    storage: Storage,
    clock: Clock,
    device_id: DeviceId,
}

impl IngestHandler {
    pub fn new(storage: Storage, clock: Clock, device_id: DeviceId) -> Self {
        Self {
            storage,
            clock,
            device_id,
        }
    }

    pub fn handle(&mut self, req: IngestRequest) -> Result<IngestResponse, JsonRpcError> {
        let hlc = self.clock.now();
        let entry = parse_ingest(&req.text, self.device_id, hlc)
            .map_err(|e| JsonRpcError::new(-32000, format!("parse error: {e}")))?;
        let id = entry.id;
        self.storage.save(&entry).map_err(|e| {
            JsonRpcError::new(-32001, format!("storage error: {e}"))
        })?;
        Ok(IngestResponse { entry_id: id })
    }
}
```

### 5.2 Handler dispatch (`src/handler/mod.rs`)

```rust
//! Request dispatch — maps JSON-RPC method names to handlers.

pub mod ingest;

pub use ingest::IngestHandler;
```

### 5.3 Connection handler (`src/server/connection.rs`)

```rust
use braint_proto::{JsonRpcRequest, JsonRpcResponse, JsonRpcError};
use interprocess::local_socket::tokio::LocalSocketStream;
use serde_json::Value;

pub async fn handle_connection(
    mut stream: LocalSocketStream,
    handler: &mut crate::handler::IngestHandler,
) -> anyhow::Result<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    loop {
        let frame = match braint_client::framing::read_frame(&mut stream).await {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e.into()),
        };

        let request: JsonRpcRequest<Value> = match serde_json::from_slice(&frame) {
            Ok(r) => r,
            Err(e) => {
                let err_resp = JsonRpcResponse::<Value>::err(0, JsonRpcError::new(-32700, format!("parse error: {e}")));
                let bytes = serde_json::to_vec(&err_resp)?;
                braint_client::framing::write_frame(&mut stream, &bytes).await?;
                continue;
            }
        };

        // NOTE: match-based routing is fine for one method. Revisit when plugins arrive.
        // TODO(phase-4a): Evaluate dynamic routing when plugins introduce runtime verbs.
        let response = match request.method.as_str() {
            braint_proto::METHOD_INGEST => {
                let params: braint_proto::IngestRequest = match serde_json::from_value(request.params) {
                    Ok(p) => p,
                    Err(e) => {
                        JsonRpcResponse::<braint_proto::IngestResponse>::err(
                            request.id,
                            JsonRpcError::new(-32602, format!("invalid params: {e}")),
                        )
                    }
                };
                match handler.handle(params) {
                    Ok(result) => JsonRpcResponse::ok(request.id, result),
                    Err(e) => JsonRpcResponse::err(request.id, e),
                }
            }
            _ => JsonRpcResponse::<Value>::err(
                request.id,
                JsonRpcError::new(-32601, format!("method not found: {}", request.method)),
            ),
        };

        let bytes = serde_json::to_vec(&response)?;
        braint_client::framing::write_frame(&mut stream, &bytes).await?;
    }

    Ok(())
}
```

### 5.4 Server loop (`src/server/mod.rs`)

```rust
//! Socket server — accepts connections and dispatches to handlers.

pub mod connection;
pub mod state;

use interprocess::local_socket::tokio::LocalSocketListener;

pub async fn run(
    listener: LocalSocketListener,
    mut handler: crate::handler::IngestHandler,
) -> anyhow::Result<()> {
    loop {
        let (stream, _) = listener.accept().await?;
        // Phase 1: sequential handling. Phase 2+ will spawn per-connection tasks.
        if let Err(e) = connection::handle_connection(stream, &mut handler).await {
            tracing::warn!("connection error: {e}");
        }
    }
}
```

### 5.5 State placeholder (`src/server/state.rs`)

```rust
//! Shared daemon state. Expanded in Phase 2+ for subscriptions and pending confirmations.
//!
// TODO(phase-2): Add pending confirmation map, subscription manager.
```

### 5.6 `src/lib.rs`

```rust
//! braint-daemon — the background process.
//!
//! Owns the async runtime, socket, SQLite, and wiring between them.
//! Business logic delegates to `braint_core`; persistence delegates to `storage`.

pub mod error;
pub mod handler;
pub mod server;
pub mod storage;

pub use error::{DaemonError, Result};
```

### 5.7 Main (`src/main.rs`)

```rust
use braint_core::Clock;
use braint_daemon::{handler::IngestHandler, server, storage::Storage};
use braint_proto::DeviceId;
use interprocess::local_socket::tokio::LocalSocketListener;
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir());
    let socket_path = runtime_dir.join("braint.sock");
    let db_path = runtime_dir.join("braint.db");

    // Cleanup stale socket
    let _ = std::fs::remove_file(&socket_path);

    let storage = Storage::open(&db_path)?;
    let device_id = DeviceId::generate(); // Phase 1: ephemeral; Phase 2: persisted
    let clock = Clock::new(device_id);
    let handler = IngestHandler::new(storage, clock, device_id);

    let listener = LocalSocketListener::bind(socket_path.to_string_lossy().as_ref())?;
    tracing::info!("daemon listening on {:?}", socket_path);

    server::run(listener, handler).await
}
```

**Gotchas addressed:**
- **Stale socket cleanup:** `remove_file` before `bind`. On Linux, a crash leaves the UDS file; next start fails with `EADDRINUSE`.
- **WAL mode:** `Storage::open` sets `PRAGMA journal_mode = WAL`.
- **Graceful shutdown:** Phase 1 can skip signal handling (Ctrl-C just kills the process). Add `tokio::signal::ctrl_c` in Phase 2.
- **Note:** `Storage` is `!Send` because `rusqlite::Connection` is `!Send`. Phase 1 handles one connection at a time, so `IngestHandler` stays in one task. Phase 2 will need `tokio::sync::Mutex<Storage>` or a connection pool.

---

## Step 6 — `braint-cli`: one subcommand + socket client

**Layout:**

```
crates/braint-cli/src/
├── main.rs             # Arg parsing, dispatch, exit codes
├── error.rs            # CliError (thiserror)
├── args.rs             # Clap derive structs
├── commands/
│   ├── mod.rs          # dispatch(cmd)
│   └── ingest.rs       # ingest handler
└── output.rs           # Human vs NDJSON output (Phase 2+)
```

### 6.1 Error type (`src/error.rs`)

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CliError {
    #[error("daemon error: {0}")]
    Daemon(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, CliError>;
```

### 6.2 Args (`src/args.rs`)

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "braint", about = "Personal daemon CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub cmd: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Ingest a raw text string as an idea
    Ingest {
        /// The text to ingest
        text: String,
    },
}
```

### 6.3 Command handlers (`src/commands/mod.rs`)

```rust
pub mod ingest;

use crate::args::Command;

pub async fn dispatch(cmd: Command) -> crate::error::Result<()> {
    match cmd {
        Command::Ingest { text } => ingest::run(text).await,
    }
}
```

### 6.4 Ingest command (`src/commands/ingest.rs`)

```rust
use braint_client::Client;
use braint_proto::{
    IngestRequest, JsonRpcRequest, Source, METHOD_INGEST,
};

pub async fn run(text: String) -> crate::error::Result<()> {
    let socket_path = std::env::var_os("XDG_RUNTIME_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir())
        .join("braint.sock")
        .to_string_lossy()
        .to_string();

    let mut client = Client::connect(&socket_path)
        .await
        .map_err(|e| crate::error::CliError::Daemon(e.to_string()))?;

    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: 1,
        method: METHOD_INGEST.to_string(),
        params: IngestRequest { text, source: Source::Cli },
    };

    let resp: braint_proto::JsonRpcResponse<braint_proto::IngestResponse> = client
        .send(&req)
        .await
        .map_err(|e| crate::error::CliError::Daemon(e.to_string()))?;

    match resp.result {
        Some(r) => {
            println!("{}", r.entry_id);
            Ok(())
        }
        None => {
            let msg = resp.error.map(|e| e.message).unwrap_or_default();
            Err(crate::error::CliError::Daemon(msg))
        }
    }
}
```

### 6.5 `src/output.rs` (placeholder)

```rust
//! Output formatting — human-readable vs NDJSON.
//!
// TODO(phase-2): Implement --json flag handling here.
```

### 6.6 `src/main.rs`

```rust
use clap::Parser;

mod args;
mod commands;
mod error;
mod output;

#[tokio::main]
async fn main() {
    let cli = args::Cli::parse();

    if let Err(e) = commands::dispatch(cli.cmd).await {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
```

**Exit codes:**
- `0` — success
- `1` — any error (daemon unreachable, daemon returned error)

**Note:** Phase 1 only has `ingest`. Phase 2 adds the dual-mode behavior (no args → TUI) and `--json`.

---

## Step 7 — Integration test

**Layout:**

```
crates/braint-daemon/tests/
├── common/
│   └── mod.rs          # spawn_test_daemon(), TestDaemonHandle
└── ingest_e2e.rs       # End-to-end ingest test
```

### 7.1 Test helper (`tests/common/mod.rs`)

```rust
use braint_daemon::{handler::IngestHandler, server, storage::Storage};
use braint_core::Clock;
use braint_proto::DeviceId;
use interprocess::local_socket::tokio::LocalSocketListener;
use std::path::PathBuf;
use tokio::task::JoinHandle;

pub struct TestDaemonHandle {
    pub socket_path: PathBuf,
    pub db_path: PathBuf,
    pub _task: JoinHandle<()>,
    pub _tempdir: tempfile::TempDir,
}

pub async fn spawn_test_daemon() -> TestDaemonHandle {
    // Use short prefix to stay under UDS path limits (108 bytes on Linux)
    let tempdir = tempfile::Builder::new().prefix("b").tempdir().unwrap();
    let socket_path = tempdir.path().join("s.sock");
    let db_path = tempdir.path().join("test.db");

    let storage = Storage::open(&db_path).unwrap();
    let device_id = DeviceId::generate();
    let clock = Clock::new(device_id);
    let handler = IngestHandler::new(storage, clock, device_id);

    let listener = LocalSocketListener::bind(socket_path.to_string_lossy().as_ref()).unwrap();

    let task = tokio::spawn(async move {
        let _ = server::run(listener, handler).await;
    });

    // Give the server a moment to start listening
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    TestDaemonHandle {
        socket_path,
        db_path,
        _task: task,
        _tempdir: tempdir,
    }
}
```

### 7.2 E2E test (`tests/ingest_e2e.rs`)

```rust
mod common;

use braint_client::Client;
use braint_proto::{IngestRequest, JsonRpcRequest, Source, METHOD_INGEST};

#[tokio::test]
async fn ingest_creates_row_in_sqlite() {
    let handle = common::spawn_test_daemon().await;

    let mut client = Client::connect(handle.socket_path.to_str().unwrap())
        .await
        .unwrap();

    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: 1,
        method: METHOD_INGEST.to_string(),
        params: IngestRequest {
            text: "explore CRDTs for sync".to_string(),
            source: Source::Cli,
        },
    };

    let resp: braint_proto::JsonRpcResponse<braint_proto::IngestResponse> =
        client.send(&req).await.unwrap();

    let entry_id = resp.result.unwrap().entry_id;

    // Assert row exists in SQLite
    let conn = rusqlite::Connection::open(&handle.db_path).unwrap();
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM entries WHERE id = ?1",
            [entry_id.0.as_bytes()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
}
```

**Implementation note:** `server::run` and `IngestHandler` are public from `braint-daemon/src/lib.rs` so tests can spawn them in-process. This is faster than process orchestration and gives better error traces.

---

## Step 8 — Dependencies and workspace adjustments

### 8.1 Add `tempfile` and `thiserror` to workspace dependencies

`Cargo.toml` `[workspace.dependencies]` additions:
```toml
tempfile = "3.14"
```

(`thiserror` is already in workspace dependencies.)

### 8.2 Crate dependency updates

**`braint-daemon/Cargo.toml`** — add `tempfile` under `[dev-dependencies]`:
```toml
[dev-dependencies]
tempfile = { workspace = true }
```

**`braint-client/Cargo.toml`** — ensure `tokio` has `io-util`, `net`:
Already present in workspace: `tokio = { version = "1.40", features = ["rt-multi-thread", "macros", "sync", "signal", "process", "io-util", "net"] }`.

**`braint-cli/Cargo.toml`** — no changes needed.

**`braint-core/Cargo.toml`** — already has `thiserror`.

---

## Execution Order (recommended)

| Order | Step | Why this order |
|-------|------|----------------|
| 1 | Proto types + JSON-RPC envelope | Everything downstream depends on these types. |
| 2 | Core parser + clock + errors | Pure logic; can unit-test immediately without I/O. |
| 3 | Daemon storage module | Independent of networking; test with a temp SQLite file. |
| 4 | Client framing + connect | Small, self-contained; test with a mock stream. |
| 5 | Daemon handler + server | Brings together storage + core + socket. |
| 6 | CLI binary | Thin wrapper over client; trivial once client works. |
| 7 | Integration test | Validates the whole pipe; depends on all above. |
| 8 | `cargo check`, `cargo test`, CI green | Final validation. |

---

## Test Plan

| Test | Type | Location | What it validates |
|------|------|----------|-----------------|
| `parse_ingest` unit | Unit | `core/src/parse.rs` | Parser returns `EntryKind::Idea`, populates body. |
| `Clock::now` unit | Unit | `core/src/clock.rs` | HLC increments logical counter, physical_ms non-decreasing. |
| `framing` round-trip | Unit | `client/src/framing.rs` | 4-byte length prefix encodes/decodes correctly. |
| `storage` save+get | Unit | `daemon/src/storage/mod.rs` | SQLite schema correct, serialization reversible. |
| `ingest_e2e` | Integration | `daemon/tests/ingest_e2e.rs` | Full pipe: client → socket → daemon → SQLite → response. |

Target: all tests pass on Linux and macOS in CI.

---

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| `interprocess` tokio API differs from docs | Pin to version `2.2` (already in workspace). Read docs inline; the API is stable. |
| SQLite `BLOB` binding for UUID is awkward | Test round-trip immediately. If painful, switch to `TEXT` (16 hex chars) — still correct, slightly slower. |
| Stale socket file on Linux after crash | `remove_file` before `bind`. Documented in code comment. |
| `tokio::main` in daemon + in-process test runtime conflict | Use `#[tokio::test]` for in-process tests; they share a runtime with the spawned server task. This is fine. |
| `Storage` is `!Send` because `rusqlite::Connection` | Phase 1 handles one connection at a time, so `Storage` stays in one task. Phase 2 will need `tokio::sync::Mutex<Storage>` or a connection pool. |
| `daemon` crate is both lib and bin | `lib.rs` exports testable API; `main.rs` is < 50 lines of glue. No logic duplication. |

---

## Out of Scope (explicitly)

- Multiple verbs (only `ingest`).
- Projects, tags, principal tags.
- Markdown files on disk (bodies live in SQLite).
- Voice input, confirmation flow, TUI.
- Subscriptions / streaming.
- Plugins.
- Graceful shutdown / signal handling.
- Persistent device ID (generated fresh each run in Phase 1).
- Config files.
- NDJSON output (`--json`).

---

## Definition of Done

- [ ] All crates compile with `cargo build --workspace`.
- [ ] All tests pass with `cargo test --workspace`.
- [ ] `cargo clippy --workspace -- -D warnings` is clean.
- [ ] `cargo fmt --check` is clean.
- [ ] CI green on Linux + macOS.
- [ ] Every `.rs` file has a module-level doc comment (`//!`).
- [ ] Every public type and function has a doc comment.
- [ ] Demo works: `cargo run --bin braintd` in one terminal; `cargo run --bin braint ingest "hello"` in another prints an ID; inspecting SQLite shows the row with HLC and device fields populated.

---

## Time Estimate

2–3 days of focused implementation. The integration test is the highest-leverage deliverable — it becomes the scaffold for every subsequent phase's E2E test.
