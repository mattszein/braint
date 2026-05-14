# Personal Daemon — Product Idea

## The Idea

A background daemon that lives on your machines, captures what you tell it — by voice, CLI, or widget — runs tasks on your behalf, and shows glanceable info through configurable widgets. Everything is a plugin. Everything you capture and every action it takes is stored, tagged, and searchable. Offline-first, single user, syncs across your own devices, open source.

Think of it as a personal cognitive assistant that sits between you and your computer — closer to a system service than an app.

**Who it's for.** Terminal-first, multi-device users who value owning their data, work across several machines, and don't want their thinking trapped inside someone else's app.

**What it isn't.** Not Obsidian (not a notes app — though notes are one plugin). Not Raycast or Alfred (not a launcher). Not Home Assistant (not for IoT). Not Siri or Alexa (not cloud-bound, not closed, and not voice-only — typing is a first-class path). The closest mental model is a personal Jarvis, but reachable equally by voice, terminal, or widget: ambient, modular, yours.

## The Core Loop

Everything else in this document is decoration around one loop:

```
capture  →  scratch  →  focus-aware triage  →  next actionable step  →  daily review
   ↑                                                                          │
   └──────────────────────────────────────────────────────────────────────────┘
```

You speak, type, or click a thought — it lands. Untriaged stuff sits in scratch. You triage it into projects with focus-aware defaults. Triaged items become next-actionable steps. At the end of the day, you review what happened. Tomorrow, repeat.

If this loop becomes a daily habit, the product has won. Crypto widgets, news feeds, third-party plugins, voice routing through LLMs — all of it is downstream. The daemon's moat is **becoming indispensable to one user's daily cognition.** Everything else exists to serve that.

## Why?

Most AI tools today are synchronous: open the app, type, wait, read. They don't accumulate your data over time, they don't run in the background, and they don't integrate with how you already work (keybinds, multiple monitors, terminal-first).

This project flips that:

- **Multi-surface input.** Voice, CLI, and widget are equal first-class paths. Use whichever fits the moment — speak when your hands are busy, type when you want precision, click when you want a visual surface. Voice is provided by a companion tool ([Voxtype](https://voxtype.io/) on Linux and macOS) that feeds transcribed text into the daemon, not built into the daemon itself; users can swap or skip the voice tool without affecting the rest. The same verb grammar runs everywhere.
- **Async by default.** You delegate, it works, it notifies. Never blocks you.
- **Your data, your machine.** It accumulates *your* stuff over time and stays yours.
- **Local by default.** Nothing leaves your machines unless a plugin explicitly needs the network and you've opted in. Cloud LLMs, calendar sync, news fetching — all opt-in, per plugin.
- **Background-native.** No main window. Widgets are optional surfaces, fully configured by you (which monitor, which workspace, which widgets).
- **Offline-first, sync when online.** Capture must never fail, even with no internet and no local LLM. When connected, your devices reconcile in the background.
- **Same brain across devices.** Desktop, laptop, homelab — same data, same plugins, different widgets per device if you want.

## What It Solves

- The "lose the thought before I can write it down" problem
- The "I want to research X but I'm in the middle of something" problem
- The "I have notes/tasks/ideas scattered across 10 tools" problem
- The "I want a personal AI but not to send everything to a cloud" problem
- The "I want my own data, my own rules, my own pace" problem
- The "I captured this on the desktop, now I'm on the laptop" problem
- The "I know what I want to do but I can't figure out where to start" problem (planning paralysis, common in neurodivergent users — addressed by the breakdown verb)
- The "I'm hyper-focused on a project and still need life to nudge me" problem (focus as a lens, not a wall)

## Example Flow

You're coding. You hit a keybind. A small widget appears on the current workspace.

You dictate: *"learn Zig — get the basics with simple examples, then advanced topics."*

The daemon:
1. The companion voice tool (Voxtype) transcribes the audio locally and pipes the text to the daemon's CLI.
2. The first word — `learn` — matches a verb contributed by the `learning` plugin. The rest is the body. No project mentioned, so it defaults to current focus (or scratch if none).
3. Because this is voice direct-match, it asks for a quick confirmation: *"Start learning topic 'Zig'?"*
4. You confirm. The widget disappears.
5. In the background, the plugin uses a model (local or cloud) to generate a structured markdown doc with basics, examples, and advanced topics. It's stored as a learning-topic entry with tags `topic:zig lang:zig`.
6. When done, a notification fires: *"Zig learning doc ready — 1200 words, 4 sources."*
7. If you had no internet, the task would have queued. When you reconnect, it runs.

A few minutes later, already in your terminal, you type: `/todo project:pro-rails when:today priority:high — finish auth refactor`. No confirmation, no voice — typed input is precise and trusted. It appears in your today widget immediately.

You have an idea about a code project. This time you use voice again, hands still on the keyboard: *"idea for pro-rails — refactor the menu bar with a more modern style."* The verb is `idea`, the project is `pro-rails`, the rest is body. It lands directly in pro-rails as an idea — no triage needed.

Then a thought you're not sure about. You say: *"capture — we should explore CRDTs for sync."* (You could just as well have typed `/capture — we should explore CRDTs for sync` — same result.) No project, no specific verb mapping. It lands in **scratch** with type `idea`, to be triaged later or assigned next time you think about it.

Meanwhile, on your third monitor, a crypto widget quietly updates BTC/USD every minute. A "today" widget shows the day's tasks filtered by focus, plus always-scope items like your 11am gym reminder. An end-of-day digest summarizes what you captured, what ran, and what's still in scratch.

Later that evening you open your laptop. The Zig doc is already there. So is the idea you captured an hour ago. Your devices reconciled in the background — same data, same plugins, but the laptop has its own widget layout.


## What It Covers

### Core (the daemon itself)
- Text-ingest API for any source (CLI, widget, voice companion tool, phone bot, etc.)
- Verb-based grammar shared across CLI, voice, and LLM routing
- Rule-based intent routing (word-keys → plugin verbs) with optional LLM routing as a fallback plugin
- Unified data model: entries organized by **project + type + tags**, with prose bodies on disk as markdown
- Search across entries (text, tags, type, project)
- Event system: plugins emit and listen on named topics
- **Jobs** (scheduled, time-based) and **hooks** (event-driven) as first-class plugin primitives
- Notification system with three shapes: interrupt, ambient, digest
- Plugin loader

### Projects as the backbone
A project is any **container of intent** — not just code. `pro-rails` is a project. `redecorate the living room` is a project. `learn Zig` is a project. `my day job` is a project. Everything else (tasks, notes, ideas, docs, learning material) hangs off a project. Captures with no project land in **scratch** until you triage them.

### Plugins (everything else)
Plugins are small mini-products that share the platform. Each plugin contributes some combination of:
- **Verbs** (`idea`, `todo`, `done`, `breakdown`, `learn`, `price`, ...) — the vocabulary users invoke via CLI, voice, or LLM
- **Widgets** — small renderable surfaces
- **Jobs** — scheduled background work
- **Hooks** — reactions to events from other plugins
- **Data types** — kinds of entries it owns
- **Notifications** — what it can emit and how

Plugins come in **two governance tiers** with the same technical contract:

- **System plugins** define the platform's vocabulary and ship with the daemon. They version-lock to it. Examples: capture, tasks, focus, search, triage, breakdown, notes, daily-log. These can't evolve independently — they *are* the user-facing language of the system. If the breakdown verb changes shape, the whole product changes.
- **External plugins** are everything else. Independent versioning. Anyone can write one. Examples: crypto, news, calendar, weather, soccer scores, reading queue. They extend the system but don't redefine its vocabulary.

Same plugin protocol, different release discipline. This keeps the product coherent while leaving the door open for an ecosystem.

Plugins (across both tiers) come in two technical flavors:

- **Feeds** — bring data in and display it. Read-only, scheduled, glanceable. Crypto, news, weather, calendar, soccer scores, stock prices.
- **Actions** — take input from you and do something with it. Capture, todos, notes, ideas, learning, research, triage, breakdown.

Both share the same primitives; they just feel different in use.

Examples of plugins, organized by tier:

**System plugins** (define the vocabulary, ship with the daemon, version-locked):
- **capture** — voice/CLI/widget input lands in scratch (or in a project if context was provided)
- **tasks** — add, complete, list, filter; supports states (open, in-progress, blocked, done, dropped), priority, scope, repeat, when/due
- **notes** — per-project, markdown bodies on disk
- **focus** — set/clear current project; emits events
- **breakdown** — executive-function aid for decomposing a goal into 3–5 concrete steps
- **triage** — process scratch into projects
- **search** — full-text and tag-filtered queries across all entries
- **daily-log** — end-of-day auto-summary

**External plugins** (independent versioning, anyone can write one):
- **learning** — topic-oriented; generates docs, tracks progress, surfaces review prompts
- **crypto** — per-asset configs, thresholds, notifications, glance widget
- **news** — RSS feeds, notification on matches
- **ai-router** — optional LLM fallback when rules don't match; exposes verbs and context via MCP
- (Anyone can build more: soccer scores, weather, reading queue, clipboard history, git status...)

### Widgets
Pure consumers of the event stream. They subscribe to topics and render. You decide which widgets exist, where they live, and what they listen to. A "today" widget might subscribe to `task.*` filtered by `focus.current_project` and re-render when focus changes. Widgets are optional — the daemon works fine with none.

## Key Design Decisions

**Projects are the backbone; principal tags are the structure.** A project is any container of intent (code, life, learning, work). Around projects, entries are organized by a small set of **principal tags** the system knows about — `project:`, `type:`, `status:`, `priority:`, `when:`, `due:`, `scope:`, `repeat:` — plus free-form tags for everything else (`rust`, `urgent`, `read-later`). One tag system, two tiers: principals drive widgets and queries; free tags describe.

**Principal-tag governance.** New principal tags will try to appear over time (`energy:`, `urgency:`, `estimate:`, `area:`, `mood:`, `blocked-by:`...). The rule: **a tag becomes principal only when core or system plugins start querying for its values across multiple contexts.** Until then, it stays free-form. So `energy:low` is a free tag until something — a widget, a query, a system plugin — needs to filter by energy level; *then* it gets promoted. This keeps the query model from fragmenting. External plugins can define their own prefixed namespaces (`crypto:asset:btc`, `weather:location:bsas`) for their internal use without polluting the principal namespace.

**Areas, contexts, and other axes are free tags.** Projects are the *primary* organizing axis but not the only legitimate one. Some people think in areas (`area:health`), contexts (`context:laptop`), energy levels, or time horizons. These are valid and natively supported — they're just free tags that you can filter on. The system doesn't elevate them to principal status unless your queries start needing them.

**Entries can be multi-tagged across projects.** A task to "research insulation materials" can belong to both `redecorate-living-room` and `nexaworld` if it genuinely serves both. The system treats `project:` as a multi-valued principal tag, not a single-parent folder.

**Storage: a row in SQLite, a markdown file when there's a body worth reading.** Every entry has a row in SQLite for its metadata — project, type, status, tags, timestamps, pointers, plugin state. An entry *also* gets a markdown file on disk when it has a body worth reading or editing separately. The rule is "promotion by need, not by type":

| Entry | Has a body worth a file? | Where it lives |
|---|---|---|
| A quick captured idea ("explore CRDTs for sync") | No | SQLite only |
| A long-form note on pro-rails architecture | Yes | SQLite + markdown file |
| A short todo ("finish auth refactor") | No | SQLite only |
| A todo with detailed description or subtasks | Yes | SQLite + markdown file |
| A learning-topic doc (e.g. Zig) | Yes — that's the whole point | SQLite + markdown file |
| A crypto price reading | No | SQLite only |
| A daily-log summary | Yes | SQLite + markdown file |

A todo can start as a row and be promoted to a file the moment its description grows past a one-liner — same entry, just with a body file now. The user never has to think about this; they just write, and the system places the content. Concretely: the task "learn about Zig" is a SQLite row; the markdown doc the learning plugin produces *as a result* of that task is a separate entry, a file, linked back to the task. Two entries, one in each storage, connected by reference.

This also gives sync flexibility for free: the markdown half can ride on Syncthing, git, or even Obsidian Sync; the SQLite half syncs separately through the daemon. Edit a markdown file in any editor (nvim, Obsidian, whatever) and the daemon picks up the change. Uninstall the daemon and your prose entries are still a folder of readable markdown.

**Capture has a friction spectrum.** Three modes, user picks per capture:
- *Zero context:* `capture — explore CRDTs for sync` → lands in **scratch**, type idea, no project
- *Project context:* `idea for pro-rails — refactor the menu bar` → lands directly in pro-rails
- *Fully specified:* `/todo project:pro-rails priority:high when:today — finish auth refactor` → fully tagged at capture

The capture surface never gets in the way. Less context = more triage later. More context = no triage needed.

**Verbs are a first-class concept.** Plugins contribute verbs to a global vocabulary (`idea`, `todo`, `note`, `done`, `breakdown`, `learn`, `focus`, `triage`, `find`, `price`, ...). Each verb works across all three input modes: CLI (`/idea project:pro-rails ...`), voice grammar (`idea pro-rails ...`), and LLM routing (natural language, via MCP). A plugin defines a verb once and gets all three for free.

**Triage is a named verb, not a hidden chore.** Captures with no project land in scratch. Triage is the act of assigning a project, type, and intent to scratch items. It can happen at capture (provide context up front), later (widget, voice command "triage scratch", morning ritual), or be auto-suggested by the system based on context (focus, similar past captures, LLM analysis). Default behavior for stale scratch entries (e.g. archive after N days) is configurable — and is itself just a job a plugin runs.

**Rule-based routing first, LLM optional.** Word-key matching is the default. It's predictable, fast, offline, and debuggable. The LLM is a plugin (`ai-router`), not infrastructure. When enabled, it acts as a *natural-language adapter* on top of the same verb grammar — it sees available verbs, projects, and current context (via MCP) and translates loose-form input into a verb invocation. No hallucinated entities. Users who don't want it never enable it.

**Confirmation is a per-path policy:**

| Input | Routing | Default confirmation |
|---|---|---|
| CLI / typed | Explicit command | None |
| Voice | Direct rule match | Confirm |
| Voice | LLM routing | None or light |

Plugins can also declare action stakes; destructive actions always confirm regardless of settings.

Confirmation is the right MVP default for voice — STT is fuzzy, rule parsing is fuzzy, and the cost of a wrong commit is real. But voice workflows die from friction, and even one extra step can become annoying. The likely post-MVP evolution is **confidence-based auto-commit with undo** for trusted/repetitive verbs (rather than confirm-every-time forever). A high-confidence "todo for pro-rails — finish auth" should auto-commit with a brief undo window. Low-confidence or destructive verbs always confirm. Defer until the rule-based path has been used enough to see where friction actually bites.

**Task lifecycle is small but expressive.** A task has a status (`open`, `in-progress`, `blocked`, `done`, `dropped`), a priority, a scope (`project` or `always`), an optional `when:today` for explicit daily intent, an optional `due:` for real-world deadlines, and an optional `repeat:` directive for recurrences. Subtasks are markdown checkboxes in the body — progress is just `done/total`. A task that grows beyond 5 steps is offered promotion to a project.

**Hybrid priority: the system fills, you edit.** The "today" view loads automatically from routines, recurring tasks, items tagged `when:today`, and (suggested, not auto-added) items with `due:today`. You can override anything — move the gym from 11 to 12, drop a task off today, reorder — at any time. Overrides apply to today only and don't silently mutate the routine. If you want to change the routine itself, you change it explicitly.

**Focus is a lens, not a wall.** Setting focus on a project filters project-scoped widgets and defaults captures to that project. But entries tagged `scope:always` (rest reminders, health, calendar alerts, the daily-log nudge) always surface regardless of focus. This is what makes the assistant *trustworthy* — you can hyper-focus and still trust the system to nudge you to stand up, eat, or get to the meeting.

**Breakdown is a first-class executive-function verb.** Any task can be broken down into 3–5 small, concrete, executable steps with an explicit "done" criterion. The system surfaces only the next step by default; the rest are available on demand. The structure is enforced as a soft constraint — too many steps → "this looks like a project, want to promote it?"; vague steps → "what's the first 10-minute action inside that?". The verb works manually; an AI-assisted variant can suggest steps when the LLM plugin is enabled. This is an explicit accessibility feature for neurodivergent users and anyone prone to planning paralysis.

**Notifications have three shapes:**
- **Interrupts** — fire now (task done, error)
- **Ambient** — badge, color, count, no sound
- **Digests** — "here are the N things since you last checked" (morning, end-of-day, weekly)

Defaults are conservative. Everything is per-plugin configurable.

**Self-logging is a feature, not plumbing.** Every action the daemon takes — every job that ran, every hook that fired, every verb that was invoked — is itself an entry. Which means "analytics" is just queries on entries. *Which plugin runs most? When do I capture ideas? What's my scratch-to-action time? Which routines do I skip?* — all free.

**Jobs and hooks are how the system stays alive.** Jobs run on a schedule (fetch news hourly, generate digest nightly, prune stale scratch weekly). Hooks fire on events (task done → ping work API, idea captured with `urgent` → notify immediately, focus changed → update slack status). Both are first-class plugin primitives. Any user-visible behavior — even "warn me about untriaged scratch after 7 days" — is just a job or a hook a plugin registers.

**Sync across your own devices.** A personal daemon that only lives on one machine is half a product. Ideas happen on the laptop; the homelab is always on; the desktop is where you do the work. The daemon is designed to run on multiple machines you own and reconcile data between them in the background.

- Each device has the full data locally. Any device works fully offline.
- All devices are equal as sources of truth. There's no "master" — they reconcile peer-to-peer when they see each other (over your own network, or via something like Tailscale).
- *Optionally*, one device can take an **always-on role** — running scheduled jobs, hosting the inbound endpoint for phone capture, running heavy local LLM tasks, and acting as a rendezvous when peers can't see each other directly. A homelab is the natural fit; without one, jobs simply run on whatever device is awake.
- Widgets and plugin configuration are per-device (the third-monitor crypto widget lives on the desktop, not the laptop); underlying data is shared.
- Sync is invisible when it works. When it can't (no network, conflicting edit), the user sees an honest indicator — never silent data loss.

**Phone capture as a thin client.** The phone doesn't run the full daemon. It's a capture endpoint that posts into your daemon over your own network — most likely starting as a chat bot (Telegram/Signal) and later a small PWA. Captures land in scratch like anything else and sync from there.

**Open source, single user, ecosystem-friendly.** No multi-tenancy, no auth complexity, no SaaS. Core and plugins all open. Not a product to sell. The plugin contract (verbs, widgets, jobs, hooks, data types) is kept small and documented from day one so other people can build plugins for their own needs — soccer scores, weather, reading queue, whatever — without forking the core.

## A Note on Implementation

This is a product document; it deliberately doesn't pick a language, an IPC mechanism, a CRDT library, or a windowing strategy. The design *does* imply real engineering work — sync conflict resolution between SQLite and markdown across peers, a low-footprint long-running daemon, widget integration with the host window manager (Hyprland on Linux, the menubar on macOS), and integration with the chosen voice companion tool. Those tradeoffs belong in a separate technical doc. The point of this one is to fix the product shape before anything is built.

## Product North Star

> *"My computer remembers everything I tell it, does the boring work while I keep coding, and surfaces the right thing at the right time — even without internet, even across devices."*

If a feature ladders up to that sentence, it belongs. If not, it doesn't.
