# braint — Personal Daemon

> **Status: pre-alpha** — early development, not yet usable.

A background daemon that lives on your machines, captures what you tell it — by voice, CLI, or widget — runs tasks on your behalf, and shows glanceable info through configurable widgets. Everything is a plugin. Everything you capture and every action it takes is stored, tagged, and searchable. Offline-first, single user, syncs across your own devices, open source.

Think of it as a personal cognitive assistant that sits between you and your computer — closer to a system service than an app.

## Quick Start

Not ready yet. This repository is tracking the 0.1 MVP build.

## Architecture

- **Rust** workspace with crates: `proto`, `core`, `storage`, `client`, `daemon`, `cli`, `plugin-sdk`
- **Daemon** (`braintd`) — long-running background process
- **CLI** (`braint`) — dual-mode: one-shot commands or TUI inspector
- **Plugins** — subprocesses speaking JSON-RPC over stdio
- **Storage** — SQLite for metadata, markdown files for long-form content

## License

MIT/Apache-2.0 dual license (to be added before release).
