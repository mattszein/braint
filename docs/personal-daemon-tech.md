# Personal Daemon — Tech Choices

A companion to the product idea and MVP docs. This pins down the technical decisions made so far, with rationales. Anything not listed here is deliberately deferred — pick it when you actually need it.

## Language: Rust

Single-binary distribution, low memory footprint, real concurrency, mature SQLite bindings, and great cross-platform support for both Linux and macOS. Async support is excellent. The compile-time cost is real but acceptable for a daemon you'll iterate on for years.

## Runtime: tokio

Standard for async Rust. Best ecosystem support for Unix domain sockets, named pipes (macOS/Windows path later), HTTP, SSE, and file watchers.

## Local IPC: JSON-RPC 2.0 over `interprocess` local sockets

- One local socket per daemon at a well-known path (UDS on Linux/macOS, Named Pipe on Windows). The `interprocess` crate gives one API across all three.
- Filesystem permissions handle authentication on Unix; Named Pipe ACLs on Windows. Only the user running the daemon can connect. No token system in v1.
- **Wire format: JSON-RPC 2.0**, length-prefixed framing. Same shape as LSP and as MCP's stdio transport — request/response with IDs, typed errors, server-pushed notifications without IDs for async events (progress, broadcasts).
- Binary serialization (bincode/postcard) only if a measured bottleneck appears.

## CLI output format: NDJSON (and human text)

CLI commands have two output modes:

- **Default human-readable text** for interactive use.
- **`--json` produces NDJSON** — one JSON object per line — for scripting, piping into `jq`, watching event streams (`daemon-cli watch events --json`), and structured logging. This is what modern CLIs (`gh`, `ripgrep --json`, `cargo --message-format json`) do, and Unix pipes handle it naturally.

**NDJSON and JSON-RPC are not competing.** JSON-RPC is the wire protocol for the daemon's local IPC (request/response with IDs, typed errors). NDJSON is the human/script-facing output format the CLI emits when asked for structured output. The CLI is a JSON-RPC client that prints results as NDJSON.

## Streaming progress: server-pushed events over local IPC

When a verb triggers a longer-running task (LLM generation, learning-topic research, news fetching, batch operations), the daemon streams progress events back to the caller over the open local socket. One-way push, JSON events, frame-delimited. CLI shows progress; widgets re-render as events arrive.

This is local-only and unrelated to MCP. The MCP-facing surface (below) uses a different transport.

## MCP: Streamable HTTP, embedded in the daemon, via `rmcp`

There are two MCP-adjacent capabilities, often conflated. Worth separating:

**Inbound MCP (daemon-as-server):** the daemon exposes its verbs (todo, idea, breakdown, search, focus, etc.) and queryable state (projects, focus, recent captures, scratch contents) as MCP tools and resources over **Streamable HTTP** ([2025-03-26 spec](https://modelcontextprotocol.io/specification/2025-03-26/basic/transports#streamable-http)). External MCP clients — Claude Code, Claude Desktop, opencode, a local LLM via Ollama with an MCP-aware wrapper, anything else MCP-compatible — connect to the daemon and call verbs as tools.

This is the highest-leverage piece of the architecture: **any MCP client becomes a natural-language adapter for the daemon's verb grammar, for free.** No daemon-side LLM integration code needed.

**Outbound LLM (the `ai-router` plugin):** an optional plugin that takes loose-form natural language input (typically from voice) and asks an external LLM to translate it into a verb call. This is the *opposite direction* — the daemon as a client of an LLM, not the LLM as a client of the daemon. Plugin, opt-in, separate concern.

In practice, most users will rely on inbound MCP (their existing AI agents call the daemon) and may never need the `ai-router` plugin.

### Implementation: `rmcp` (official Rust SDK)

Use [`rmcp`](https://github.com/modelcontextprotocol/rust-sdk), the official Rust SDK from the modelcontextprotocol org. Rationale:

- Official means closest to spec and longest-lived
- Supports Streamable HTTP, stdio, and SSE transports
- `#[tool]` and `#[tool_router]` macros remove protocol boilerplate
- Uses axum under the hood — so the axum question is settled by transitive dependency; we don't pick it directly
- Active development, matches the latest protocol revisions

The community alternative (`rust-mcp-sdk` from rust-mcp-stack) is also good and protocol-complete, but no reason to deviate from official.

### Where MCP lives

Embedded in the daemon binary, not a separate process. The MCP tools are thin wrappers around the daemon's internal verb router and queries — running MCP in a separate process would mean it talks back to the daemon over local IPC to do anything useful, which is one extra hop for no benefit.

The MCP HTTP listener is gated by a config flag (off by default in MVP) and binds to localhost. Remote access (e.g. from your phone or another laptop) goes over Tailscale, where the network layer provides identity.

Users who don't enable inbound MCP have zero MCP code in the request path. The dependency is compiled in but dormant.

### MCP tools share data shape with the TUI

A constraint worth stating explicitly: **MCP tools produce the same view of state that the TUI shows.** When Claude Code asks "what tasks do I have today?" via the `tasks_list` tool with the focus filter applied, the result should be the same list, in the same order, with the same fields, that the user sees in the TUI's Today panel.

Why this matters: if the MCP tool returns a richer or different shape than the TUI, the agent answers in ways that contradict what the user sees on their screen. ("Claude says I have 5 tasks, my TUI shows 4 — which is right?") That's an immediate trust failure.

Practical implication: MCP tool implementations and TUI subscriptions both go through the same internal query API. The TUI's `today_panel_query()` and the MCP `tasks_list` tool with focus filter share one underlying function. Don't let them diverge.

## Database: rusqlite (not SeaORM)

- SQLite for all structured data: entries metadata, principal tags, free tags, event log, jobs, hooks, plugin state, sync metadata.
- `rusqlite` with the `bundled` feature so we ship a known SQLite version.
- WAL mode on. Single writer, many readers — fine for our concurrency profile.

**Why not SeaORM (or sqlx, or diesel)?** Considered and rejected for this project:

- Sync logic doesn't ORM well. Per-row hybrid logical clocks, tombstones, conflict markers, last-write-wins comparisons — all want hand-written SQL. cr-sqlite specifically operates at the SQL/extension layer, not the ORM layer.
- The schema isn't really relational. Mostly entries + tags + events + jobs; few joins, many indexed queries. ORMs shine for app-server domain models with rich relationships, which this isn't.
- Compile time and footprint matter for a daemon iterated on for years.
- We lose nothing important. Migrations via `refinery` or hand-rolled. Type safety via a thin repository module per domain.

If the schema ever grows past ~15 tables with real relationships, or if a second backend (Postgres) becomes a requirement, revisit — `sqlx` would be the natural middle path.

## Write serialization: the daemon owns entry mutations

A real concern surfaces once entries can live in two places (SQLite row + markdown file): two writers can collide. Example: the daemon updates an entry's status while the user simultaneously saves an edit to its markdown body in nvim. Without a clear policy, you get torn writes, dropped updates, or worse.

**The model: the daemon is the serializer for every entry.** Every write to an entry — from a plugin, from the verb router, or from an external markdown edit picked up by the `notify` watcher — goes through a per-entry mutex (or single-threaded actor) owned by the daemon. Operations are queued and applied in order.

- **Plugin writes**: handler returns a write intent → daemon acquires the entry's lock → applies the change → releases.
- **External markdown edits**: `notify` reports a file change → daemon acquires the entry's lock → re-parses frontmatter and body → writes back to SQLite → releases.
- **Concurrent edits**: when both happen, they serialize. Whoever acquires the lock first wins; the other waits and applies its change against the now-current state.

This is straightforward single-device. The multi-device version of this problem (two devices both serializing locally, then trying to reconcile) is part of the sync research problem flagged below.

## Shipping model: daemon binary + CLI binary, MCP embedded

Two binaries:

- **`<daemon>`** — the long-running background process. Hosts the local IPC server, the event bus, the job/hook scheduler, the SQLite layer, the plugin loader, and (when enabled) the MCP Streamable HTTP listener.
- **`<daemon>-cli`** — small companion binary. Talks to the daemon over local IPC. Used by users in the terminal, by scripts, and by the Voxtype integration wrapper. Starts fast, exits fast.

This is the model `docker`/`dockerd`, `git`, `systemctl`/`systemd` use. Familiar, easy to package, easy to invoke from anywhere.

MCP is **not** a separate binary. It's embedded in the daemon as one of the daemon's network surfaces (alongside the local socket and, eventually, the peer-sync surface). Reason: MCP tools are thin wrappers around the daemon's internal verb router, so running MCP out-of-process would mean an extra IPC hop for every call. Embedded keeps the path direct.

Plugins remain out-of-process (subprocesses spawned and managed by the daemon, per earlier section).

### CLI invocation rules: TTY-aware dual mode

`<daemon>-cli` has three invocation paths and must distinguish them safely:

1. **`<daemon>-cli <subcommand> ...`** — explicit subcommand, one-shot CLI mode. Always.
2. **`<daemon>-cli`** (no args, stdin is a TTY) — launch the TUI.
3. **`<daemon>-cli`** (no args, stdin is *not* a TTY) — treat piped stdin as input to `ingest`. Equivalent to `<daemon>-cli ingest "$(cat)"`. This is what makes `echo "idea — explore CRDTs" | <daemon>-cli` do the right thing, and what lets scripts pipe transcripts in without the CLI trying to hijack the terminal.

Detection uses `std::io::IsTerminal` on stdin. If you really want to launch the TUI while stdin is piped (rare, weird), use the explicit `<daemon>-cli tui` subcommand, which always launches the TUI regardless of stdin.

This rule prevents a common dual-mode footgun where piped input either hangs (waiting for keyboard) or worse, gets fed as keystrokes into the TUI.

## Markdown files: on disk, watched by `notify`

Promoted entries (long-form notes, learning docs, daily logs, etc.) live as `.md` files in a configured directory. The daemon uses the `notify` crate to detect external edits and re-index metadata into SQLite. Edit in nvim, Obsidian, anywhere — the daemon picks it up.

## Plugin model: subprocesses speaking JSON-RPC

Plugins run as separate processes managed by the daemon. They communicate over stdio (JSON-RPC) or a per-plugin UDS socket. Rationale:

- **Isolation.** A misbehaving plugin can't crash the daemon.
- **Language-agnostic.** Plugins can be written in any language with JSON support.
- **Hot-reload.** Restart a plugin without restarting the daemon.
- **MCP convergence.** MCP servers are already subprocesses speaking JSON-RPC over stdio. A plugin and an MCP server are nearly the same shape — plugins can double as MCP servers, reusable in Claude Code or any other MCP client. This is a real platform win.

In-process Rust plugins (as crates compiled in) stay possible for core plugins where performance matters, but the *default* model is out-of-process.

### Plugin tiers: system vs. external

The product doc introduces a distinction the tech model needs to enforce:

- **System plugins** define the platform's vocabulary. They live in the daemon's repo, version-lock to the daemon, ship together. Examples: capture, tasks, focus, notes, triage, breakdown, search, daily-log. Breaking changes here are daemon-version-bumps.
- **External plugins** extend the system. Separate versioning, separate repos, independent release cadence. Examples: crypto, news, learning, ai-router, weather, soccer.

Same JSON-RPC protocol, same manifest format. The daemon doesn't *technically* enforce the distinction at runtime — it's a governance pattern, not a permission boundary. But in practice: a system plugin's manifest is loaded from the bundled plugins directory at startup; external plugins are loaded from the user plugins directory (`~/.local/share/<name>/plugins/`). Two directories, same loader.

### Plugin manifest via `--manifest` flag

When the daemon loads a plugin, it invokes the plugin binary with `--manifest`. The plugin prints a JSON manifest to stdout and exits. The daemon parses the manifest and stores it. Then the daemon spawns the plugin for real (without the flag) and starts the JSON-RPC conversation.

Why this rather than a parallel TOML or YAML file alongside the binary:
- **Manifest can never drift from the binary.** Build the plugin, ship the binary — the manifest is whatever that binary prints. No "I forgot to update the .toml" failures.
- **Compile-time generation.** The plugin SDK can derive the manifest from `#[verb]` and `#[event]` attribute macros at compile time, so authors don't write it by hand.
- **Versioning is trivial.** The plugin's own version comes from `CARGO_PKG_VERSION` (or equivalent in other languages) — automatic.

### Plugin governance and the event/data contracts

Some constraints the daemon should enforce (or document for plugin authors):

- **Events from plugins are namespaced.** A plugin named `tasks` can only emit events under `tasks.*`. The daemon refuses unprefixed or foreign-prefixed event names. This makes the global event vocabulary stay coherent.
- **Plugins own their own entry kinds.** The `tasks` plugin owns `EntryKind::Todo`. Other plugins can read these entries, can subscribe to their events, but can't mutate them directly — mutations route through the owning plugin's verbs.
- **Verbs are unique.** The daemon refuses to register two plugins claiming the same verb name. Resolution: rename one, or future versions introduce namespacing.
- **Plugin API version.** The manifest declares a daemon API version. If the daemon version is incompatible, the plugin is refused with a clear error. Prevents subtle protocol drift breaking things silently.

These rules cost nothing in the simple case and prevent the platform vocabulary from fragmenting as third-party plugins arrive.

## Event bus: causality, idempotency, recursion guards

The event bus is in-process pub/sub: plugins publish events on namespaced topics, subscribers receive them, hooks fire on patterns. Conceptually simple. Operationally, it's the system component most likely to grow unpredictable behavior, so a few rules need to be locked in before phase 7 builds it.

**Every event carries causality metadata:**

- `event_id` — UUIDv7, unique per event
- `parent_event_id` — the event that *caused* this one (null for user-initiated events)
- `origin` — the verb invocation or job that started this causal chain
- `chain_depth` — count of hops from the origin (incremented per hop)
- `chain_id` — same value across all events in one causal chain; useful for tracing

This means every event knows where it came from. The action log shows not just "event X happened" but "event X was caused by event Y, which was caused by user verb Z."

**Recursion guards.** A hook firing in response to event X can publish event Y, which can trigger another hook, and so on. To prevent runaway loops:

- A hard `chain_depth` cap (e.g. 5). Events beyond the cap are dropped with a logged error.
- Per-chain idempotency: a hook receiving an event whose `chain_id` it has already handled (within a configurable window, e.g. 60 seconds) skips re-firing. Keyed on `(hook_id, chain_id)`.

**Per-hook idempotency keys.** Some hooks are inherently idempotent (mark task done → write to log: writing twice is harmless). Others aren't (mark task done → ping external API: pinging twice causes a problem). The hook manifest declares an `idempotency_key` expression — a derivation from event fields. Before firing, the daemon checks "has this hook already fired with this key recently?" and skips if so. Default behavior: every event is unique (no dedup) unless the hook opts in.

**Delivery semantics.** Events are at-most-once and fire-and-forget. Subscribers have bounded channels; on full, the event is dropped with a warning. The daemon never blocks publishers. Critical paths (the action log, sync log) get unbounded channels backed by SQLite — those *cannot* drop. Slow/optional subscribers are bounded.

**Replay (sync v2).** When events sync across devices, the receiving device replays them — but with the causality model, replay can detect "I've already processed this chain" via `chain_id` and skip. This is the basis for hook-safe sync replay. Spec to be finalized when sync ships.

**Action log subscribes to `*`.** The action log records every event with its causality metadata. Self-logging and analytics flow from this. The log is append-only and never participates in causality (the log can't trigger hooks).

These rules are cheap to implement (causality fields are just five extra columns on the event record; recursion guard is a depth check; idempotency is a small hash table per hook). They prevent a whole class of bugs that would otherwise emerge in phase 7 and only manifest under real load.

## Identity: device IDs and cross-device UUIDs

A foundational decision for a multi-device product. Locked in from day one because retrofitting is painful.

**Every entry has a UUID, generated locally, unique across devices.** Use **UUIDv7** — time-ordered (good for database locality and natural sorting), 128-bit (collision-free across devices without coordination), standard. Not UUIDv4 (random, no ordering). Not autoincrement integers (would collide across devices).

**Every daemon instance has a stable device ID.**
- Generated at first run, stored in local config, never changes.
- Stored as a UUID; the user sets a human-readable label (`laptop`, `desktop`, `homelab`) for display.
- The UUID is what gets recorded in entries; the label is presentation-only.

**Every entry records two device IDs:**
- `created_on_device` — set at creation, never changes. Useful for "show me everything I captured on my phone," for audit, and for understanding origins.
- `last_modified_on_device` — updated on every write. Used in sync conflict resolution and for "who touched this last."

**Action log entries also get device IDs.** Self-logging analytics can then answer "where do I capture most ideas?" (phone in the evenings, desktop during work hours) without extra plumbing.

**Write ordering for sync: hybrid logical clocks (HLC).** When two devices edit the same entry offline, last-write-wins needs a defensible total ordering even with clock skew. An HLC is a tuple of (physical timestamp, logical counter, device ID), updated on every write and compared lexicographically. cr-sqlite handles this natively; if rolling our own sync layer, it's encoded as a column.

## Sync (post-MVP, research-grade complexity)

**This section is a sketch, not a settled plan.** Multi-device sync is the hardest engineering area in the system. The product-doc framing ("offline-first, peer-to-peer, devices reconcile") is correct as direction, but the actual semantics — conflict resolution, plugin state replay, hook re-firing, tombstone lifecycle — are research-grade. This section captures the *intended shape*; the real work happens after MVP and will surface decisions we can't fully make from the desk.

Three layers, each handled with the simplest tool that fits:

1. **Markdown files: external sync.** Syncthing, git, or Obsidian Sync — user's choice. The daemon doesn't sync prose files itself; it watches and re-indexes. Zero sync code to write.
2. **SQLite: peer-to-peer reconciliation.** Per-row sync metadata (hybrid logical clocks for ordering, last-write-wins with explicit conflict surfacing). `cr-sqlite` is the leading candidate — it's exactly this shape and proven. Vendoring or wrapping it is acceptable.
3. **Transport between peers: HTTP + SSE over Tailscale.** Tailscale's Local API gives identity for free — no token database, no credential rotation. If a user doesn't run Tailscale, the daemon can fall back to a manual peer config, but Tailscale is the recommended path.

**Open problems that this sketch does not yet answer.** These need explicit specification before any sync code is written:

- **Authoritative mutation ordering between SQLite and markdown.** Device A edits the markdown body via nvim. Device B updates metadata via the daemon. SQLite and the file disagree. Which side wins, per-field? The current direction: per-entry locking with the daemon as serializer, but the cross-device version of this is unresolved.
- **Plugin state sync.** Plugins own state (focus's current project, crypto's last-fetched-prices, breakdown's pending sessions). Some of it should sync (focus state across devices); some shouldn't (per-device crypto fetch history). Plugins need a way to declare per-state-type sync policy.
- **Hook replay semantics.** Device A marks a task done while offline. The task.done hook fires locally, sends a notification, updates the daily-log. Device A then syncs to Device B. Should the hook fire again on B? If the daily-log already has the entry from sync, double-firing duplicates. Idempotency keys per hook invocation are likely the answer; spec needed.
- **Tombstone lifecycle.** Deletions need to propagate. When are tombstones safe to garbage-collect?
- **Conflict surfacing UX.** When LWW genuinely loses information (two simultaneous edits to different fields of the same entry), the user sees... what? A merge UI? A conflict entry? A notification? Undefined.

These are not blockers for MVP because MVP is single-device. They are blockers for v2, and the v2 design phase will look more like a research project than a build phase.

**Not event sourcing.** The action log we keep for self-logging is *not* the source of truth. SQLite state tables are. The sync log is a separate, sync-only change feed used for reconciliation. Event sourcing as the primary model is a larger commitment than this design needs.

**Not libp2p (yet).** Powerful, but heavy. Tailscale + HTTP gives us a private mesh, identity, and NAT traversal without a new dependency tree. libp2p stays an option for the day non-Tailscale users need P2P.

## Voice input: a separate companion tool, not part of the daemon

Voice input is intentionally outside the daemon. The daemon accepts text from any source; getting from speech to text is a different concern with mature open-source solutions already.

**Recommended companion on Linux and macOS: [Voxtype](https://voxtype.io/).** Push-to-talk, fully local, ships Whisper plus seven ONNX engines (Parakeet, Moonshine, SenseVoice, Paraformer, Dolphin, Omnilingual, Cohere), handles the Wayland keybind problem via evdev or compositor bindings, and supports several output modes that map cleanly onto how the daemon wants to receive input. Linux: any desktop, Wayland-optimized. macOS: Apple Silicon, 13+. Windows is not supported by Voxtype — see below.

### Two voice routing routes (both supported, user picks per keybind)

We ship example wrapper scripts for both, and a sample Hyprland config showing both keybinds in parallel.

**Route A — Direct routing (no LLM in the loop).**

Voxtype keybind triggers `record start` / `record stop`. Voxtype's `post_process_command` is set to a wrapper script that pipes the transcript into the daemon CLI:

```bash
# voxtype-to-daemon.sh
exec <daemon>-cli ingest --source voice "$(cat)"
```

The daemon parses the verb out of the first word(s) and acts. Fast, deterministic, offline-friendly, zero LLM cost. Requires the user to speak the verb grammar (`idea for pro-rails — …`, `todo today — …`).

**Route B — LLM-routed via opencode (or any MCP-aware CLI agent).**

Voxtype keybind triggers a different wrapper that pipes the transcript to a CLI agent connected to the daemon over MCP Streamable HTTP:

```bash
# voxtype-to-opencode.sh
exec opencode --mcp-config ~/.config/<daemon>/opencode-mcp.json "$(cat)"
```

The agent reads the transcript, sees the daemon's tools (verbs) via MCP, and chooses which to call based on natural language. Slower, requires LLM availability (local or cloud), but accepts loose phrasing ("oh by the way I should refactor the auth thing for the rails project sometime this week").

The MCP config file just points opencode (or claude-code, or any MCP-aware agent) at the daemon's local MCP endpoint.

**Mapping to Voxtype profiles.** Voxtype profiles let you bind different `post_process_command` per keybind. The shipped example config wires:

- `super + v` → profile `direct` → Route A script
- `super + shift + v` → profile `assistant` → Route B script

Users pick per-utterance: tight verb grammar for speed, loose natural language when they don't want to think about syntax.

This is also why **inbound MCP earns its keep before any in-daemon LLM code is written.** Route B doesn't add a single line of LLM-related code to the daemon — it's just users pointing their existing AI agent at the MCP endpoint.

### Windows

Voxtype is Linux + macOS only. On Windows, voice input falls back to the OS dictation (Win+H) or a third-party tool (Talon Voice, Wispr Flow, etc.). The integration story is identical: whatever tool the user picks types text into the daemon's CLI or a watched file. Windows users without a voice tool simply use CLI and widget — voice was never a hard requirement.

## Widgets: out-of-process, OS-specific

The daemon doesn't own rendering. Widgets are separate processes that subscribe to the daemon's event stream and render themselves.

- **Linux/Hyprland**: `eww` is the leading candidate. Custom Wayland clients possible later.
- **macOS**: SwiftUI menubar app or a Tauri shell. Deferred.

The contract between daemon and widget is: subscribe to topics, receive events, render. Same as any other client.

## Auth: filesystem perms in v1, capability tokens later

- Local clients: UDS filesystem permissions. Only your user can talk to the daemon.
- Phone/remote clients (post-MVP): Tailscale identity. The daemon trusts the network layer.
- Capability tokens (macaroons/biscuits): when there's a real need for scoped, attenuated access — third-party widgets, shared devices, etc. Not v1.

## Configuration: TOML files in `$XDG_CONFIG_HOME`

Daemon config, plugin configs, keybinds, widget definitions — all TOML. Hot-reloadable where it makes sense (keybinds, widget definitions); restart-required for the rest. No GUI settings panel in MVP.

## Cross-platform priorities

- **Primary target:** Linux (CachyOS, Hyprland) — this is what's being built and used daily.
- **Equal targets:** macOS and Windows — designed for from day one, even if MVP testing happens on Linux. Tech choices below explicitly avoid Linux-only dependencies.

Concrete cross-platform implications baked into the choices:
- **IPC**: `interprocess::local_socket` instead of raw `tokio::net::UnixListener`. Abstracts Unix Domain Sockets (Linux/macOS) and Named Pipes (Windows) behind one API.
- **File watching**: `notify` works across all three (inotify, FSEvents, ReadDirectoryChangesW).
- **Config paths**: `directories` or `dirs` crate, not hardcoded `$XDG_CONFIG_HOME`. Returns the right path per OS.
- **Daemon lifecycle** (run-on-login): systemd user units / launchd / Windows startup — out of scope for MVP, but the design accommodates all three.
- **Voice input**: external companion tool. Voxtype on Linux and macOS; OS dictation or third-party tools on Windows. See the Voice section below.

## Architecture map

```
┌────────────┐  ┌────────────┐  ┌──────────────────────┐
│  CLI tool  │  │  Widget(s) │  │  Voxtype (external)  │
│            │  │            │  │  voice → text → CLI  │
└─────┬──────┘  └─────┬──────┘  └──────────┬───────────┘
      │ IPC/JSON      │ IPC+push           │ CLI invocation
      └───────┬───────┴────────┬───────────┘
              │                │
        ┌─────▼────────────────▼─────┐
        │       Daemon (Rust)         │
        │  - verb router              │
        │  - event bus                │
        │  - job/hook scheduler       │
        │  - SQLite (rusqlite)        │
        │  - notify file watcher      │
        └────┬───────────────┬────────┘
             │ stdio/JSON-RPC│ filesystem
             │               │
    ┌────────▼──────┐  ┌─────▼──────┐
    │   Plugins     │  │  Markdown  │
    │  (subprocs)   │  │   files    │
    │  may double   │  └─────┬──────┘
    │   as MCP      │        │ Syncthing/git/etc.
    │   servers     │        ▼
    └───────────────┘   ┌──────────────┐
                        │   Peer       │
                        │   devices    │
                        └──────────────┘
              ▲
              │ MCP Streamable HTTP over Tailscale (post-MVP)
              │
    ┌─────────┴────────┐
    │  Peer daemons    │
    │  (SQLite sync)   │
    └──────────────────┘
```

## What's deliberately deferred

- Plugin sandboxing (WASM, seccomp) — subprocess isolation is enough for v1
- Encryption at rest — filesystem encryption (LUKS, FileVault) covers this
- Multi-user — not a goal
- A GUI configurator — TOML is fine
- Mobile native apps — bot/PWA is the path
- Performance optimization beyond "fast enough" — measure first

## Open technical questions

- cr-sqlite vs. rolling our own sync metadata layer (and the broader sync research problem flagged above)
- Plugin discovery: scan a directory + invoke `--manifest` flag on each binary (current direction). Open: how to express enable/disable without renaming files — a small enabled-plugins config list, probably.
- macOS daemon lifecycle (launchd plist generation) — straightforward but needs writing
- Windows daemon lifecycle (Service vs. startup entry vs. just user-launched) — pick the simplest acceptable path
- Whether Route A and Route B share a single Voxtype profile-modifier setup or two separate keybinds in the example config (probably two separate, but worth testing)
- Whether opencode is the right "default" LLM agent for the Route B example, or claude-code, or something simpler — pick one for the install guide, mention the others
- FTS5 maintenance: a `<name>-cli search rebuild` command to pause triggers and rebuild the index. Needed for the future sync bulk-write case; ship in phase 8 so it exists when v2 needs it.
- The post-MVP confirmation evolution (confidence-based auto-commit with undo) — what does "confidence" mean concretely for rule-matched voice input, and what's the undo window?
