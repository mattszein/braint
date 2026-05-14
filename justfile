# Personal Daemon — Just commands

# Default: show available commands
_default:
    @just --list

# Run all quality checks (fmt, clippy, test)
check:
    cargo fmt --all -- --check
    cargo clippy --workspace -- -D warnings
    cargo test --workspace

# Build the workspace
build:
    cargo build --workspace

# Run tests
test:
    cargo test --workspace

# Run the daemon in debug mode (placeholder for when daemon is implemented)
dev:
    cargo run --bin braintd

# Run the CLI/TUI (placeholder for when CLI is implemented)
cli:
    cargo run --bin braint

# Clean build artifacts
clean:
    cargo clean
