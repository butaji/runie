#!/bin/bash
set -euo pipefail

BINARY="$(pwd)/target/release/runie"
SESSION="runie_providers_$$"
LOG="/tmp/runie_providers_$$.log"
HOME_TMP="/tmp/runie_providers_home_$$"

rm -rf "$HOME_TMP"
mkdir -p "$HOME_TMP"
export HOME="$HOME_TMP"
# Use mock provider so the app starts at the prompt instead of auto-opening
# the login dialog.
export RUNIE_MOCK=1

trap 'tmux kill-session -t "$SESSION" 2>/dev/null || true; rm -rf "$HOME_TMP" "$LOG"' EXIT

tmux new-session -d -s "$SESSION" -x 80 -y 24 "$BINARY"
sleep 0.5

# Helper: paste text into the tmux pane as a bracketed-paste so it lands in
# the input field without triggering the '/' -> palette shortcut.
paste_text() {
    printf '\e[200~%s\e[201~' "$1" | tmux load-buffer -
    tmux paste-buffer -t "$SESSION"
}

# /providers command: should open the providers dialog.
paste_text "/providers"
tmux send-keys -t "$SESSION" Enter
sleep 0.5

# The providers dialog should open.
# Press Escape to close it.
tmux send-keys -t "$SESSION" Escape
sleep 0.3

# /provider alias: should also open the providers dialog.
paste_text "/provider"
tmux send-keys -t "$SESSION" Enter
sleep 0.5

# Press Escape to close it.
tmux send-keys -t "$SESSION" Escape
sleep 0.3

# Capture and check
tmux capture-pane -t "$SESSION" -p > "$LOG"
tmux send-keys -t "$SESSION" C-c

# Assert no stuck timers or panics.
if grep -E '[0-9]{4}\.[0-9]s' "$LOG"; then
    echo "STUCK TIMER!"; exit 1
fi
if grep -i "panic\|thread.*panicked" "$LOG"; then
    echo "PANIC!"; exit 1
fi

echo "Tmux providers smoke test passed"
