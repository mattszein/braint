# Design: Workspace Skeleton

## Architecture Decisions

### AD1 — Workspace Layout
We use `crates/` subdirectory instead of flat workspace members. Keeps root clean and scales to 10+ crates.

### AD2 — Crate Prefix
All internal crates use `braint-` prefix to avoid name collisions on crates.io if published.

### AD3 — Edition 2024
The project started with edition 2024. We keep it. All member crates inherit edition from workspace.

### AD4 — Empty Crates
Phase 0 crates compile but contain only stub `lib.rs` / `main.rs`. Real code lands in subsequent phases.

### AD5 — Storage as Module (not Crate)
Storage starts as `daemon/src/storage/` module. It is SQLite-only, daemon-adjacent, and not reused by other crates in Phase 0–1. If it later needs independence (shared by cli, used in tests without daemon binary), promote to crate — the move is mechanical in Rust.

### AD6 — justfile over xtask
A `justfile` covers common commands (`just check`, `just dev`) without an extra crate. Migrate to `xtask` only if build automation grows complex enough to justify it.

## File Layout

```
braint/
├── Cargo.toml                 # Workspace manifest
├── rust-toolchain.toml
├── rustfmt.toml
├── clippy.toml
├── justfile                   # Common commands
├── .github/
│   └── workflows/
│       └── ci.yml
├── crates/
│   ├── proto/
│   │   └── Cargo.toml
│   │   └── src/lib.rs
│   ├── core/
│   │   └── Cargo.toml
│   │   └── src/lib.rs
│   ├── client/
│   │   └── Cargo.toml
│   │   └── src/lib.rs
│   ├── daemon/
│   │   └── Cargo.toml
│   │   └── src/main.rs
│   │   └── src/lib.rs        # exposes storage module
│   │   └── src/storage/
│   │       └── mod.rs        # SQLite persistence
│   ├── cli/
│   │   └── Cargo.toml
│   │   └── src/main.rs
│   └── plugin-sdk/
│       └── Cargo.toml
│       └── src/lib.rs
└── README.md
```

## Dependency Graph

```
proto ← core
  ↑           
  └────── client
              ↑
         daemon (depends on proto, core, client; contains storage module)
              ↑
  cli ────────┘ (depends on proto, client, core)

plugin-sdk → proto
```

## CI Matrix

| OS | Rust | Steps |
|----|------|-------|
| ubuntu-latest | stable | build, test, clippy, fmt-check |
| macos-latest | stable | build, test, clippy, fmt-check |

## Release Profile

```toml
[profile.release]
panic = "abort"
lto = "thin"
codegen-units = 1
```
