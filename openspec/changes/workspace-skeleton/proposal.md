# Proposal: Workspace Skeleton

## Intent
Convert the bare `braint` binary into a proper Cargo workspace with all crates defined, CI configured, and build passing. This is Phase 0 of the build plan.

## Scope
- Workspace with crates: proto, core, storage, client, daemon, cli, plugin-sdk, xtask
- Workspace dependencies pinned in root Cargo.toml
- rust-toolchain.toml, rustfmt.toml, clippy.toml
- GitHub Actions CI (Linux + macOS)
- Minimal README with elevator pitch
- cargo build && cargo test passing (zero tests)

## Approach
Follow the phase-details.md Phase 0 steps exactly. Mechanical setup with no product behavior.

## Affected Areas
- Root Cargo.toml (replaced with workspace manifest)
- src/main.rs (deleted)
- New crates/ directory with 8 crates
- .github/workflows/ci.yml
- README.md

## Rollback Plan
Delete crates/ directory, restore original Cargo.toml and src/main.rs.
