# Personal Daemon — Phase Details

A companion to the build plan. For each phase: pre-flight checks, step-by-step work order, architecture notes, gotchas, tips, and definition of done. No code — just the *how* underneath the *what* in the build plan.

Read the phase before starting it. Reference it while working. Update it if you learn something the future-you would want to know.

---

## Phase 0 — Workspace skeleton

**Pre-flight check**
- Rust toolchain installed (`rustup`, stable channel pinned in `rust-toolchain.toml` for reproducibility).
- Project name chosen. This locks crate prefixes, socket names, config paths, repo name.
- GitHub repo created (private or public, your call).
- Decided on license: MIT/Apache-2.0 dual is the Rust standard.

**Steps in order**
1. `cargo new --bin <name>` at the repo root, then convert to a workspace: delete the auto-generated `src/`, replace `Cargo.toml` with a workspace manifest.
2. Create the empty crates: `cargo new --lib crates/proto`, `--lib crates/core`, `--lib crates/client`, `--lib crates/plugin-sdk`, `--bin crates/daemon`, `--bin crates/cli`. (Storage starts as a module inside `daemon` — promote to its own crate later if it earns it. Skip `xtask` for now — a `justfile` covers the simple cases without an extra crate.)
3. Populate the workspace `Cargo.toml`: list members, declare `[workspace.dependencies]` with pinned versions of the shared crates.
4. Add `rust-toolchain.toml` pinning the Rust version.
5. Add `.gitignore` (target/, .env, .DS_Store).
6. Add `rustfmt.toml` and `clippy.toml` with project conventions (e.g. `edition = "2021"`, max width 100).
7. Add `.github/workflows/ci.yml` running `cargo build`, `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check` on Linux and macOS matrices.
8. Add minimal `README.md` with a one-paragraph elevator pitch and a "status: pre-alpha" banner.
9. Verify `cargo build` succeeds from a clean clone. Push.

**Architecture notes**
- Workspace dependencies in `[workspace.dependencies]` mean each member crate references the version with `dep.workspace = true`. One place to bump versions.
- Use a `justfile` for common command shortcuts (`just dev`, `just check`, `just release`). Simple, requires only the `just` tool, no extra crate. Migrate to an `xtask` binary later *only* if you find yourself wanting to share code between the workspace and your build automation (cross-platform packaging, regenerating files from workspace types, etc.).
- Keep the crate graph acyclic: `proto` depends on nothing, `core` depends on `proto`, `daemon` depends on `proto`/`core` (and contains the storage module), `cli` depends on `proto`/`client`/`core`, `client` depends on `proto`, `plugin-sdk` depends on `proto`/`core`. Don't let anything reach upward.

**Gotchas**
- Don't put `tokio` in `core`. The temptation is real because you'll want async APIs everywhere. Resist. `core` stays sync and pure; `daemon` adapts it to async.
- Setting up CI for macOS now (even with one test) catches platform-specific issues before they pile up.
- Don't commit `Cargo.lock` to `.gitignore`. For a binary project (which this is), commit the lock file — reproducible builds matter.

**Tips**
- Use `cargo-nextest` for test runs; faster and clearer output than `cargo test`.
- Set `panic = "abort"` in release profile to shrink the binary. Set `lto = "thin"` and `codegen-units = 1` for the smallest, fastest release builds.
- A `justfile` for common invocations: `just dev` runs the daemon with debug logs, `just check` runs fmt + clippy + test, `just demo <phase>` runs the smoke test for a phase.

**Definition of done**
- `cargo build` succeeds. `cargo test` passes (zero tests is fine). CI is green on Linux + macOS. Repo has README, license, .gitignore, toolchain file, format/lint configs.

---

## Phase 1 — Skeleton end-to-end

**Pre-flight check**
- Phase 0 complete.
- Decided: the local socket path convention (use `directories` crate's `runtime_dir()` on Linux/macOS; fall back to a temp path; named pipe on Windows).

**Steps in order**
1. **`proto` first.** Define the core wire types: `EntryId` (UUIDv7 wrapper), `Entry`, `EntryKind` (just `Idea` for now), `HybridLogicalClock`, `DeviceId`. All `serde`-derived. Constants for JSON-RPC method names.
2. **Define the first JSON-RPC method**: `ingest`. Request: `{ text: String, source: Source }`. Response: `{ entry_id: EntryId }`. Add `JsonRpcRequest`/`JsonRpcResponse`/`JsonRpcError` envelope types (matches the JSON-RPC 2.0 spec).
3. **`core`**: write `parse_ingest(text: &str) -> Entry`. For now: returns `Entry { kind: Idea, body: text, … }`. Implement UUIDv7 generation, initial HLC, device ID loading from config or generating a fresh one.
4. **`daemon::storage` module**: SQLite migration 0001 that creates `entries` (id, kind, body, created_at_hlc, created_on_device, last_modified_at_hlc, last_modified_on_device, …). Implement `save(entry)` and `get(id)`. Use `rusqlite` with `bundled`. Enable WAL mode. Lives in `daemon/src/storage/` as a module; promote to its own crate later only if it earns it.
5. **`client`**: connect to local socket via `interprocess`, send a JSON-RPC request, await response. One public function: `send_request(req) -> Result<Response>`.
6. **`daemon`**: open the local socket, accept connections, spawn a tokio task per connection, read JSON-RPC, dispatch `ingest` to a handler that calls `core::parse_ingest` and `storage::save`, send response.
7. **`cli`**: `clap` for arg parsing. One subcommand: `ingest <text>`. Wire it to `client::send_request`. Print the entry ID on success; exit nonzero with a friendly message on daemon-unreachable.
8. **Integration test**: spin up the daemon in a tokio test, send an ingest, assert the row appears in SQLite. This is your first end-to-end test — invest in making it clean because every later phase will copy this pattern.

**Architecture notes**
- The JSON-RPC envelope handling lives in `client` (and a mirror in `daemon`). Don't duplicate the envelope types — define them in `proto` and import.
- The handler in `daemon` is a sync function from request → response. The async I/O is the socket; the business logic is sync. This keeps testability high.
- Migrations: store them as `.sql` files in `crates/daemon/src/storage/migrations/` (since storage starts as a module inside daemon). Use `refinery` or `rusqlite_migration` — both work; the former has a more mature feature set, the latter is lighter.
- HLC implementation: a tuple `(u64 physical_ms, u32 logical, DeviceId)`. Update rules: on each write, `new = max(current_hlc + (0, 1, _), (now_ms, 0, device))`. Total ordering by lexicographic compare.

**Gotchas**
- `interprocess` socket cleanup: on Linux, if the daemon crashes, the UDS file stays around and the next start fails with `EADDRINUSE`. Catch this and `unlink` first, then `bind`. Make this graceful — you'll restart the daemon hundreds of times during dev.
- Don't use `Uuid::new_v4()` anywhere. Force UUIDv7 from day one (use the `uuid` crate's `v7` feature). v4 ids in early data will haunt you when you try to sort or sync.
- WAL mode requires a writable directory (it creates `-wal` and `-shm` sidecar files). If you point the daemon at a read-only path during a test, you'll get a confusing error.
- `serde_json` is happy to parse partial JSON. The wire format needs framing — settle on **length-prefixed JSON** (4-byte big-endian length, then JSON bytes). Don't try to make raw newline-delimited JSON work over a streaming socket; you'll regret it.

**Tips**
- Use `tracing` from the start. Initialize a subscriber in `daemon::main` that respects `RUST_LOG`. Add `tracing::instrument` to handler functions. You'll thank yourself in phase 4a.
- Keep the daemon's main loop tiny: accept → dispatch → respond. All real logic in handler modules.
- Write the integration test as a library helper (`tests/common/mod.rs`) — `spawn_test_daemon() -> TestDaemonHandle`. Reused in every phase.
- For the CLI: use `clap`'s derive macros, not the builder API. Less code, better help output.

**Definition of done**
- `<name>` daemon runs in one terminal. `<name>-cli ingest "hello"` in another returns an ID. A row exists in SQLite at the configured path with HLC and device fields populated. Integration test passes in CI on Linux and macOS.

---

## Phase 2 — Verb grammar

**Pre-flight check**
- Phase 1 complete. Integration test infrastructure in place.
- Decided: the initial verb vocabulary (`idea`, `todo`, `note`, `capture` — start small).
- Decided: the principal-tag prefix list (`project:`, `type:`, `status:`, `priority:`, `when:`, `due:`, `scope:`, `repeat:`).

**Steps in order**
1. **Grammar design first, code second.** On paper or in a markdown file, enumerate ~20 example utterances per verb, the expected parse, edge cases ("idea pro-rails" — is pro-rails the project or part of the body?). Resolve ambiguities. Update the verb table in `core` with this resolved grammar.
2. **`core::parse_verb`**: takes a transcript string, returns a parsed `VerbInvocation { verb, project, principal_tags, free_tags, body }`. Write the test suite *first* (table-driven, one row per example utterance). Make the parser pass.
3. **Extend `proto`**: `Entry` now has `project`, `principal_tags`, `free_tags`. JSON-RPC methods: `ingest` parameters expand to support source flag. New method `confirm(pending_id)`. New method `cancel(pending_id)`.
4. **Streaming subscriptions in `proto`**: define `Subscribe { topic, filter }` request, `Notification { topic, payload }` server-pushed message. The server pushes notifications without a request ID, matching JSON-RPC 2.0 notification semantics.
5. **`daemon`**: handler for `ingest` now calls `core::parse_verb`. If `source = voice`, store the parsed entry in an in-memory `pending` map and return a `Confirmation { pending_id }` response instead of committing. `confirm` commits; `cancel` discards. Set a TTL on pending entries (60s) to avoid leaks.
6. **`daemon`**: subscription manager. Each subscription holds a filter (kind, project, tags). When any entry is created or updated, evaluate it against all open subscriptions; push notifications to matching subscribers.
7. **`cli` subcommands**: `idea <body>`, `todo <body>`, `note <body>`, `capture <body>`. Each is a thin alias that prepends the verb. Add `<name>-cli confirm <id>` and `<name>-cli cancel <id>` for the wrapper script use case.
8. **`cli` dispatch with TTY detection**: when invoked with no arguments, branch on stdin:
   - stdin is a TTY → launch the TUI.
   - stdin is piped → treat input as `ingest` (read stdin, pass as body). This lets `echo "idea — try crdt" | <name>-cli` do the right thing.
   - Explicit `<name>-cli tui` subcommand always launches the TUI regardless of stdin.
9. **`cli` TUI bootstrap**: initial layout: header (project name placeholder, focus indicator placeholder), two panels (Scratch, Recent activity), bottom command line. Start with stub data; wire to real subscriptions next.
10. **TUI wiring**: open two subscriptions on TUI startup — one for "all entries in scratch" (no project), one for "all entries created in the last hour" (recent activity). Each panel renders its subscription's current state and updates on every notification.
11. **TUI command line**: `:`-mode opens an input field. Enter sends the input through the same `ingest` path the CLI uses (source = cli). `Esc` cancels. `q` quits the TUI.

**Architecture notes**
- The parser is pure. Tests don't touch the daemon. This is what "domain pure" looks like in practice.
- `parse_verb` returns a `Result<VerbInvocation, ParseError>`. Errors come back as JSON-RPC errors to the caller. Never panic on bad input.
- Subscription filters are *evaluated server-side*. The client doesn't see entries that don't match. This matters for performance later when there are millions of action-log entries.
- The TUI uses `ratatui` with `crossterm` as the backend. Architecture: a single event loop polling both keyboard input (`crossterm::event::poll`) and subscription notifications (a `tokio::sync::mpsc` channel from the IPC client). Render on every tick.
- Keep TUI state in a single `App` struct. Each panel is a method on `App`. No global state, no `Rc<RefCell<…>>` mess.

**Gotchas**
- The "is `pro-rails` the project or part of the body" problem only resolves if the parser knows the current set of projects. For phase 2, simplify: `for <project>` is the only project syntax. `project:<id>` works in CLI but not voice. Voice users will say "idea for pro-rails." Defer "known-entity matching" until phase 6 (when projects actually exist as first-class state).
- TUI redraws on every key press *and* every notification. Make sure the render path is fast — no SQL queries, no IPC calls. The TUI's local state is the source of truth for what's drawn; updates come from notifications.
- Terminal raw mode and TUI cleanup: if your code panics inside the TUI, the terminal stays in raw mode (no input, no echo, broken). Wrap the TUI's main function in a panic hook that restores the terminal. `crossterm::terminal::disable_raw_mode()` + `LeaveAlternateScreen` on Drop.
- Confirmation flow latency: when source = voice, the user is waiting in front of the screen. Don't make confirmation feel like a round-trip — the daemon should respond in <50ms with the parsed preview.

**Tips**
- For `parse_verb`, **don't write a real parser library** (no `nom`, no `pest`). String split with smart whitespace handling and a switch on the first token is faster to write, faster to debug, and good enough for voice grammar.
- For the TUI, look at `gitui` and `atuin` as references for layout, keybinding, and modal flows. Both are open-source and well-designed.
- `ratatui` has a `TestBackend` — write golden tests of the TUI's rendered output. A snapshot of the rendered cells in a buffer is the assertion. Lets you iterate on layout without manual checking.
- Confirmation UX in TUI: a small modal overlay with the parsed entry preview and `[y]es/[n]o` keybindings. Fast, no prompt-loop awkwardness.

**Definition of done**
- The phase 2 demo (5 steps in the build plan) works end-to-end. The parser test suite has at least 30 cases covering all four verbs plus error cases. The TUI launches, renders, updates live, and responds to `:`-commands. Confirmation flow round-trips for voice-sourced input.

---

## Phase 3 — Voice via Voxtype

**Pre-flight check**
- Phase 2 complete. Confirmation flow works.
- Voxtype installed on the dev machine. A model downloaded (start with `parakeet` — fast and good).
- Hyprland config understanding: you know where to put keybind definitions.

**Steps in order**
1. Pick the default integration pattern (post_process_command). Write a shell script `voxtype-to-daemon.sh` that reads stdin and calls `<name>-cli ingest --source voice "$(cat)"`. Drop it in `examples/voxtype-route-a/`.
2. Configure a Voxtype profile that uses this script as `post_process_command`. Document the profile in `examples/voxtype-route-a/voxtype-config.toml`.
3. Add a Hyprland keybind that triggers Voxtype's `record start --profile=daemon` (push-to-talk on a chord like `Super+V`). Document in `examples/voxtype-route-a/hyprland-keybinds.conf`.
4. Confirmation UX: simplest possible. The daemon emits a `Confirmation` response; the CLI's ingest path detects this and shells out to `notify-send` with action buttons (Confirm / Cancel). Action buttons map to `<name>-cli confirm <id>` and `<name>-cli cancel <id>`. Document this in the same examples dir.
5. End-to-end manual test: press the keybind, dictate "idea — try cr-sqlite for sync," release. Notification appears. Click Confirm. Open the TUI — the entry is there in scratch.
6. Document the troubleshooting steps for common failures (Voxtype model not loaded, keybind not firing, daemon not running).

**Architecture notes**
- The daemon doesn't know Voxtype exists. From its perspective, an ingest came in with `source: voice`. This separation is by design — swapping Voxtype for another tool changes the wrapper script, not the daemon.
- The notification-with-action pattern works on Linux out of the box via `notify-send`. On macOS the equivalent is harder (no built-in actionable notifications from CLI); the TUI confirmation modal is a fallback. Document both.
- Voxtype profiles map to keybinds. You can have multiple profiles with different post-processors — useful in phase 9 when Route B lands.

**Gotchas**
- Voxtype's `post_process_command` runs in a non-interactive shell. `$PATH` may be minimal. Use absolute paths to `<name>-cli` in the wrapper script, or set `PATH` explicitly.
- Wayland focus loss: when a notification pops up, the focused window doesn't change. So the user's typing context is preserved. This is good; don't try to "fix" it.
- If the notification daemon (Mako, Dunst, etc.) drops the action buttons silently, the user has no way to confirm. Detect this in docs and offer the TUI fallback.
- Notification timeout: if the user doesn't confirm within 60s, the pending entry is dropped. Make this explicit in the notification text ("Confirm within 60s").

**Tips**
- Test the Voxtype integration on a fresh terminal session (not the one you've been developing in) to catch `$PATH` issues.
- Use `pactl list short sources` to verify Voxtype is reading from the right mic.
- The shipped script should `set -euo pipefail` and log errors to a file in `$XDG_RUNTIME_DIR` so failures are debuggable post-mortem.
- For the example Hyprland config: use `bindd` (with description) so the keybind shows up in any keybinding inspector the user has.

**Definition of done**
- The full keybind → dictate → confirm → entry visible in TUI flow works on the dev machine. Documentation in `examples/voxtype-route-a/` is enough that a friend with Voxtype installed can reproduce it.

---

## Phase 4a — Plugin infrastructure

**Pre-flight check**
- Phases 0–3 complete.
- Decided: stdio JSON-RPC as the plugin transport (matches MCP convention, simpler than per-plugin sockets).
- Decided: where plugins live on disk (bundled directory `<install-prefix>/lib/<name>/plugins/` for system plugins; `~/.local/share/<name>/plugins/` for external).
- Decided: manifest delivery — plugin binary invoked with `--manifest` prints JSON to stdout. No parallel files.

**Steps in order**
1. **Manifest type in `proto`**: `PluginManifest { name, version, api_version, verbs, events_subscribed, kinds_owned }`. Stable across daemon versions; api_version field allows the daemon to refuse plugins it can't talk to.
2. **`plugin-sdk` crate**: helpers for plugin authors. Reads JSON-RPC requests from stdin, writes responses to stdout. Routes incoming verbs to user-defined handlers. Attribute macros `#[verb]`, `#[on_event]` that collect into a compile-time manifest. A `--manifest` short-circuit at startup: if argv includes `--manifest`, print the manifest as JSON and exit before anything else runs.
3. **`daemon` plugin manager**:
   - At startup, scan the bundled and user plugin directories.
   - For each binary, invoke with `--manifest` and parse the JSON output.
   - Validate: api_version compatible, no verb-name collisions, event topic prefixes match plugin name, no kind-ownership collisions. Refuse on any violation with a clear error.
   - For accepted plugins, spawn as subprocess (without `--manifest`) and start the JSON-RPC conversation.
   - Maintain `PluginHandle { manifest, child_handle, stdin_tx, stdout_rx }` per plugin.
4. **Verb routing through plugins**: when a parsed `VerbInvocation` has a verb owned by a plugin, the daemon forwards it over JSON-RPC, awaits `VerbResult`, commits to storage. The verb-routing path is unified — the verb router doesn't care if the verb is implemented in-daemon or by a plugin.
5. **Plugin lifecycle handling**:
   - Subprocess crash detection: read pipe closed → mark plugin dead, log error.
   - One automatic restart with exponential backoff (1s, then 5s; give up after second crash).
   - Graceful shutdown on daemon SIGTERM: send SIGTERM to children, wait 5 seconds, SIGKILL stragglers. Process-group handling on Linux (`setpgid`) so the daemon and its children form one group.
6. **Governance enforcement** in code: event topics must start with `<plugin_name>.`. Verb registration is rejected on duplicates. Kind ownership is unique.
7. **`plugins/hello` example plugin**: registers one verb `hello`, responds with `"hello {arg}"`. Lives in the workspace as the canonical reference and as the test subject.
8. **Test harness**: `tests/common/plugin.rs` helper that spawns a plugin, sends a JSON-RPC request, asserts on the response or on a crash. Deterministic — no `sleep`, use channel handshakes for "plugin is ready."
9. **CI: run plugin tests on Linux and macOS.** Cross-platform subprocess differences will bite; better to catch them now.

**Architecture notes**
- The plugin manager is the *only* place that knows about subprocesses. The rest of the daemon talks to plugins through the manager's API.
- Reads through `storage` are direct (no plugin involvement). Writes that come from a verb go through the plugin handler. Plugins implement behavior; reads are unrestricted.
- Plugin stderr is piped into the daemon's `tracing` subscriber so logs are unified. Each log line gets a `plugin=<name>` field.
- The `--manifest` flag pattern means a plugin's manifest is *whatever its binary prints* — impossible for the manifest to drift from the code.

**Gotchas**
- **Stdout buffering.** Plugin authors will accidentally `println!` for debug output, corrupting the JSON-RPC stream. The SDK should panic with a clear error message if anyone calls `println!`, and force `eprintln!` for debug. Document this loudly in the plugin author guide.
- **Zombie processes.** If the daemon crashes, plugin subprocesses can become orphans. The process-group setup ensures that when the daemon group dies, children die with it on Linux. macOS process group semantics are slightly different — test explicitly.
- **Closed-pipe blocking.** Reading from a closed pipe can block indefinitely on some platforms. Use `tokio::io::AsyncRead` with explicit timeouts. The deterministic test harness will catch this fast.
- **Manifest validation must be strict.** A plugin with a malformed manifest, or claiming an already-claimed verb, must fail loudly at load time — not at runtime when someone invokes that verb. Strict-mode loading.
- **Cross-platform signal handling.** SIGTERM on Linux/macOS, no SIGTERM equivalent on Windows. Either skip Windows in 4a or use `tokio::process::Child::kill()` (cross-platform but less graceful).

**Tips**
- Build the `hello` plugin and the plugin SDK together. They co-evolve. Don't try to spec the SDK perfectly first — iterate against `hello` as the smallest possible client.
- Look at `rmcp`'s stdio transport source for the JSON-RPC-over-stdio framing — solves length-prefix framing, async stdio, the same problem.
- The test harness is the most important deliverable of this phase. Every later plugin phase reuses it. Invest the time.

**Definition of done**
- `hello` plugin spawns, manifest validates, verb invocation round-trips correctly. Killing the plugin causes one automatic restart; killing twice causes the daemon to give up and log. Daemon shutdown cleans up all plugin children. CI passes on Linux and macOS.

---

## Phase 4b — Tasks plugin + TUI integration

**Pre-flight check**
- Phase 4a complete. The plugin infrastructure is proven by `hello`.
- The `tasks` plugin's verb shapes and state model are sketched on paper.

**Steps in order**
1. **`plugins/tasks` crate**: handles `todo`, `done`, `drop`, `block` verbs. State transitions are pure functions over `Entry`. Subtasks are markdown checkboxes in the body. Uses `plugin-sdk` attribute macros.
2. **`core`**: a `Task` view over `Entry` — given `Entry { kind: Todo, … }`, derive current state from the `status:` principal tag.
3. **`cli` subcommands**: `done <id>`, `drop <id>`, `block <id>`, `tasks list [--project ...] [--status ...] [--when ...]`. Reads go through normal storage queries (not the plugin). Mutations route through the plugin via the verb path.
4. **`cli` TUI**: add `Today` and `Tasks` panels. Subscribe to entries with `kind:Todo`. Keystrokes: `d` mark done, `D` drop, `b` block, `t` switch to tasks view, `j/k` navigate.
5. **End-to-end integration test**: CLI → daemon → tasks plugin → state change → SQLite update → subscription notification → TUI re-render. The whole pipe.

**Architecture notes**
- The `Task` view is read-only. It's a derivation from `Entry` + principal tags, not a separate type. This keeps the "entries are the source of truth" rule intact.
- The TUI Today panel's query is `kind=Todo AND status=open AND when=today`. Predicate filters happen daemon-side; the panel just renders results.

**Gotchas**
- Two TUI panels subscribing to overlapping entries (a task is in both Today and Tasks). The daemon should send one notification; the TUI's local state should know both panels need to re-render. Easiest path: each panel filters from a single shared task-state cache in the TUI.
- "Mark done" race: user presses `d` twice quickly. Make the keystroke handler idempotent — second press is a no-op if the task is already done.

**Tips**
- Copy phase 2's TUI subscription pattern: each panel owns a subscription, renders from local state, updates on notifications.
- For task ordering in Today: by priority desc, then by capture time asc. Simple and predictable.
- The CLI's `tasks list --json` is the test scaffold for everything — pipe it into jq, assert on shape.

**Definition of done**
- The phase 4b demo (todo → appears in TUI Today → press d → marks done → TUI updates) works. CLI list-with-filters works. State transitions tested.

---

## Phase 5 — Markdown files + notes plugin

**Pre-flight check**
- Phases 4a and 4b complete. Plugin model proven.
- Decided: the markdown root directory (`~/Documents/<name>/` or `~/<name>-notes/` — under `$HOME` so users can find it; configurable).
- Decided: filename convention (`<id-prefix>-<slug>.md`, with frontmatter for metadata).

**Steps in order**
1. **`storage` per-entry write lock**: implement the serialization model from the tech doc. A `Mutex<()>` or single-threaded actor per entry ID. All writes (from plugins, from the file watcher) acquire the lock before applying. This eliminates the SQLite-vs-frontmatter race.
2. **`storage` file manager**: `promote_to_file(entry)` writes a markdown file with YAML frontmatter + body. `demote_to_row(entry)` is the reverse (only for tiny bodies). Add `body_path` column to `entries`. Both operations go through the per-entry lock.
3. **Frontmatter spec**: YAML with `id`, `kind`, `project`, `principal_tags`, `free_tags`, `created_at`. The daemon writes it; the user can edit it; the daemon re-reads it on file change.
4. **`notify` integration**: watch the markdown root recursively. On `Modify`, debounce 200ms, acquire the entry's write lock, parse frontmatter, update SQLite metadata. Ignore swap files (`.swp`, `~`, `.*`).
5. **`plugins/notes` crate**: `note <body>` creates an entry of kind `Note`, always file-backed. `notes list` returns recent notes.
6. **CLI**: `notes edit <id>` resolves the file path, suspends, opens `$EDITOR`. Returns cleanly.
7. **TUI**: `Notes` view (key `n`). Pressing enter on a note opens `$EDITOR` (TUI suspends, restores on exit).
8. **Frontmatter conflict policy**: when the file is edited externally, SQLite re-reads the frontmatter. If the user changes `project:` in frontmatter, that overrides what's in SQLite. The file is the user-facing surface; SQLite is the cache.

**Architecture notes**
- The promotion threshold is intentionally soft. Notes always promote; ideas never promote unless the body exceeds some heuristic length; tasks promote when the user runs breakdown. Encode this per-plugin.
- The file format is "human-first": frontmatter at top, markdown below. A user opening one of these files in nvim should see something sensible without daemon mediation.
- File path resolution is a daemon concern. Plugins don't construct paths; they ask the daemon for a path given an entry.
- **Per-entry write lock is the contract.** Any code path that mutates an entry — plugin handler, file-watcher reaction, future sync replay — goes through it. Document this loudly in the storage crate.

**Gotchas**
- `notify` fires multiple events for one save (atomic rename pattern: create temp → write → rename). Debounce: ignore events for the same file within 200ms.
- Editor swap files. nvim creates `.<filename>.swp` while open. Filter these out in the `notify` handler.
- Symlinks. If the markdown root is a symlink (some users will set this up for Syncthing), `notify` may or may not follow it. Test both ways and document.
- `$EDITOR` not set. Default to `vi`. Don't crash; just open something.
- TUI suspension and restoration. `crossterm` has the dance: leave alternate screen + disable raw mode → spawn editor → wait → enter alternate screen + enable raw mode. If anywhere in this fails, the terminal breaks. Wrap in a guard.
- Lock contention. The per-entry lock is fine-grained (one per entry, not one global) so contention is minimal. But if a plugin holds a write lock during a long operation (LLM call, network fetch), other writes for that entry queue. Plugins should acquire late, hold briefly.

**Tips**
- Look at how Obsidian formats frontmatter — match it closely. Users may want to point Obsidian at the markdown root, and matching conventions makes that frictionless.
- Don't try to be smart about merging external edits with daemon-pending writes. Single-device, last-write-wins on file changes. Multi-device sync handles this in v2.
- Test the file watcher on the actual file systems users will hit: ext4, APFS, Btrfs (your CachyOS setup uses this). Each has slightly different fsync semantics.

**Definition of done**
- Note creation produces a markdown file with frontmatter. Editing the file via nvim updates SQLite. TUI notes view lists and opens files. No swap-file false positives.

---

## Phase 6 — Focus, breakdown, triage

**Pre-flight check**
- Phase 5 complete. Files round-trip cleanly.
- Spec out the breakdown flow on paper first: what the prompts say, how steps are validated, when promotion to project is offered.

**Steps in order**
1. **`plugins/focus`**: tiny state machine. `focus <project>` sets state, emits `focus.changed`. `unfocus` clears. State is one row in a plugin-state table.
2. **`core::focus_filter`**: a function that takes a query and current focus, returns a refined query. All "list X" queries route through this. Honors `scope:always`.
3. **`plugins/breakdown`**: a multi-turn JSON-RPC conversation. The plugin sends `Prompt { question, expected: StepText | StepCount }` requests to the *client* (TUI or CLI). The client renders a prompt, sends back the answer. The plugin validates and continues or finishes.
4. **Breakdown structure enforcement**: 3–5 steps required. Steps must be under ~80 characters (forcing concrete phrasing). If the user inputs more than 5, offer "this looks like a project — promote?".
5. **Breakdown output**: append a markdown checklist to the task's body. If the task was a row, promote it to a file first.
6. **`plugins/triage`**: iterates scratch entries. For each, prompts: assign project, set type, optional `when:today`. State held in plugin memory (resumable across sessions via a "next pending entry" SQL query).
7. **TUI: focus indicator** in header (visible from all views). Update on `focus.changed` events.
8. **TUI: breakdown modal**. Triggered by `b` on a selected task. Full-screen overlay with step prompts. Esc cancels mid-flow.
9. **TUI: triage modal**. Triggered by `T` from scratch panel. Full-screen overlay cycling entries. Quit/resume.
10. **Edge cases**: focus set when the focused project doesn't exist anymore (allow it — the project is just a string tag). `scope:always` items with no project (allowed).

**Architecture notes**
- The "multi-turn JSON-RPC conversation" is just regular request/response with the *plugin* initiating requests *to the client*. This requires bidirectional JSON-RPC, which the protocol supports natively (the client also acts as a server for prompt requests).
- The TUI's modal overlay is just another rendering mode in the `App` state. Toggle a flag, change what gets rendered, redirect keystrokes.
- Triage idempotency: the "next pending entry" query is `SELECT * FROM entries WHERE project IS NULL AND kind = 'Idea' ORDER BY created_at LIMIT 1`. Quitting and resuming just runs the same query.

**Gotchas**
- Breakdown step validation can feel hostile if too strict. Tune thresholds (length, count) on yourself first. If you're rejecting your own real steps, the rules are wrong.
- Bidirectional JSON-RPC means request IDs need to be unique across both directions. Easy fix: client uses negative IDs, server uses positive. Document the convention.
- TUI modal nesting (breakdown inside triage? — no, don't allow this). Make modal flows mutually exclusive.
- The focus filter shouldn't filter the action log (you want to see all activity). Make sure `scope:always` is recognized but also that *certain entry kinds* like `ActionLog` are exempt by default.

**Tips**
- Breakdown is the project's signature feature. Spend extra time on the prompt copy. "What does done look like?" is better than "Define completion criteria."
- Triage in the TUI should support keyboard-only flow: number keys for common projects, slash for "type a new project name."
- Test breakdown on a real task of yours during development. If it doesn't help you finish that task, the design is off.

**Definition of done**
- Focus works across all views. Breakdown produces a markdown checklist with concrete steps. Triage cycles scratch and assigns. `scope:always` items pierce focus. All flows have keyboard-only interactions in the TUI.

---

## Phase 7 — Event bus, jobs, hooks

**Pre-flight check**
- Phases 0–6 complete.
- Decided: event topic naming convention (`<plugin>.<noun>.<verb>`, e.g. `tasks.entry.completed`).
- Decided: cron-syntax-lite grammar for job schedules (don't use real cron — use simpler human phrases).
- **Read the tech doc's "Event bus: causality, idempotency, recursion guards" section before starting.** It locks in design constraints (causality metadata on every event, recursion depth cap, idempotency keys per hook) that must be built in *from the start*, not bolted on.

**Steps in order**
1. **Event envelope with causality**: in `proto`, define `Event { event_id, parent_event_id, origin, chain_id, chain_depth, topic, payload, emitted_at, emitted_by_plugin }`. Every published event carries this whole envelope.
2. **Event bus**: in-process pub/sub. `publish(event)` synchronously evaluates pattern matchers and pushes to subscriber channels. Subscribers are `(topic_pattern, channel)`. Pattern matching: glob-style (`tasks.*`, `*.created`).
3. **Recursion guard**: when a hook publishes an event, the new event inherits `chain_id` from the triggering event and increments `chain_depth`. If `chain_depth > 5`, drop the event with a logged error. Configurable cap.
4. **Plugin events**: extend the plugin manifest to declare `subscribes_to: [topic_patterns]` and `hooks: [{on_event, verb, idempotency_key}]`. The plugin manager forwards matching events over JSON-RPC. The plugin SDK fills in causality metadata automatically when a hook handler publishes events.
5. **Hook idempotency**: each hook entry in the manifest declares an `idempotency_key` expression (a derivation from event fields, e.g. `"task-done-{event.payload.task_id}"`). Before firing, the daemon checks an in-memory map `(hook_id, idempotency_key) -> last_fired_at`. If matched within the configurable dedup window (default 60s), skip the fire. Default: no idempotency key means fire every time.
6. **Action log**: subscribes to `*`, writes every event into `action_log` table with the full envelope (causality columns included). Backed by an unbounded channel — the log must never drop events. Schema: indexes on `(emitted_at, topic)`, `chain_id`, `parent_event_id` for tracing.
7. **Job scheduler**: `jobs` table with `next_fire_at` column. A background tokio task wakes on the soonest job, fires it (sends a JSON-RPC method to the owning plugin with a fresh causality envelope — origin is the job ID), updates `next_fire_at`.
8. **Schedule grammar**: parse "every 5 minutes", "daily at 23:00", "every Monday", "every hour at :30". Keep it small.
9. **Catch-up policy**: on daemon startup, jobs whose `next_fire_at` was missed during downtime fire *once* (catch-up), then resume the schedule. Configurable per job (some jobs explicitly want skip-on-miss).
10. **`plugins/daily-log`**: subscribes to `*`, accumulates an in-memory bucket per day, fires a scheduled job at user-configured time (e.g. 23:00) that writes the markdown summary. The hook is idempotent (key: `"daily-log-{event.payload.date}"`) so it can't double-fire even if the schedule glitches.
11. **TUI: Events panel** (key `e`). Subscribes to `*`, renders a live tail. Each event shows `chain_id` and depth for traceability.
12. **TUI: Trace view** (post-MVP nice-to-have, not required): given a `chain_id`, show the whole causal chain. Defer if tight on time.
13. **Job persistence**: jobs survive daemon restarts. On startup, the scheduler reads pending jobs and resumes.

**Architecture notes**
- The bus is in-process and synchronous in the *publishing* path (publish returns when all matchers are evaluated). Delivery to subscribers (via channels) is async — slow subscribers don't block publishers.
- Most events are fire-and-forget. If a subscriber's channel is full, drop with a log warning. Exception: the action log uses an unbounded channel — it must never drop, because losing log entries means losing audit/replay capability.
- Hooks vs jobs: a hook is "when event happens, run this." A job is "at time T, run this." Unified dispatch path — both invoke a plugin verb. Difference is the trigger and the causality envelope (jobs get `origin = job:<id>`, hooks get `origin = hook:<id>`, `parent_event_id = <triggering event>`).
- The causality fields are five extra columns and an in-memory dedup table. Cheap. Pays for itself the first time you debug a "why did this fire twice" bug.

**Gotchas**
- **Synchronous fan-out latency** if many subscribers. Mitigation: bounded channels with `try_send`. The publish call returns quickly even with hundreds of subscribers.
- **Clock drift** on scheduled jobs. Catch-up vs skip policy needs to be a per-job config field, not a global one. Some jobs (daily-log) want catch-up; others (every-5-minutes fetch) want skip.
- **Action log size.** Will be your biggest table. Index `(emitted_at, topic)` minimum. Add `chain_id` and `parent_event_id` indexes for trace queries. Consider rotation/pruning post-MVP.
- **Hook recursion without the guard.** Without the depth cap, "task done → daily-log update → emits update event → hook fires → publishes → triggers another hook" goes infinite. The cap is non-negotiable.
- **Idempotency key derivation errors.** If a plugin author writes a buggy idempotency expression, hooks could either never deduplicate or always deduplicate. Validate at manifest-load time that the expression parses; fail loud.

**Tips**
- The event bus is the single most reusable piece of the architecture. Spend the time on a clean API; you'll lean on it for everything.
- For the daily-log plugin's first version, just dump every event of the day as bullet points. Refine later. The point is the loop is closed: events → log → daily summary.
- Build the eww follow-on as a 2-hour project after the bus works. It validates that streaming subscriptions from outside the workspace work as advertised.
- Add a `<name>-cli trace <chain_id>` command that prints the whole chain from the action log. Indispensable for debugging hook cascades. Cheap to add.

**Definition of done**
- Plugins receive events they subscribe to. Events carry causality metadata. Hooks fire on events with idempotency. Recursion depth is capped. Jobs fire on schedule with catch-up policy. Daily-log produces a real markdown summary at end of day. TUI events panel shows live activity. CLI trace command works.

---

## Phase 8 — Search + crypto plugin

**Pre-flight check**
- Phases 0–7 complete.
- Decided: FTS5 vs. a Rust-side fuzzy matcher. FTS5 for now (built into SQLite, fast, no extra dep).

**Steps in order**
1. **FTS5 setup**: an FTS5 virtual table mirroring the `entries` body. Triggers on insert/update/delete in `entries` keep it in sync.
2. **`plugins/search`**: `find <query>` verb. Parses the query for filter syntax (`project:`, `type:`, etc.), passes the rest to FTS5. Returns ranked results.
3. **`cli`**: `find <query>` and `find --json`. The JSON output is NDJSON, one result per line, sorted by FTS rank.
4. **`cli`**: `<name>-cli search rebuild` command — drops the FTS5 index, disables triggers, bulk re-inserts from `entries`, re-enables triggers. Needed for the future sync v2 bulk-write scenario; shipping it now means it exists when needed. Even single-device users may want it after a crash.
5. **TUI: Search panel** (`/` from anywhere). Live filter as you type — debounce 50ms, query on every input, render results. Filter chips above the input show active filters.
6. **`plugins/crypto`**: config in `~/.config/<name>/plugins/crypto.toml` listing asset/symbol/threshold. A scheduled job (every minute) hits a free price API, stores readings as entries of kind `PriceReading`. A hook on `crypto.price.received` checks thresholds and emits notifications.
7. **Example eww widget**: an eww config that subscribes to `crypto.*` events via `<name>-cli watch --topic crypto.* --json` and renders the latest price for a configured asset. Lives in `examples/widgets/eww-crypto/`.

**Architecture notes**
- FTS5 doesn't natively do fuzzy matching. For typo tolerance you'd need to layer something (e.g. trigram index, or `tantivy`). For MVP, exact-substring search is enough.
- Crypto plugin is the *example of a feed plugin*. Document it well — it's what plugin authors will copy as a template.
- The eww widget is a *renderer*, not a client. It receives NDJSON from the CLI's subscription path. Eww has a `listen` source type designed for exactly this.

**Gotchas**
- FTS5 triggers fire on every insert/update. Bulk inserts during sync (post-MVP) will be slow. Disable triggers, bulk insert, rebuild FTS. Note in code for future-you.
- The free price API will rate-limit. Cache aggressively; respect 429s. Document expected request rate in the config.
- Eww's listen source spawns a subprocess and reads its stdout forever. If the daemon restarts, eww doesn't reconnect. Document the limitation; consider an eww wrapper script that respawns.

**Tips**
- The TUI search experience can be the killer feature. Get this *fast* — sub-50ms from keystroke to rendered results. People will use it more than they expect.
- Crypto plugin's API choice: CoinGecko's free tier is generous and doesn't require an API key. Start there.
- The eww widget should look minimal — a single number with a delta. Resist the urge to add charts in MVP.

**Definition of done**
- TUI search is live, fast, filterable. CLI `find` works. Crypto plugin fetches and stores prices, fires threshold notifications. Eww widget shows live price on a monitor.

---

## Phase 9 — Inbound MCP

**Pre-flight check**
- Phases 0–8 complete.
- `rmcp` version pinned and reviewed.
- Decided: which AI agent is the documented Route B default (probably `opencode` — open source, MCP-capable, terminal-friendly).

**Steps in order**
1. Add `rmcp` to `daemon` dependencies. Build a `McpServer` module that translates the daemon's verbs into MCP tools and the daemon's query API into MCP resources.
2. **Shared query layer between MCP and TUI**: the TUI's subscriptions and the MCP tools call the *same* internal query functions. The `today_panel_query()`, `scratch_query()`, etc. live in `core` (or a `queries` module) and have a single canonical shape. MCP tools wrap them with MCP-specific metadata; TUI panels wrap them with rendering. Both produce identical data.
3. Tool mapping: each verb becomes a tool. Tool name = verb name. Tool description = a sentence about what it does. Input schema = the verb's expected arguments, generated from the `proto` types.
4. Resource mapping: read-only queries (list projects, get current focus, list recent entries, list scratch) become MCP resources with stable URIs (`<daemon>:projects`, `<daemon>:focus`, etc.). These wrap the same query functions the TUI uses.
5. Streamable HTTP server: `rmcp` provides this. Bind to `127.0.0.1` on a configurable port. Gate the whole MCP feature behind a config flag (off by default).
6. Documentation: an MCP config file for Claude Code and one for opencode pointing at the daemon. Include in `examples/mcp/`.
7. Route B Voxtype wrapper: `voxtype-to-opencode.sh` script that pipes the transcript through opencode with the daemon's MCP config attached. Drop in `examples/voxtype-route-b/`.
8. Hyprland keybind example: `Super+Shift+V` triggers Route B; `Super+V` (from phase 3) triggers Route A.
9. **Trust validation test**: open the TUI's Today panel. Ask Claude Code (via MCP) "what tasks do I have today?". The list must match exactly. Make this a documented manual test, then automate it.
10. Integration tests: spin up the daemon with MCP enabled, connect a mock MCP client, list tools, call a tool, assert the side effect.

**Architecture notes**
- MCP tools are *one-shot RPCs* from the protocol's perspective. The daemon implements each by dispatching to the same verb handlers the CLI uses. No new logic.
- **MCP tools and TUI subscriptions share data shape.** Both go through the same query functions in `core`. If an agent shows you a different list than your TUI does, trust dies fast.
- Resources can be subscribed to in MCP, but we defer this (one-shot reads only for MVP). The agent re-reads when it needs fresh state.
- The MCP server runs *inside* the daemon process — it's another tokio task, not a separate process. Shutdown coordinates with the daemon's overall shutdown.

**Gotchas**
- `rmcp` evolves. Pin a specific version and *don't* track main. Spec updates are easier to absorb on a controlled cadence.
- Tool descriptions matter — they're what the LLM sees when deciding which tool to call. Write them carefully. "Add a todo" is worse than "Add a new todo item to a project, with optional priority and due date."
- Localhost binding: even though it's localhost, anyone on the machine can connect. If the user runs an untrusted process, it could call daemon tools. For a single-user system this is fine; document the caveat.
- Tailscale MCP later: when remote agents connect over Tailscale, identity becomes a real concern. The Streamable HTTP transport supports auth headers; defer to a future phase.
- **Query divergence between MCP and TUI** is the subtle failure mode. If you add a new query path that only one of them uses, you create the kind of contradiction that breaks user trust. Discipline: every new query is a function in `core`; both surfaces wrap it.

**Tips**
- For the demo, use Claude Code or opencode with the MCP config. Ask "what tasks do I have?" — the agent will discover the tool and call it. Magical when it works.
- Route B is where the LLM cost (or local model latency) becomes user-visible. Document expectations clearly: Route A is instant, Route B takes a second or two.
- The "tool description" is the most important piece of LLM context. Iterate on these descriptions by testing real natural-language inputs and seeing if the right tool gets called.

**Definition of done**
- Claude Code (or opencode) connects to the daemon's MCP endpoint and can call every verb as a tool. Route B voice wrapper works end-to-end. Documentation is enough that a friend can wire it up.

---

## Phase 10 — Polish, docs, release

**Pre-flight check**
- Phases 0–9 complete.
- The MVP success criteria from the MVP doc all hold for *you*, the user, after a week of dogfooding.

**Steps in order**
1. **Defaults audit**: every config option's default. Are they sensible for a new user? Document the defaults explicitly in a "config reference" doc.
2. **First-run experience**: when the daemon starts for the first time, create the config dir, the markdown root, an empty SQLite db, and emit a friendly welcome event. The user opens the TUI and sees a hint: "Hit Super+V to capture an idea, or type :help here."
3. **Installation**:
   - Cargo install path: `cargo install <name>` from a published crate or a git ref.
   - AUR package: `<name>-bin` for Arch users. Update CachyOS-friendliness.
   - Homebrew tap on macOS (deferrable to v0.2 if time-constrained).
   - Systemd user unit shipped as an example for Linux autostart.
4. **Documentation**:
   - README with quickstart (install, capture your first idea, open TUI).
   - `docs/plugin-author-guide.md` showing how to write a plugin from scratch.
   - `docs/mcp-integration.md` showing Claude Code / opencode setup.
   - `docs/architecture.md` distilling the tech doc for contributors.
5. **License**: MIT/Apache-2.0 dual. Add `LICENSE-MIT` and `LICENSE-APACHE`. Update `Cargo.toml` license fields.
6. **Naming**: lock the project name. Update all crate prefixes, socket paths, config paths. (If you've been using a placeholder, this is the last chance to swap.)
7. **Versioning**: tag `v0.1.0`. Future tags follow SemVer; pre-1.0 means breaking changes are allowed.
8. **Release notes**: a short narrative of what shipped, what's known-incomplete, what's coming next.

**Gotchas**
- Naming changes are surprisingly invasive: socket paths, config paths, crate names, binary names, branding everywhere. Do this in one PR and grep everything.
- Installation from `cargo install` requires all dependencies to be on crates.io. If you depend on a git ref of anything, that breaks `cargo install`. Audit dependencies.
- macOS Gatekeeper will complain about unsigned binaries. Document the workaround (`xattr -d com.apple.quarantine`) or accept that v0.1 is "you build from source" on macOS.

**Tips**
- The quickstart in the README should take a fresh user under 10 minutes from `git clone` (or `cargo install`) to "I captured my first idea." Time yourself.
- Release without fanfare. v0.1 is "it works for me, and the architecture is clean enough that a contributor could try." Save the launch post for v0.5 when there's real user feedback baked in.
- Open issues for everything in the post-MVP roadmap from the build plan. Lets contributors see what's wanted.

**Definition of done**
- Anyone with a Linux machine can install the daemon, follow the quickstart, and capture their first idea within 10 minutes. The MVP success criteria all hold. v0.1.0 tagged and announced.

---

## Working tips that apply to every phase

- **Commit often, on green only.** Each commit should leave `cargo build && cargo test` passing. Use a `pre-commit` hook to enforce this if necessary.
- **Write the test before the feature.** Especially for `core` logic — parsers, filters, state machines. Easier to know when you're done.
- **Open a "Decisions" doc.** Every time you make a tech choice mid-build that wasn't in the tech doc, log it there with a one-line rationale. Future-you and contributors will need this.
- **Keep the demos working.** As you add phases, the earlier demos may break. Have a `just demo <phase>` recipe that runs each phase's demo as a smoke test. Run before any commit.
- **Don't refactor backward.** If phase 6 reveals that phase 4a's architecture was wrong, write down the lesson and keep moving. Refactor at the *end* of phase 10 if at all. Pre-MVP refactors eat months.
- **Resist scope creep per phase.** If a phase is taking 2x its estimate, ship the partial version and move the missing bits to the next phase. Never let a phase block the whole pipeline.
