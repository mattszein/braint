# Personal Daemon — MVP

## Goal of the MVP

Prove that the platform works end-to-end on a single device, with enough plugins to be actually useful for one user (me) for a week. Every architectural layer is exercised, but nothing is over-built.

## Demo: what the user can do after install

After installing the MVP (daemon + voice companion tool):

- Hit your Voxtype keybind, dictate `idea — explore CRDTs for sync` and see it land in scratch
- Hit the keybind and dictate `idea for pro-rails — refactor the menu bar` and see it land directly in the pro-rails project, no triage needed
- Run `<name>-cli` with no arguments to open the **TUI inspector**: today's tasks, scratch, focus, notes, search, live events — the canonical way to interact with the daemon when not dictating
- Type `<name>-cli todo project:pro-rails when:today priority:high — finish auth refactor` for one-shot CLI use; the entry appears in the TUI immediately
- Trigger breakdown on a task (TUI key, or voice, or CLI) and walk through the 3–5 step decomposition flow
- Triage scratch into projects via the TUI's triage modal
- Set focus to a project and watch every panel filter — except `scope:always` items like rest reminders, which still surface
- Search anything with `/` in the TUI (live fuzzy filter) or `<name>-cli find --json` for scripting
- Get an end-of-day digest summarizing captures, completions, and untriaged scratch
- Glance at a crypto widget on a configured monitor/workspace (or the same data in the TUI)

That's the demo. Small surface, but every layer of the platform is real.

## In scope — Core

- Text-ingest API on the CLI for any source (typed by user, piped from voice companion tool, written to a watched file)
- Widget input (typed)
- Voice input via companion tool ([Voxtype](https://voxtype.io/) on Linux/macOS) — documented integration recipe, not in-daemon STT
- Verb grammar shared across CLI and voice-via-companion (LLM-routed verbs deferred to post-MVP)
- Rule-based intent routing (word-keys → plugin verbs)
- Confirmation policy: voice-sourced input confirms, CLI doesn't
- Unified data model: projects + principal tags + free tags; prose bodies on disk as markdown; structured data in SQLite
- Search across entries (text + tags + type + project)
- Event system: plugins emit and listen on named topics
- Jobs (scheduled) and hooks (event-driven) as plugin primitives
- Notification system with three shapes (interrupt, ambient, digest)
- Plugin loader
- **TUI inspector** built into the CLI binary (dual-mode: no args = TUI, with args = one-shot CLI) — the canonical interactive surface, ratatui-based, works on Linux/macOS/Windows and over SSH

## In scope — Starter plugins

Plugins ship in two tiers: **system plugins** version-lock to the daemon and define the platform's vocabulary; **external plugins** evolve independently. The MVP ships 8 system plugins and 1 external plugin (crypto) as the canonical external-plugin reference.

**System plugins** (define the vocabulary):
1. **capture** — voice/CLI/widget input → scratch (or project if context provided)
2. **tasks** — verbs (`todo`, `done`, `drop`, `block`), states, priority, scope, `when:today`, `repeat:`, subtasks as markdown checkboxes
3. **notes** — markdown bodies, per-project
4. **focus** — set/clear current project, emits events; `scope:always` items always surface
5. **breakdown** — executive-function aid for decomposing a goal into 3–5 concrete steps
6. **triage** — process scratch into projects
7. **daily-log** — end-of-day auto-summary (driven by a job)
8. **search** — text + tag + type + project filters, voice "find X" verb

**External plugin** (proves the third-party model):
9. **crypto** — per-asset config, threshold notifications, glance widget. Reference implementation external plugin authors copy from.

## Out of scope (roadmap, post-MVP)

Listed roughly in priority order:

1. **Multi-device sync** — the next major milestone after MVP. The data model is designed peer-to-peer from day one, but actual reconciliation ships as v2. Optional always-on role (homelab as job runner / rendezvous) ships with sync.
2. **Phone capture** — thin client (Telegram/Signal bot, later PWA) once sync exists.
3. **AI router plugin** — LLM fallback for natural-language input; exposes verbs and context via MCP.
4. **Learning plugin** — topic-oriented, generates docs, tracks progress, surfaces review prompts.
5. **News plugin** — RSS feeds, per-interest notification.
6. **Calendar plugin** — read-only Google/CalDAV display.
7. **Analytics digest** — weekly self-introspection from the action log.
8. **AI-assisted variants** of existing verbs (LLM-suggested breakdown steps, auto-tagging, auto-triage suggestions).

Naming these explicitly so they don't sneak into MVP scope.

## MVP non-goals

- Not multi-user
- Not mobile (yet)
- Not synced across devices yet — MVP is single-device, but the data model is designed so sync can ship as v2 without rework
- Not LLM-dependent — the LLM is a future plugin, never a hard dependency. Voice routing in MVP is rule-based only.
- Not bundling voice — voice input is via Voxtype (or any other tool); the daemon doesn't do STT. MVP ships an integration recipe, not an engine.
- Not a polished GUI app — widgets are minimal, deliberate surfaces

## Build sequence

Each step is demoable on its own.

1. **Skeleton.** CLI ingest + scratch + one widget. End-to-end pipe working with one verb (`capture` or `idea`).
2. **Verb grammar.** Multiple verbs, principal tags, confirmation flow.
3. **Voice via Voxtype.** Document and test the integration recipe (CLI pipe / post_process_command / file watcher). Confirmation flow tuned for voice-sourced input.
4. **Content plugins.** Tasks (with states, priority, `when:today`, subtasks) + notes + search.
5. **Structure plugins.** Focus + triage + breakdown.
6. **State + observability.** Event system + jobs + hooks + daily-log.
7. **Proof of plugin model.** Crypto plugin as the external-facing example: "anyone can build a plugin."
8. **Release.** Polish, docs, name, open source.

## Open questions to lock before building

- **Name.** Affects binaries, sockets, config paths, brand. Last big decision before phase 0.
- **Default keybinds, default confirmation policies, default stale-scratch policy.** Ship with sensible defaults so users aren't configuring on day one.
- **What the TUI's default layout looks like in detail.** The sketch in the build plan is loose; the actual layout — panel widths, what's visible at startup, key bindings table — will be refined as you build.
- **Whether `opencode` is the right "default" LLM agent** for the Route B example, or claude-code, or something simpler.

Now resolved by the tech and build plan docs (kept here for reference):
- ~~Minimum plugin manifest~~ → declared via `--manifest` flag on the plugin binary; manifest auto-generated from `#[verb]`/`#[on_event]` macros in `plugin-sdk`.
- ~~Principal-tag prefixes~~ → governed by the rule in the idea doc (becomes principal only when core or system plugins query for it).

## Success criteria

The MVP succeeds if, after a week of single-user daily use:

- Capture has never failed
- The user prefers dictating ideas to the daemon over writing them in another tool
- The TUI inspector is open in a tmux pane or terminal tab most of the working day
- The end-of-day digest is something the user actually reads
- The breakdown verb has been used on a real task and helped it actually move
- Scratch has been triaged at least a few times without it feeling like a chore
- Adding a new plugin feels obvious enough that a contributor could try
