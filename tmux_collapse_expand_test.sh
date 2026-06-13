#!/bin/bash
set -euo pipefail

# Smoke test: Ctrl+O collapses and expands tool/thought posts in the feed.
# Runs the real binary inside tmux, feeds keys, captures pane output, and
# asserts the feed actually toggles between expanded and collapsed states.

BINARY="$(pwd)/target/release/runie"
SESSION="runie_collapse_$$"
LOG="/tmp/runie_collapse_$$.log"
HOME_TMP="/tmp/runie_collapse_home_$$"

rm -rf "$HOME_TMP"
mkdir -p "$HOME_TMP"
export HOME="$HOME_TMP"
# Use mock provider so the app starts at the prompt without login dialog.
export RUNIE_MOCK=1

trap 'tmux kill-session -t "$SESSION" 2>/dev/null || true; rm -rf "$HOME_TMP" "$LOG"' EXIT

# Build release binary if missing.
if [[ ! -x "$BINARY" ]]; then
    cargo build --release -p runie-term
fi

tmux new-session -d -s "$SESSION" -x 80 -y 24 "$BINARY"
sleep 0.5

# Helper: paste text without triggering slash-command shortcuts.
paste_text() {
    printf '\e[200~%s\e[201~' "$1" | tmux load-buffer -
    tmux paste-buffer -t "$SESSION"
}

# Submit a prompt that triggers a tool call, producing a multi-line tool post.
paste_text "list files"
tmux send-keys -t "$SESSION" Enter
sleep 2.0

# Capture expanded state.
tmux capture-pane -t "$SESSION" -p > "$LOG"

# Verify the tool output is visible before collapse.
if ! grep -E 'run \(.*\)|\.git \(.*\)|\.hermes \(.*\)' "$LOG" >/dev/null; then
    echo "PRE-COLLAPSE: expected tool file list not found"
    tail -20 "$LOG"
    exit 1
fi

# Collapse feed posts with Ctrl+O.
tmux send-keys -t "$SESSION" C-o
sleep 0.5
tmux capture-pane -t "$SESSION" -p > "${LOG}.collapsed"

# Verify collapsed state hides the file list behind a [+] summary.
if grep -E 'run \(.*\)|\.git \(.*\)|\.hermes \(.*\)' "${LOG}.collapsed" >/dev/null; then
    echo "COLLAPSED: file list should be hidden"
    tail -20 "${LOG}.collapsed"
    exit 1
fi
if ! grep -E '\[\+\]' "${LOG}.collapsed" >/dev/null; then
    echo "COLLAPSED: expected [+] summary indicator"
    tail -20 "${LOG}.collapsed"
    exit 1
fi

# Expand again with Ctrl+O.
tmux send-keys -t "$SESSION" C-o
sleep 0.5
tmux capture-pane -t "$SESSION" -p > "${LOG}.expanded"

# Verify expanded state shows the file list again.
if ! grep -E 'run \(.*\)|\.git \(.*\)|\.hermes \(.*\)' "${LOG}.expanded" >/dev/null; then
    echo "RE-EXPANDED: expected tool file list to reappear"
    tail -20 "${LOG}.expanded"
    exit 1
fi

# Quit cleanly.
tmux send-keys -t "$SESSION" C-c
sleep 0.3

# Assert no stuck timers (elapsed > 1000s means infinite loop).
if grep -E '[0-9]{4}\.[0-9]s' "$LOG" "${LOG}.collapsed" "${LOG}.expanded"; then
    echo "STUCK TIMER!"
    exit 1
fi

# Assert no panics.
if grep -i "panic\|thread.*panicked" "$LOG" "${LOG}.collapsed" "${LOG}.expanded"; then
    echo "PANIC!"
    exit 1
fi

echo "Tmux collapse/expand smoke test passed"
