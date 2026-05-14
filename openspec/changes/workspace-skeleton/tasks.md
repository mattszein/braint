# Tasks: Workspace Skeleton

## Phase 0 — Workspace Skeleton

### Infrastructure

- [x] 1.1 Convert root to workspace manifest
  - Delete `src/`
  - Replace root `Cargo.toml` with workspace manifest
  - Add `[workspace.dependencies]` with pinned versions

- [x] 1.2 Create empty crates (6 crates)
  - `cargo new --lib crates/braint-proto`
  - `cargo new --lib crates/braint-core`
  - `cargo new --lib crates/braint-client`
  - `cargo new --lib crates/braint-plugin-sdk`
  - `cargo new --bin crates/braint-daemon`
  - `cargo new --bin crates/braint-cli`
  - ~~`cargo new --lib crates/braint-storage`~~ → REMOVED: storage lives as module inside daemon
  - ~~`cargo new --bin crates/xtask`~~ → REMOVED: replaced by justfile

- [x] 1.3 Wire crate dependencies
  - Each crate's `Cargo.toml` references workspace deps
  - Set up inter-crate dependencies per design (6-crate graph)

- [x] 1.4 Add toolchain and quality configs
  - `rust-toolchain.toml`
  - `rustfmt.toml`
  - `clippy.toml`
  - ~~`.cargo/config.toml` (xtask alias)~~ → REMOVED: xtask not used

- [x] 1.5 Add justfile
  - `just check` runs fmt + clippy + test
  - `just build`, `just test`, `just dev`, `just cli`

- [x] 1.6 Configure CI
  - `.github/workflows/ci.yml`
  - Linux + macOS matrix
  - build, test, clippy, fmt-check

- [x] 1.7 Add README and gitignore
  - One-paragraph elevator pitch
  - "Status: pre-alpha" banner
  - `.gitignore` updated

- [x] 1.8 Create daemon storage module placeholder
  - `crates/braint-daemon/src/lib.rs` exposes `pub mod storage`
  - `crates/braint-daemon/src/storage/mod.rs` placeholder

- [x] 1.9 Verify clean build
  - `cargo build` succeeds
  - `cargo test` passes (0 tests)
  - `cargo clippy -- -D warnings` clean
  - `cargo fmt --check` clean

- [x] 1.10 Commit and push
  - All files staged
  - CI runs and passes
