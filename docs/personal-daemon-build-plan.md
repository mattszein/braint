# Personal Daemon — MVP Build Plan

A phased plan with concrete milestones. Each phase ends on a demoable state — meaning at the end of the phase, you can show a working slice of the product, even if narrow.

This is the *build sequence*. Scope per phase is intentionally small. The MVP doc defines the destination; this doc defines the route.

## Working principles

- **Domain pure, I/O at the edges.** Lessons from DDD without the ceremony. `core` crate has no SQLite, no sockets, no `tokio`. Pure logic, fast unit tests.
- **One vocabulary, everywhere.** `entry`, `project`, `verb`, `scratch`, `triage`, `focus`, `scope`, `breakdown`, `principal tag`. Same words in code, docs, and conversation.
- **Each phase ends demoable.** No phase is "lay groundwork for the next phase." If you stopped after phase 3, what you have is useful.
- **TUI is the default interactive surface; CLI subcommands are for one-shot and scripted use.** Both go through the same `client` library; no duplicated logic. The CLI binary is dual-mode: with arguments it acts as a CLI tool; without arguments it launches the TUI.
- **Tech doc decisions are locked.** rusqlite, JSON-RPC 2.0 over local sockets, NDJSON for CLI output, `rmcp` for MCP, two binaries in a workspace. Don't relitigate during the build.

## Surfaces: TUI in-repo, graphical widgets in separate repos

The **TUI is the canonical interactive client**, built in `ratatui` and shipped as part of the CLI binary. Run `<name>-cli` without arguments and you get the inspector: today's tasks, scratch, focus, search, breakdown, triage, live events. Run `<name>-cli` with a subcommand and you get one-shot CLI behavior. Same binary, dual mode. The TUI grows view-by-view alongside each phase.

Why TUI in-repo and first-class:
- Works on Linux, macOS, Windows identically. Works over SSH (great for managing your homelab from a laptop tmux pane).
- Exercises every IPC path — live subscriptions, multi-turn flows, request/response. If the protocol is awkward, the TUI surfaces it immediately.
- Becomes the reference client: anything you can render in the TUI you can render in a graphical widget. Same wire protocol underneath.
- Matches the project's terminal-first ethos.

Why graphical widgets stay in separate repos under the same GitHub org:
- Platform-specific (eww/Linux/Hyprland, Swift/macOS menubar, JavaScript/GNOME, QML/Plasma).
- Iterate independently of daemon code.
- Different contributor skill sets.
- Each is a small reference implementation of "render this query as a glance view."

When the first eww widget appears (phase 7, as a cheap follow-on once the event bus exists), it lives in `examples/widgets/` as a config file. Polished cross-platform widgets are separate repos.

**TUI scope guard:** the TUI is fun to build and easy to over-build. In MVP it stays minimal — scratch inspector, today view, search, basic task management, focus switcher, breakdown and triage flows, live event panel. Themes, fancy popups, animations are post-MVP.

### Rough TUI layout sketch

A loose mockup of what the default view looks like — refined as you build, not a binding spec:

```
┌─ <name> ─────────────────────────────── focus: pro-rails ──┐
│                                                            │
│ Today (5)                    │ Scratch (12 untriaged)      │
│   ⬚ finish auth refactor [H] │   • explore CRDTs for sync  │
│   ⬚ review PR #143           │   • try cr-sqlite           │
│   ▣ deploy staging           │   • bookmark that talk      │
│   ⬚ gym at 11 [always]       │   • feedback on RFC         │
│   ⬚ rest break [always]      │   ... 8 more                │
│                              │                             │
├──────────────────────────────┼─────────────────────────────┤
│ Recent activity              │ Search                      │
│ 10:42 idea pro-rails         │ > _                         │
│ 10:31 done #142              │                             │
│ 10:15 todo today             │                             │
│ 09:58 focus pro-rails        │                             │
│                              │                             │
└────────────────────────────────────────────────────────────┘
 [t]oday [s]cratch [p]rojects [f]ind  [F]ocus  [b]reakdown  [?]
```

Vim-style keybindings by default (`hjkl`, `:` for command mode, `/` for search), with arrow keys also working. `Enter` opens an entry. `Tab` cycles panes. `?` shows help. `:`-mode runs the same verb grammar as the CLI (`:todo for pro-rails — finish auth`). Header shows current focus from every view.

## Phase 0 — Workspace skeleton

**Goal:** Cargo workspace compiles. CI runs. No product behavior yet.

**Concrete deliverables:**
- Workspace with empty crates: `proto`, `core`, `client`, `plugin-sdk`, `daemon` (storage as a module inside), `cli`.
- One workspace `Cargo.toml`, shared `[workspace.dependencies]` pinning versions of `tokio`, `serde`, `serde_json`, `rusqlite`, `interprocess`, `notify`, `clap`, `ratatui`, `crossterm`, `anyhow`, `thiserror`, `uuid`, `tracing`.
- GitHub Actions: `cargo build`, `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check`, on Linux + macOS.
- A `justfile` with common commands (`just dev`, `just check`, `just test`). Simple, no extra crate needed. If automation grows complex enough to justify it later, migrate to an `xtask` binary — but that's a "later" problem.
- Project name chosen. README with a one-paragraph description.

**Demo:** `cargo build` succeeds, `cargo test` passes (zero tests), CI is green.

**Time:** half a day to a day.

---

## Phase 1 — Skeleton end-to-end

**Goal:** CLI sends a JSON-RPC message over a local socket; daemon receives it, persists an entry to SQLite, responds. The simplest possible pipe, working.

**Scope:**
- `proto` crate: `Entry`, `EntryId`, `EntryKind`, `ProjectId`, `TagSet`, `HybridLogicalClock`, `DeviceId`. JSON-RPC method `ingest` with one parameter (a raw text string). One response shape.
- `core` crate: parse incoming text into `Entry` (default `EntryKind::Idea`, no project, no tags). UUIDv7 generation. HLC initialization.
- `daemon::storage` module: SQLite migration that creates the `entries` table. `save(entry)` and `get(id)` functions. WAL mode on. Lives inside the daemon crate as a module.
- `daemon` binary: opens local socket, accepts JSON-RPC, dispatches `ingest` to a handler, persists via the storage module, responds with the entry ID.
- `cli` binary: one subcommand, `<name>-cli ingest "<text>"`. Connects to local socket, sends JSON-RPC request, prints the entry ID. Exit codes for success / daemon-unreachable.
- `client` crate: extract the IPC connect-and-send logic for reuse.

**Out of scope:**
- Multiple verbs. Only `ingest` exists.
- Projects, tags, types — everything is just an `Idea` entry.
- Markdown files. Body lives in SQLite.
- Voice, widgets, MCP, plugins, events, search, jobs, hooks.

**Demo:** Run the daemon in one terminal. In another: `<name>-cli ingest "explore CRDTs for sync"`. Get an ID back. Inspect SQLite directly and see the row.

**Tests:** Unit tests for `core::parse_ingest`. One integration test that spins up the daemon, sends a request through `client`, asserts the row appears in SQLite.

**Time:** 2–3 days. The point of this phase is to prove every crate boundary works.

---

## Phase 2 — Verb grammar

**Goal:** The same `ingest` path now parses verbs. Multiple verbs work. Principal tags work. Confirmation policy is in place.

**Scope:**
- `core`: verb parser. First word matches a known verb (`idea`, `todo`, `note`, `capture`). Optional `for <project>` or `project:<id>` recognized. Optional `priority:`, `when:today`, `due:`, `scope:`. Free tags via `#tag` or `tags:a,b,c`. Verb table is in `core`, extensible.
- `proto`: extend `Entry` with `project`, `principal_tags`, `free_tags`. Add `Confirmation` request/response for the two-step voice flow. Add `Subscription` message type for streaming queries (server-pushed notifications).
- `daemon`: when a request comes in flagged `source: voice`, generate a confirmation request, hold the parsed entry pending, send it back. CLI calls `confirm` to commit, or `cancel` to drop. CLI source skips confirmation.
- `daemon`: streaming query support. A client can open a long-lived JSON-RPC subscription (e.g. "watch scratch", "watch tasks in project X") and the daemon pushes updates as JSON-RPC notifications whenever matching entries change.
- `cli`: add subcommands `idea`, `todo`, `note`, `capture` (mostly aliases that prepend the verb to the text). Add `--source voice` flag for the wrapper script. Add `--json` for NDJSON output.
- `cli`: **dual-mode behavior.** Running `<name>-cli` with no arguments launches the TUI. Running with a subcommand stays in one-shot CLI mode.
- `cli` (TUI mode): first version of the inspector built in `ratatui`. Two panels for now: **Scratch** (live-updating list of untriaged entries, navigable with j/k, enter to open) and **Recent activity** (live stream of incoming entries). A `:` command line for typing verbs (`:idea for pro-rails — …` runs the same grammar as the CLI). `?` shows help. `q` quits.
- The TUI consumes the same streaming subscriptions as any other client — it's the first real client of that path.

**Out of scope:**
- Persistence of `status`, `repeat`, subtasks — those land with the tasks plugin in phase 4b. Right now status defaults to `open` for todos and that's it.
- Markdown files on disk. Bodies still in SQLite.

**Demo:**
1. `<name>-cli idea "explore CRDTs for sync"` → idea in scratch.
2. `<name>-cli todo project:demo "finish the parser" priority:high when:today` → fully-specified todo, no confirmation, lands tagged.
3. With `--source voice`: same call returns a pending confirmation; `<name>-cli confirm <id>` commits it.
4. In one terminal: run `<name>-cli` with no arguments — the TUI opens, scratch panel showing entries, recent-activity panel scrolling. In another terminal: capture a few ideas via CLI. The TUI updates live. Type `:idea for pro-rails — something` inside the TUI to capture without leaving.
5. Inspect SQLite: rows show correct `project`, principal tag JSON, free tags.

**Tests:** A solid suite of `core::parse_verb` tests for the grammar. Round-trip tests for confirmation flow. Subscription test that asserts a `watch` subscription receives a notification when a matching entry is created. CLI golden-output tests for `--json` mode. TUI smoke tests using ratatui's test backend (renders to a buffer, asserts on the output).

**Time:** 6–8 days. The TUI adds real work, but it's the most leveraged work in the project — every subsequent phase gets a TUI view almost for free, and the streaming protocol gets validated now instead of in phase 7.

---

## Phase 3 — Voice via Voxtype

**Goal:** A keybind in Hyprland triggers Voxtype, the transcript hits the daemon, you see the entry land.

**Scope:**
- Example wrapper script `voxtype-to-daemon.sh` — reads stdin, calls `<name>-cli ingest --source voice "$(cat)"`.
- Example Voxtype config snippet using `post_process_command`.
- Example Hyprland keybind snippet.
- Confirmation UX: simplest possible — a desktop notification ("Pending: 'idea — explore CRDTs.' Confirm?") via `notify-send`, with a second keybind for `<name>-cli confirm latest`. Cancel after N seconds if no response.
- Documentation in `examples/voxtype-route-a/`.

**Out of scope:**
- Route B (LLM-routed). Comes in phase 9 when MCP is up.
- A nice on-screen confirmation widget. Notifications are fine for MVP.

**Demo:** Hit a keybind. Say "idea — try cr-sqlite for sync." Get a notification asking to confirm. Hit the confirm keybind. See it in SQLite, properly parsed and tagged.

**Tests:** Manual end-to-end on the dev machine. Hard to unit-test the keyboard → audio → STT → script pipeline; settle for documented manual test steps.

**Time:** 1–2 days. Most of the work is documentation and config; the code already exists.

---

## Phase 4a — Plugin infrastructure

**Goal:** The plugin subsystem works end-to-end. A trivial "hello plugin" runs as a subprocess, registers a verb, handles a request, returns a response. No real product behavior yet — just the architecture proven.

**Scope:**
- `plugin-sdk` crate: subprocess JSON-RPC plumbing, plugin manifest type, helpers for registering verbs and event subscriptions. Attribute macros (`#[verb]`, `#[on_event]`) that generate manifest entries at compile time.
- **Manifest via `--manifest` flag**: the daemon invokes the plugin binary with `--manifest`, plugin prints JSON to stdout and exits, daemon parses. Manifest never drifts from the binary. (See tech doc.)
- `daemon`: plugin manager. On startup, scans the bundled plugins directory and the user plugins directory, invokes `--manifest` on each binary, parses, then spawns each plugin as a subprocess over stdio for the real run.
- `daemon`: verb routing through plugins. Verb resolution checks plugin-registered verbs and routes the parsed `VerbInvocation` over JSON-RPC to the owning plugin. Plugin returns a `VerbResult`. Daemon commits.
- `daemon`: plugin lifecycle. Subprocess crash detection (read pipe closed). One automatic restart with backoff. Graceful shutdown on daemon SIGTERM (kill child processes cleanly across Linux/macOS).
- `daemon`: enforces plugin governance rules from the tech doc — event topics namespaced to the plugin's name, verb name uniqueness, manifest API version compatibility check.
- A `plugins/hello` example plugin in the workspace whose only job is to validate the plumbing: registers a verb `hello`, responds with a fixed string. Discarded after this phase or kept as a reference.

**Out of scope:**
- The tasks plugin (that's 4b).
- TUI updates (that's 4b).
- Hot-reload — for now, restart the daemon to pick up plugin changes.

**Demo:** Run the daemon. Run `<name>-cli hello "world"`. The daemon routes the verb to the hello plugin subprocess, gets back "hello world," and the CLI prints it. Kill the plugin manually with `kill`; the daemon logs the crash and restarts it once. Kill it twice; the daemon stops restarting and logs the giveup.

**Tests:** Plugin lifecycle tests (spawn, send, receive, clean shutdown, crash, restart, give-up). Manifest parsing tests. Governance enforcement tests (try to register a duplicate verb → refused; try to emit an unprefixed event → refused). All using the `hello` plugin as the test subject.

**Time:** 5–7 days. This is the hardest infrastructure phase. Subprocess management, JSON-RPC over stdio framing, manifest contract, governance rules, lifecycle semantics. Investing in clean test infrastructure here pays for every later plugin.

---

## Phase 4b — Tasks plugin + TUI integration

**Goal:** The first real plugin (`tasks`) is built on the now-proven infrastructure. The TUI grows task panels with live updates.

**Scope:**
- `plugins/tasks` crate: provides verbs `todo`, `done`, `drop`, `block`. Manages task state transitions. Subtasks as markdown checkboxes (in the body). Status, priority, scope, when, due, repeat are stored as principal tags.
- `core`: a `Task` view over `Entry` — given an entry of kind `Todo`, derive its current state, priority, etc. from principal tags. The "entry is the source of truth" rule holds.
- `cli`: new subcommands `done <id>`, `drop <id>`, `block <id>`, `tasks list [filters]`. `tasks list` supports `--project`, `--status`, `--when`, etc.
- `cli` (TUI): add a **Today** panel showing tasks with `when:today`, ordered by priority. Add a **Tasks** view (press `t`) listing all tasks with filters. Mark done / drop / block via single keystrokes. The panels subscribe to task events; state changes show up live.

**Out of scope:**
- Focus filtering. That's phase 6.
- Breakdown. That's phase 6.
- Recurring task instantiation (the `repeat:` directive is *stored* but no scheduler runs yet — that's phase 7).

**Demo:** Voice or CLI: `todo for pro-rails — finish auth refactor`. Open the TUI — the new task appears in Today (it had `when:today` if you said so) and in the Tasks view. Press `d` on it to mark done; the row updates immediately. Confirm via `<name>-cli tasks list --project pro-rails` from another terminal.

**Tests:** State-transition unit tests in `plugins/tasks/core`. Integration test that uses CLI to add and complete a task end-to-end. TUI smoke tests on the Today and Tasks panels.

**Time:** 3–4 days. The infrastructure is already there from 4a; this is mostly product surface work.

---

## Phase 5 — Markdown files + notes plugin

**Goal:** Long-form entries live as markdown files on disk. The notes plugin uses them. External edits are picked up.

**Scope:**
- `storage`: file management. When an entry's body crosses a threshold (or the plugin explicitly asks), promote to a markdown file under a configured directory. Filename is derived from the entry ID plus a slug. SQLite row keeps a `body_path` pointer.
- `storage`: `notify` integration. File changes outside the daemon → trigger a re-read, parse YAML frontmatter for principal tags, update SQLite metadata.
- `plugins/notes` crate: verbs `note` (create) and `notes list`. Notes are always file-backed.
- `cli`: `notes edit <id>` opens `$EDITOR` on the file. `notes list` shows recent notes with project / tags.
- `cli` (TUI): add a **Notes** view (press `n`) listing notes with project / tags. Pressing enter on a note opens `$EDITOR` (TUI suspends, editor takes over, returns cleanly).

**Out of scope:**
- Two-way conflict resolution between editor save and daemon-side change (single-device, single-editor, eventual consistency is fine for MVP).
- Markdown frontmatter is *read* by the daemon but the canonical source of truth for principal tags is still SQLite. Frontmatter is a convenience for users editing files directly.

**Demo:** Voice or CLI: `note for pro-rails — thoughts on the auth refactor architecture`. Open the resulting `.md` file in `nvim` directly. Edit. Save. Run `<name>-cli notes list` — the daemon picks up the change.

**Tests:** File-promotion logic unit tests. `notify` integration test that touches a file and asserts the SQLite row updates.

**Time:** 4–5 days.

---

## Phase 6 — Focus, breakdown, triage

**Goal:** The three structural plugins that turn "stuff in a database" into "an executive-function aid."

**Scope:**
- `plugins/focus`: verbs `focus <project>` and `unfocus`. Stores current focus as a tiny piece of plugin state. Publishes `focus.changed` events. Adds `scope:always` to entries that should bypass focus filtering.
- `plugins/breakdown`: verb `breakdown <task-id>`. The multi-turn flow is implemented as a JSON-RPC conversation: the plugin prompts, the client responds, the plugin enforces structure (3–5 steps, executable phrasing). Writes the steps as a markdown checklist into the task's body (promoting it to a file if needed). Detects "more than 5 steps" → offers to promote to a project.
- `plugins/triage`: verb `triage`. Walks scratch entries one at a time; for each, lets the user assign a project, type, and optional `when:today`. Idempotent — you can quit and resume.
- `cli`: `focus <project>` one-shot. `breakdown <id>` and `triage` exist as one-shot CLI commands too (for scripting), but the *primary* surface for these is the TUI.
- `cli` (TUI): focus indicator in the header (visible from every view). New full-screen modal flows for **Breakdown** (triggered by `b` on a task) and **Triage** (triggered by `T` from scratch panel). Vim-style entry — feels like a real interactive flow, not a prompt loop. Header shows current focus everywhere.
- `core`: a `focus_filter(query, current_focus)` function that all "list X" queries pass through. Honors `scope:always`.

**Out of scope:**
- Suggested project assignment based on past similar captures (could come with the LLM plugin later).

**Demo:**
1. Open the TUI. Press `:focus pro-rails`. Header updates. The task panels filter to pro-rails.
2. A `scope:always` rest reminder still shows up.
3. Move to a task, press `b` → full-screen breakdown flow. Walk through 3 steps with the prompt enforcing concrete phrasing. Exit; the task's body now has a checklist. Open the markdown file in `$EDITOR` to verify.
4. From scratch panel, press `T` → triage modal. Cycle through entries assigning each. Quit halfway, reopen, resume where you left off.

**Tests:** Focus filter unit tests (lots of edge cases — multi-project, scope:always, no focus set). Breakdown step-count validation tests. Triage idempotency tests. TUI modal-state tests using ratatui's test backend.

**Time:** 6–8 days. The TUI flows are real work but they're where the product earns its keep — these are the daily-driver interactions.

---

## Phase 7 — Event bus, jobs, hooks

**Goal:** The daemon's nervous system. Plugins can subscribe to events. Scheduled jobs run. Hooks fire on events.

**Scope:**
- `daemon`: event bus. In-process pub/sub keyed on topic strings. Plugins declare subscriptions in their manifest; the daemon routes published events to subscribed plugins over their JSON-RPC stdio channel.
- `daemon`: job scheduler. Reads scheduled jobs from a SQLite table; fires them via the appropriate plugin at the right time. Cron-syntax-lite ("every 5 minutes", "daily at 23:00", "every Monday").
- `daemon`: hook scheduler. Hooks are jobs that fire on events instead of on time. Plugin manifests declare `on_event: <topic>` hooks.
- `proto`: event envelope type. Subscription registration messages.
- The action log is now driven by event subscriptions, not by the verb router writing directly to SQLite. Cleaner: anything that's an event becomes a log entry automatically.
- `plugins/daily-log`: end-of-day job that aggregates the action log into a markdown summary for the day, written as a `daily-log` entry. Configurable schedule.
- `cli` (TUI): add an **Events** panel (press `e`) showing the live event stream — every verb invocation, every job firing, every hook reacting. This is the user-facing view of self-logging.

**Out of scope:**
- Distributed event bus across devices. Local only.
- A UI for managing scheduled jobs. Plugins declare them; users edit config files to override.

**Demo:**
1. Configure daily-log to fire at 23:00. Capture stuff during the day. At 23:00, a markdown summary appears in your notes.
2. Add a hook to the tasks plugin: "on task done, publish a notification." Mark a task done. See the notification.
3. Open the TUI's Events panel — watch the day's activity scroll by in real time.
4. As a tiny follow-on (a couple hours, not a full phase): an eww widget that consumes one of the daemon's streaming subscriptions and renders it as an always-on-screen glance view. Now that the streaming path is battle-tested by the TUI, eww is just a different renderer. Lives in `examples/widgets/eww-scratch/`.

**Tests:** Event bus pub/sub tests. Job scheduler timing tests (with mocked clock). Action log emission tests.

**Time:** 5–7 days. Lots of plumbing.

---

## Phase 8 — Search + crypto plugin

**Goal:** Search across everything. One feed-style plugin to prove plugins can do more than verbs.

**Scope:**
- `plugins/search`: verb `find <query>`. Full-text search via SQLite FTS5. Supports filters: `find rust project:pro-rails type:note`. Voice-friendly grammar.
- `plugins/crypto`: per-asset config in `~/.config/<name>/plugins/crypto.toml`. A job fires every minute (configurable), fetches prices from a free API, stores them as entries of kind `PriceReading`. Threshold alerts via notification hooks. A widget config example (eww) showing the latest reading.
- `cli`: `find <query>`, `find --json` for scripts.
- `cli` (TUI): add a **Search** panel (`/` from anywhere) with live fuzzy filtering as you type. Filter chips for project / type / status. Enter opens the selected entry. This is the daily-driver way to find things.

**Out of scope:**
- Vector search / semantic search. That's a future LLM-enabled feature.
- The full crypto experience (price history charts, alerts on % changes). Just current price + simple threshold.

**Demo:**
1. In the TUI, press `/`, type "auth" — results filter live across all entries. Press enter on one to open it.
2. `find auth --json` from the CLI for the same thing, piped to jq for a script.
3. Crypto widget on a configured eww monitor shows BTC/USD updating every minute.
4. Set a threshold; cross it; get a notification.

**Tests:** FTS query tests. Crypto plugin job tests with a mocked HTTP client.

**Time:** 4–5 days.

---

## Phase 9 — Inbound MCP

**Goal:** External AI agents (Claude Code, opencode) can connect to the daemon and call verbs.

**Scope:**
- `daemon`: embed `rmcp` Streamable HTTP server. Off by default; enabled via config flag. Binds to `127.0.0.1` on a configured port.
- Translate the daemon's verbs and resources into MCP tools and resources. The translation is mechanical because verbs already have a clean schema (defined in `proto`).
- Provide a small `mcp-config.json` example for opencode and Claude Code pointing at the local MCP endpoint.
- Documentation: how to connect Claude Code / opencode, what tools they see, how to use it for Route B voice.
- Example wrapper script for Route B in `examples/voxtype-route-b/` — pipes Voxtype transcript into opencode with the MCP config.

**Out of scope:**
- Auth on the MCP endpoint beyond bind-to-localhost. Cross-device MCP via Tailscale is post-MVP.
- Resource subscriptions (where MCP clients get notified of changes). Tools and one-shot resource reads only.

**Demo:**
1. Enable MCP in config, restart daemon.
2. Open Claude Code with the MCP config. Ask: "what tasks do I have for pro-rails today?" — Claude Code calls the daemon's `tasks list` tool and answers.
3. Voice via Route B: hit the LLM-routed keybind, say "remind me to refactor the menu bar in pro-rails sometime this week." opencode receives the transcript, calls the daemon's `todo` tool with sensible arguments.

**Tests:** MCP integration test using a mock MCP client. Tool-listing test asserts every verb is exposed.

**Time:** 4–6 days. `rmcp` handles most of the protocol work, but mapping verbs to tools is real work and so is integration testing.

---

## Phase 10 — Polish, docs, release

**Goal:** A first public 0.1 release. Linux, single-device. Documented and installable.

**Scope:**
- Installation: at least a Cargo install path and AUR-friendly packaging. macOS via Homebrew tap is nice-to-have. Windows deferred to a later release.
- Systemd user unit for daemon autostart on Linux.
- Defaults shipped: a sensible set of principal-tag prefixes, default config, default keybind suggestions.
- Documentation: README, quickstart, plugin author guide, MCP integration guide.
- The 9 first-party plugins (capture, tasks, notes, focus, triage, breakdown, daily-log, search, crypto) all stable.
- Choose name (if not already), choose license (MIT/Apache-2.0 dual is standard), publish.

**Demo:** You install it from scratch on a fresh CachyOS install in under 10 minutes, follow the quickstart, and it works.

**Time:** 3–5 days.

---

## Total estimate

If full-time: roughly **9–11 weeks** of focused work (phase 4 was split into 4a and 4b; the realistic estimate grew by about a week). For a side project at, say, 10–15 hours/week, more like **5–7 months** to a 0.1 release. The architecturally risky phases are **4a** (plugin infrastructure), **7** (event bus with causality and idempotency), and **9** (MCP). The TUI work is spread across phases 2/4b/5/6/7/8 and pays for itself by making every phase demoable interactively.

## Risk register

A few places the plan might slip and what to do about them:

- **Verb grammar gets gnarly.** If natural-feeling voice grammars conflict with each other, the parser might need to grow. Mitigation: keep the grammar small in phase 2 (4 verbs); extend with care.
- **Plugin subprocess lifecycle is fiddly.** Crashes, restarts, stdio buffering, child-process zombies, cross-platform signal semantics. Mitigation: borrow patterns from `rmcp`'s stdio transport. Invest in test infrastructure during 4a so later phases inherit good ergonomics.
- **`notify` cross-platform quirks.** Especially around fsync semantics and editor swap-files. Mitigation: read notes from atomic-writes (most editors do this); ignore swap-file paths via a small filter. Per-entry write lock handles the simultaneous-edit race.
- **MCP spec drifts.** It has moved several times in the past year. Mitigation: pin `rmcp` to a known-good version; revisit after release.
- **TUI scope creep.** A TUI is fun to build. You'll keep wanting to add views, popups, themes, animations. Mitigation: stick to the MVP TUI scope (scratch, today, tasks, notes, focus, breakdown, triage, events, search). Anything else is post-MVP. The TUI is a means; the daemon is the product.
- **TUI modal flows under-budgeted.** Breakdown and triage as full-screen modal flows in ratatui, with multi-turn JSON-RPC conversations, are state-heavy. Phase 6's 6–8 day estimate is tight. Mitigation: pre-design the state machines on paper; if you hit week two of phase 6, ship the simpler version and iterate.
- **Sync semantics are research-grade.** Already flagged in the tech doc. MVP doesn't ship sync, so this isn't an MVP risk — but the v2 phase will look more like research than build. Budget accordingly when you get there.
- **Hooks/jobs growing emergent complexity.** Recursion, race conditions, duplicate triggers. Mitigation: implement causality metadata, depth caps, and idempotency keys *in* phase 7, not after (per the event bus section of the tech doc). The early structure prevents whole classes of bugs.


## Definition of done for the MVP

Quoted directly from `mvp.md` for convenience — the MVP is done when, after a week of use, all of these are true:

- Capture has never failed.
- The user prefers dictating to the daemon over writing in another tool.
- The TUI inspector is open in a tmux pane or terminal tab most of the working day.
- The end-of-day digest is something the user actually reads.
- The breakdown verb has been used on a real task and helped it move.
- Scratch has been triaged at least a few times without it feeling like a chore.
- Adding a new plugin feels obvious enough that a contributor could try.
