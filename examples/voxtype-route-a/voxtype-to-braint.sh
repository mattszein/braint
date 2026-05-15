#!/usr/bin/env bash
# Route A: pipe Voxtype transcript directly into braint as a voice ingest.
# braint parses the verb from the transcript (idea/todo/note/capture).
# On success, a desktop notification shows: "idea added: <body>".
# On unknown verb, a notification shows the error.
#
# Voxtype passes the transcript via stdin (post_process_command contract).
# Script outputs nothing so voxtype types nothing into the focused window.
# Logs to $XDG_RUNTIME_DIR/braint-voxtype.log

set -euo pipefail

DBG="/tmp/braint-debug.log"
echo "--- $(date -Iseconds) ---" >> "$DBG"
echo "PATH=$PATH" >> "$DBG"
echo "HOME=$HOME" >> "$DBG"
echo "XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR:-unset}" >> "$DBG"

# Ensure user binaries are in PATH (non-interactive shell has minimal PATH).
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH"
echo "PATH after=$PATH" >> "$DBG"

LOG_FILE="${XDG_RUNTIME_DIR:-/tmp}/braint-voxtype.log"

TRANSCRIPT="$(cat)"
echo "TRANSCRIPT=[$TRANSCRIPT]" >> "$DBG"

if [[ -z "$TRANSCRIPT" ]]; then
    echo "empty transcript, exit" >> "$DBG"
    exit 0
fi

BRAINT="$(which braint 2>&1 || echo 'not found')"
echo "braint=$BRAINT" >> "$DBG"

braint ingest --source voice "$TRANSCRIPT" >> "$DBG" 2>&1
echo "braint exit=$?" >> "$DBG"
