# Spec: Workspace Skeleton

## Requirements

### R1 — Workspace Structure
**Given** a fresh clone of the repo  
**When** `cargo build` is run  
**Then** all workspace members compile successfully with zero errors

### R2 — Crate Graph (6 crates)
**Given** the workspace members  
**Then** the dependency graph MUST be acyclic:
- `proto` depends on nothing
- `core` depends on `proto`
- `client` depends on `proto`
- `plugin-sdk` depends on `proto`
- `daemon` depends on `proto`, `core`, `client` (and contains `storage` as a module)
- `cli` depends on `proto`, `client`, `core`

### R3 — Shared Dependencies
**Given** the root Cargo.toml  
**Then** `[workspace.dependencies]` MUST pin versions of: tokio, serde, serde_json, rusqlite, interprocess, notify, clap, ratatui, crossterm, anyhow, thiserror, uuid, tracing

### R4 — Toolchain
**Given** rust-toolchain.toml  
**Then** it MUST pin a stable Rust version for reproducibility

### R5 — Code Quality
**Given** any code change  
**When** CI runs  
**Then** `cargo clippy -- -D warnings` and `cargo fmt --check` MUST pass on Linux and macOS

### R6 — justfile
**Given** a `justfile` in the repo root  
**Then** common commands (`just check`, `just build`, `just test`) MUST work

## Scenarios

### S1 — Clean Build
```
$ cargo build
   Compiling braint-proto v0.1.0
   Compiling braint-core v0.1.0
   ...
    Finished `dev` profile [unoptimized + debuginfo] target(s) in Xs
```

### S2 — CI Green
A push to any branch triggers CI. All matrix jobs (Linux stable, macOS stable) pass.

### S3 — Zero Tests Pass
```
$ cargo test
    Finished `test` profile [unoptimized + debuginfo] target(s) in Xs
     Running unittests ...
   result: ok. 0 tests passed; 0 failed
```
