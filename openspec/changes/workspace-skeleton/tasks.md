# Tasks: Workspace Skeleton

## Phase 0 — Workspace Skeleton

### Infrastructure

- [ ] 1.1 Convert root to workspace manifest
  - Delete `src/`
  - Replace root `Cargo.toml` with workspace manifest
  - Add `[workspace.dependencies]` with pinned versions

- [ ] 1.2 Create empty crates
  - `cargo new --lib crates/braint-proto`
  - `cargo new --lib crates/braint-core`
  - `cargo new --lib crates/braint-storage`
  - `cargo new --lib crates/braint-client`
  - `cargo new --bin crates/braint-daemon`
  - `cargo new --bin crates/braint-cli`
  - `cargo new --lib crates/braint-plugin-sdk`
  - `cargo new --bin crates/xtask`

- [ ] 1.3 Wire crate dependencies
  - Each crate's `Cargo.toml` references workspace deps
  - Set up inter-crate dependencies per design

- [ ] 1.4 Add toolchain and quality configs
  - `rust-toolchain.toml`
  - `rustfmt.toml`
  - `clippy.toml`
  - `.cargo/config.toml` (xtask alias)

- [ ] 1.5 Configure CI
  - `.github/workflows/ci.yml`
  - Linux + macOS matrix
  - build, test, clippy, fmt-check

- [ ] 1.6 Add README and gitignore
  - One-paragraph elevator pitch
  - "Status: pre-alpha" banner
  - `.gitignore` updated

- [ ] 1.7 Verify clean build
  - `cargo build` succeeds
  - `cargo test` passes (0 tests)
  - `cargo clippy -- -D warnings` clean
  - `cargo fmt --check` clean

- [ ] 1.8 Commit and push
  - All files staged
  - CI runs and passes
