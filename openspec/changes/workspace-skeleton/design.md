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

## File Layout

```
braint/
├── Cargo.toml                 # Workspace manifest
├── rust-toolchain.toml
├── rustfmt.toml
├── clippy.toml
├── .gitignore
├── .cargo/
│   └── config.toml            # xtask alias
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
│   ├── storage/
│   │   └── Cargo.toml
│   │   └── src/lib.rs
│   ├── client/
│   │   └── Cargo.toml
│   │   └── src/lib.rs
│   ├── daemon/
│   │   └── Cargo.toml
│   │   └── src/main.rs
│   ├── cli/
│   │   └── Cargo.toml
│   │   └── src/main.rs
│   ├── plugin-sdk/
│   │   └── Cargo.toml
│   │   └── src/lib.rs
│   └── xtask/
│       └── Cargo.toml
│       └── src/main.rs
└── README.md
```

## Dependency Graph

```
proto ← core ← storage
  ↑           ↑
  └────── client
              ↑
         daemon (depends on all)
              ↑
  cli ────────┘ (depends on proto, client, core)

plugin-sdk → proto
xtask → (none)
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
