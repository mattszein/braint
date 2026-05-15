# Route A — Voxtype → braint (direct verb grammar)

Push-to-talk voice capture. Hold `Super+V`, speak a verb phrase, release.
braint parses the verb directly (no AI). A desktop notification confirms the captured entry.

## Prerequisites

- `braint` binary in `$PATH` (built with `cargo build --release`)
- Voxtype installed and running as a systemd user service (`systemctl --user status voxtype`)
- A Whisper model downloaded (e.g. `base.en`)
- `notify-send` (ships with libnotify)
- Mako notification daemon running

## Setup

**1. Copy the wrapper script**

```bash
cp voxtype-to-braint.sh ~/.local/bin/
chmod +x ~/.local/bin/voxtype-to-braint.sh
```

**2. Add the profile to `~/.config/voxtype/config.toml`**

Use the **absolute path** — the voxtype daemon runs as a systemd service with minimal PATH.

```toml
[profiles.braintd]
post_process_command = "/home/YOU/.local/bin/voxtype-to-braint.sh"
output_mode = "clipboard"
```

Replace `YOU` with your username.

**3. Restart the voxtype service**

Config is read by the daemon. Changes don't take effect until restart:

```bash
systemctl --user restart voxtype
```

**4. Add Hyprland keybinds**

```
bind  = SUPER, V, exec, voxtype record start --profile braintd
bindr = SUPER, V, exec, voxtype record stop
```

Reload: `hyprctl reload`.

**5. Ensure braintd is running**

```bash
braintd &
# or add to your Hyprland autostart
```

## Usage

1. Hold `Super+V` and speak: `"idea try cr-sqlite for sync"`
2. Release. Voxtype transcribes, pipes transcript to the script.
3. braint parses the verb and commits immediately.
4. Notification appears: `"idea added: try cr-sqlite for sync"`

Entry is immediately visible in the TUI (no confirmation step).

## Verb grammar

Leading/trailing punctuation and casing on the verb are ignored (voice transcription often adds them).

| Phrase | Kind |
|--------|------|
| `idea ...` | Idea |
| `todo ...` | Todo |
| `note ...` | Note |
| `capture ...` | Capture |

Examples that all work: `"Idea."`, `"IDEA"`, `"idea,"`, `"idea -"`

## Troubleshooting

**Script never runs / profile not applied**
Check the voxtype service logs:
```bash
journalctl --user -u voxtype -f
```
Look for `INFO Using profile override: braintd`. If missing, the profile name in the keybind doesn't match the config.

**`braint` not found (exit code 127)**
The service PATH is minimal. The script exports `~/.cargo/bin` and `~/.local/bin`, but if `braint` is elsewhere use the absolute path inside the script.

**Nothing in log / script not called**
Always use the absolute path to the script in `post_process_command`. Relative names fail silently.

**Transcript gets pasted into focused window**
Ensure `output_mode = "clipboard"` is set in the profile and voxtype was restarted after the config change.

**Keybind not firing**
```bash
hyprctl binds | grep -i voxtype
```

**Wrong microphone**
```bash
pactl list short sources
```

**braintd not running**
Script logs errors to `$XDG_RUNTIME_DIR/braint-voxtype.log`:
```bash
tail -f "$XDG_RUNTIME_DIR/braint-voxtype.log"
```
