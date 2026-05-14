# Architecture Conventions — braint

> Rules for how code is organized inside every crate. Read this before writing code in any phase. Update it when a new pattern is established.

---

## Guiding Principles

1. **A module is a unit of cohesion.** One file = one responsibility. If you describe a module with "and," split it.
2. **Dependency direction is strict.** Inner layers (core, storage) do not know about outer layers (server, CLI, TUI). The `daemon` crate is the adapter layer; it owns the async runtime and wires pure logic to I/O.
3. **Errors are structured and typed.** Every crate defines its own error enum with `thiserror`. `anyhow` is for binaries and top-level glue only — never for library crates.
4. **Pure logic is sync and testable without tokio.** If a function needs `async`, question whether it should be in `core`.
5. **Performance is by design, not by optimization.** Zero-copy where natural (borrowing), explicit cloning where needed, no premature abstraction.
6. **Public API is minimal.** Re-export only what callers need. Internal modules stay `pub(crate)`.

---

## Crate-Level Layout

### Every crate follows this directory convention

```
crates/braint-{name}/
├── Cargo.toml
├── src/
│   ├── lib.rs          # Minimal: re-exports, feature flags, doc root
│   ├── error.rs        # ONE error enum for the whole crate (thiserror)
│   ├── {domain}/       # One directory per major concern
│   │   ├── mod.rs
│   │   ├── ...
│   └── {domain}/
│       ├── mod.rs
│       └── ...
└── tests/
    ├── common/
    │   └── mod.rs      # Shared test helpers (setup, builders, fixtures)
    └── {feature}_test.rs
```

Rules:
- `lib.rs` is a table of contents, not a junk drawer. No logic, no long functions.
- `error.rs` lives at crate root. All submodules return `crate::error::Error` or `Result<T, crate::error::Error>`.
- Integration tests live in `tests/`, not inside `src/`.
- Unit tests live in the same file as the code they test (`#[cfg(test)] mod tests`).

---

## Crate-by-Crate Architecture

### `braint-proto` — The contract layer

**Responsibility:** Everything that crosses a crate boundary. Types, wire format, method names. No logic, no I/O.

```
src/
├── lib.rs              # Re-export all submodules; version constant
├── entry.rs            # Entry, EntryId, EntryKind, HLC, DeviceId, Source, TagSet
├── jsonrpc.rs          # JsonRpcRequest, JsonRpcResponse, JsonRpcError
└── method.rs           # Method name constants (METHOD_INGEST, etc.)
                     # + request/response types per method (IngestRequest, IngestResponse)
```

**Rules:**
- All types derive `Serialize + Deserialize`.
- Newtypes over primitive types (`EntryId(Uuid)`, `DeviceId(Uuid)`) prevent accidental mixing.
- No `async`, no `tokio`, no `std::fs`, no `rusqlite`.

---

### `braint-core` — The domain layer

**Responsibility:** Pure business logic. Parsing, validation, query building, filtering. No SQLite, no sockets, no `tokio`.

```
src/
├── lib.rs              # Re-export submodules
├── error.rs            # CoreError (ParseError, ValidationError, ...)
├── parse.rs            # Text → Entry (parse_ingest, parse_verb, ...)
├── clock.rs            # HLC generation, device ID handling
├── filter.rs           # focus_filter, query builders (Phase 6+)
└── view.rs             # Read-only views over Entry (Task view, etc. Phase 4b+)
```

**Rules:**
- Every public function is a pure function (output depends only on inputs) or a method on a small struct with explicit state (`Clock`).
- `core` returns `crate::error::Result<T>` (alias for `std::result::Result<T, CoreError>`).
- `Clock` is the only mutable state in `core`. It uses `AtomicU32`, not `Mutex`.
- No `String` allocations in hot paths if `&str` suffices. Parser works on `&str` slices.

---

### `braint-client` — The IPC client layer

**Responsibility:** Connect to daemon, send requests, receive responses. Framing, connection lifecycle. No CLI logic, no TUI rendering.

```
src/
├── lib.rs              # Re-export Client, framing
├── error.rs            # ClientError (Io, Serde, DaemonUnreachable, ...)
├── framing.rs          # Length-prefixed JSON (write_frame, read_frame)
└── client.rs           # Client struct: connect, send_request, close
```

**Rules:**
- `Client` owns the `LocalSocketStream`. Drop = close.
- `send` is generic over request/response types, bounded by `Serialize`/`DeserializeOwned`.
- Connection errors are mapped to `ClientError::DaemonUnreachable` with a helpful message.
- No `anyhow`. Structured errors so CLI can choose exit codes.

---

### `braint-daemon` — The adapter / runtime layer

**Responsibility:** Owns the async runtime, the socket, the SQLite connection, and the wiring between them. Business logic delegates to `core`; persistence delegates to `storage`.

```
src/
├── lib.rs              # Re-export public API for tests
├── main.rs             # Minimal: init tracing, build deps, call daemon::run()
├── error.rs            # DaemonError (Storage, Io, JsonRpc, ...)
├── config.rs           # DaemonConfig, socket path, db path (Phase 2+)
├── server/
│   ├── mod.rs          # run(listener, state) → accept loop
│   ├── connection.rs   # One connection: read frame → dispatch → write frame
│   └── state.rs        # Shared daemon state: Arc<RwLock<...>> or channels
├── handler/
│   ├── mod.rs          # Dispatch table: method name → handler fn
│   ├── ingest.rs       # handle_ingest: core::parse + storage::save
│   └── confirm.rs      # handle_confirm, handle_cancel (Phase 2)
├── storage/
│   ├── mod.rs          # Storage struct, public API
│   ├── connection.rs   # Connection management, WAL, pragmas
│   ├── migrations/
│   │   ├── mod.rs      # Migration runner
│   │   └── 0001_*.sql  # One file per migration
│   ├── entry.rs        # Entry CRUD: save, get, list, update
│   └── query.rs        # Query builders, filter application (Phase 4+)
└── plugin/
    ├── mod.rs          # Plugin manager (Phase 4a)
    ├── lifecycle.rs    # Spawn, crash detection, restart (Phase 4a)
    └── router.rs       # Verb routing to plugins (Phase 4a)
```

**Rules:**
- `main.rs` is < 50 lines. It builds `DaemonConfig`, opens `Storage`, creates `Handler`, and calls `server::run`.
- `server::connection` is the ONLY place that knows about length-prefixed framing. Handler functions receive deserialized request structs.
- `handler::mod.rs` is a pure dispatch table. No business logic — just "call this handler with these args."
- `storage` is the ONLY place that knows about `rusqlite`. No SQL outside `storage/`.
- Shared state (for Phase 2+ subscriptions, pending confirmations) lives in `server::state` and is `Arc<tokio::sync::RwLock<…>>` or `tokio::sync::mpsc` channels. Never `std::sync::Mutex` in async code.
- Every write to storage goes through a single path so WAL mode assumptions hold.

---

### `braint-cli` — The user-facing binary

**Responsibility:** Parse CLI args, invoke client, print output, launch TUI. No socket logic directly — delegates to `client`.

```
src/
├── main.rs             # Arg parsing, dispatch to command handlers, TUI gate
├── error.rs            # CliError (Daemon, Io, ...)
├── args.rs             # Clap derive structs (Cli, Command, IngestArgs, ...)
├── commands/
│   ├── mod.rs          # dispatch(cmd) → Result<()>
│   ├── ingest.rs       # ingest command handler
│   ├── confirm.rs      # confirm/cancel (Phase 2)
│   └── list.rs         # list tasks, notes, etc. (Phase 4+)
├── output.rs           # Human formatter vs NDJSON formatter
└── tui/
    ├── mod.rs          # TUI bootstrap, panic hook, event loop
    ├── app.rs          # App state: panels, focus, mode
    ├── panels/         # One file per panel
    │   ├── scratch.rs
    │   ├── today.rs
    │   └── search.rs
    └── widgets/        # Reusable ratatui components
        └── header.rs
```

**Rules:**
- `main.rs` decides "CLI mode or TUI mode?" then delegates. No logic after the branch.
- `commands/` handlers return `Result<()>` and print via `output.rs`.
- `output.rs` owns the `--json` flag. All command handlers return data; `output.rs` decides how to render it.
- TUI is a separate module tree. It uses `client` to talk to the daemon, same as CLI commands.
- TUI cleanup is bulletproof: `panic_hook` restores terminal on panic; `Drop` impl on `App` cleans up on normal exit.

---

### `braint-plugin-sdk` — The plugin author layer

**Responsibility:** Everything a plugin author needs. Subprocess plumbing, manifest generation, attribute macros.

```
src/
├── lib.rs              # Re-export all public symbols
├── error.rs            # PluginSdkError
├── transport.rs        # JSON-RPC over stdio framing (same as client framing)
├── manifest.rs         # PluginManifest type, manifest generation helpers
├── router.rs           # Route incoming JSON-RPC to user-defined handlers
└── macro.rs            # #[verb], #[on_event] proc-macro stubs (Phase 4a)
```

**Rules:**
- Shares framing logic with `client` (copy the code, don't depend on `client` — plugins shouldn't depend on client crate).
- Manifest is generated at compile time from attribute macros.
- `--manifest` flag handling is built-in: if argv contains `--manifest`, print manifest and exit before user code runs.

---

## Error Handling Strategy

### Library crates (`proto`, `core`, `client`, `plugin-sdk`, `daemon` as lib)

Use `thiserror`:

```rust
// crates/braint-core/src/error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("parse error: {0}")]
    Parse(String),
    #[error("invalid verb: {0}")]
    InvalidVerb(String),
}

pub type Result<T> = std::result::Result<T, CoreError>;
```

- Every error variant carries context.
- No `.unwrap()` or `.expect()` in library code except for truly impossible invariants (documented with `// SAFETY:`).

### Binary crates (`daemon` bin, `cli` bin)

Use `anyhow` at the top level for ergonomics, but convert library errors explicitly:

```rust
fn main() -> anyhow::Result<()> {
    let entry = core::parse_ingest("...").map_err(|e| anyhow::anyhow!("failed to parse: {e}"))?;
    // ...
}
```

### The `daemon` crate is both lib and bin

- `src/lib.rs` exports `error::DaemonError` (thiserror) for tests and other crates.
- `src/main.rs` uses `anyhow` and maps `DaemonError` at the boundary.

---

## Async Boundaries

| Layer | Async? | Rule |
|-------|--------|------|
| `proto` | No | Types only. |
| `core` | No | Pure functions. `Clock` uses `AtomicU32`, not async locks. |
| `storage` | No | `rusqlite::Connection` is sync. Called from async tasks via `tokio::task::spawn_blocking` if the query is slow (Phase 4+). Phase 1: direct call is fine. |
| `client` | Yes | `tokio::io::AsyncWriteExt`, `AsyncReadExt`. |
| `daemon::server` | Yes | `tokio::net`, `tokio::signal`. |
| `daemon::handler` | No | Receives deserialized structs, returns result. Called from async context but is sync itself. |
| `daemon::storage` | No | Same as `storage` module. |
| `cli::commands` | Yes | `tokio::main`, async client calls. |
| `cli::tui` | Yes | `tokio::select!` on crossterm events + client notifications. |

---

## Concurrency Model

`core` is stateless. Functions like `parse_ingest` take inputs and return new values — no shared mutable state, no locks needed. Multiple daemon tasks can call `core` in parallel with zero contention.

`Clock` is the only mutable struct in `core`. It uses `AtomicU32`, so `clock.now()` is lock-free and safe across threads.

Shared mutable resources (`rusqlite::Connection`, subscription state, pending confirmations) live in `daemon`, not `core`. `daemon` serializes access to them:

- **Phase 1:** One connection at a time, sequential accept loop — no contention.
- **Phase 2+:** One task per connection. `Storage` is wrapped in `tokio::sync::Mutex` or moved to a dedicated writer task so SQLite writes never race.

CPU-bound work (parsing, filtering, HLC generation) runs in parallel. I/O-bound work (SQLite, sockets) is managed by `tokio` with explicit synchronization only where needed.

---

## Testing Strategy

### Unit tests

Live in the same file, under `#[cfg(test)] mod tests`. Test the public API of the module.

```rust
// crates/braint-core/src/parse.rs
pub fn parse_ingest(text: &str, ...) -> Entry { ... }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_idea() {
        let entry = parse_ingest("hello", ...);
        assert_eq!(entry.kind, EntryKind::Idea);
        assert_eq!(entry.body, "hello");
    }
}
```

### Integration tests

Live in `tests/`. Use the `common` module for setup helpers.

```rust
// crates/braint-daemon/tests/common/mod.rs
pub async fn spawn_test_daemon() -> TestHandle { ... }

// crates/braint-daemon/tests/ingest_e2e.rs
use common::spawn_test_daemon;

#[tokio::test]
async fn ingest_creates_row() {
    let handle = spawn_test_daemon().await;
    // ...
}
```

### Test data: builders, not fixtures

Prefer builder functions over static JSON files:

```rust
// tests/common/builders.rs
pub fn an_entry() -> EntryBuilder { ... }

// In test:
let entry = an_entry().with_body("explore CRDTs").build();
```

---

## Naming Conventions

| Thing | Rule | Example |
|-------|------|---------|
| Crate | `braint-{name}` | `braint-daemon`, `braint-cli` |
| Binary | `braintd` (daemon), `braint` (cli) | — |
| Module file | `snake_case.rs` | `entry.rs`, `jsonrpc.rs` |
| Struct | `PascalCase` | `JsonRpcRequest`, `HybridLogicalClock` |
| Function | `snake_case`, verb-first | `parse_ingest`, `save_entry`, `handle_confirm` |
| Error enum | `{Crate}Error` | `CoreError`, `DaemonError` |
| Result alias | `Result<T>` | `pub type Result<T> = std::result::Result<T, CoreError>;` |
| Newtype | `PascalCase`, wraps in tuple | `EntryId(Uuid)`, `DeviceId(Uuid)` |
| Constants | `SCREAMING_SNAKE_CASE` | `METHOD_INGEST`, `DEFAULT_TTL_SECS` |
| JSON-RPC methods | `snake_case` | `ingest`, `confirm`, `tasks.list` |

---

## Performance Guidelines

1. **Borrow over clone.** Functions take `&str` until ownership is truly needed. `Entry::body` is `String` because it must own; but `parse_ingest` takes `&str`.
2. **No allocation in framing.** `framing::read_frame` returns `Vec<u8>` because it must; but the parser should work on `&str` without allocating.
3. **SQLite: prepare once, execute many.** `Storage` holds prepared statements for hot paths (Phase 4+).
4. **TUI: no SQL queries on render.** Panels render from local state. Updates come from notifications.
5. **Streaming: bounded channels.** `tokio::sync::mpsc::channel(1024)` for most. Unbounded only for the action log.

---

## Documentation Rules

- Every public type and function has a doc comment.
- Module-level docs (`//!`) explain the module's responsibility.
- `// SAFETY:` for every `unsafe` block (there should be very few).
- `// TODO(phase-N):` for deferred work, with the phase number.
- `// NOTE:` for non-obvious invariants that future readers need.

---

## Lessons from Phase 1

Real gotchas that surfaced during implementation. Update this as new phases teach new things.

### `interprocess` tokio API

- Import from `interprocess::local_socket::tokio::prelude::*`, not direct module paths.
- `LocalSocketStream::connect` takes a `Name` from `path.to_fs_name::<GenericFilePath>()`, not a raw `&str`.
- `LocalSocketListener::accept()` returns just a `Stream` (not a tuple).
- Creating a listener requires the `ListenerOptions` builder: `ListenerOptions::new().name(name).create_tokio()`.

### `rusqlite` parameter binding

- The `params!` macro creates temporary values. Returning `impl Params` from a function fails because the temporaries are dropped before the caller uses them.
- **Fix:** Return a fixed-size array `[rusqlite::types::Value; N]` instead. It owns its data and implements `Params`.

### Workspace dependency hygiene

- Every crate that uses a proc-macro crate (e.g., `thiserror`) must declare it as a **direct** dependency. Transitive deps do not enable derive macros.
- `tracing-subscriber` needs `features = ["env-filter"]` explicitly; it is not on by default.

### Lifetime gotchas with `to_string_lossy()`

- `path.to_string_lossy()` returns a `Cow<str>` that borrows from `path`. Calling `.to_fs_name()` on it fails because the `Cow` is temporary.
- **Fix:** `let s = path.to_string_lossy().to_string();` then `s.to_fs_name::<GenericFilePath>()`.

### UDS path limits

- Linux/macOS UDS paths are limited to ~104–108 bytes. `tempfile::tempdir()` paths can be long.
- **Fix:** Use short prefixes (`tempfile::Builder::new().prefix("b")`) and short socket names (`s.sock`).

### `thiserror` derive requires direct dep

- `#[derive(Error)]` and `#[from]` only work if the crate using them directly depends on `thiserror`. A transitive dep through another workspace crate is not enough.

---

## Technical Debt Register

Debt items discovered during implementation, scheduled for cleanup. Update phase numbers when they land.

| # | Debt | Phase to fix | Reason | Current workaround |
|---|------|-------------|--------|-------------------|
| 1 | **Framing refactor** | 2 or 4a | Manual `read_frame`/`write_frame` is 20 lines but lacks buffering, backpressure, and stream combinators. `tokio-util::codec::LengthDelimitedCodec` or `rmcp`'s stdio transport (for plugin IPC) may replace it. | Manual framing works fine for one request/response. |
| 2 | **Eliminate `unwrap`/`expect`** | 2 | Production code has `unwrap()` in `main.rs` (temp dir) and test helpers. Phase 2 adds config loading, verb parsing, and confirmation flows — all new failure modes that need proper handling. | Acceptable for Phase 1 skeleton; will be audited in Phase 2. |
| 3 | **Structured error taxonomy** | 2 | Daemon errors are generic strings. CLI can't distinguish "disk full" from "bad parse." Need JSON-RPC error code ranges mapped to daemon error variants. | All errors map to `-32000` / `-32001` for now. |
| 4 | **Socket graceful shutdown** | 2 | Daemon deletes stale socket on startup but doesn't catch `SIGTERM`/`SIGINT` to clean up on exit. | `remove_file` on startup handles dev-loop crashes. |
| 5 | **Connection concurrency** | 2 | Phase 1 handles one connection at a time. Phase 2 TUI + CLI simultaneous use requires per-connection tasks + `Arc<Mutex<Storage>>`. | Sequential `accept()` loop. |

---

## Versioning & Evolution

- These conventions apply from Phase 1 onward.
- When a new pattern is needed (e.g., a new crate, a new layer), propose it here first, then implement.
- Breaking a convention is allowed if the reason is documented in a `// NOTE:` or a brief ADR in `docs/adr/`.
