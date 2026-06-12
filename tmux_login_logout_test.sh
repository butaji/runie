#!/bin/bash
set -euo pipefail

BINARY="$(pwd)/target/release/runie"
SESSION="runie_login_logout_$$"
LOG="/tmp/runie_login_logout_$$.log"
HOME_TMP="/tmp/runie_login_logout_home_$$"

rm -rf "$HOME_TMP"
mkdir -p "$HOME_TMP"
export HOME="$HOME_TMP"
# Use mock provider so the app starts at the prompt instead of auto-opening
# the login dialog; this lets us exercise the direct /login and /logout form
# commands without navigating the guided dialog first.
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

# /login direct command: should write provider to config.toml and confirm.
paste_text "/login minimax sk-test"
tmux send-keys -t "$SESSION" Enter
sleep 0.5

# /logout direct command: should remove provider from config.toml and confirm.
paste_text "/logout minimax"
tmux send-keys -t "$SESSION" Enter
sleep 0.5

# /logout with no providers configured should show the empty message.
paste_text "/logout"
tmux send-keys -t "$SESSION" Enter
sleep 0.5

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

# Assert expected messages appeared.
if ! grep -q "Logged in to 'minimax'" "$LOG"; then
    echo "Missing login confirmation"; exit 1
fi
if ! grep -q "Logged out from 'minimax'" "$LOG"; then
    echo "Missing logout confirmation"; exit 1
fi
if ! grep -q "No providers configured" "$LOG"; then
    echo "Missing empty providers message"; exit 1
fi

# Assert config.toml reflects the add/remove cycle.
CONFIG="$HOME_TMP/.runie/config.toml"
if [[ -f "$CONFIG" ]] && grep -q "minimax" "$CONFIG"; then
    echo "Provider should have been removed from config.toml"; exit 1
fi

echo "Tmux login/logout smoke test passed"
