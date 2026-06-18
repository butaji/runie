#!/bin/bash
set -euo pipefail

BINARY="${1:-./target/release/runie}"
SESSION="runie_session_io_$$"
LOG="/tmp/runie_session_io_$$.log"
TMUX="tmux -L runie_session_io_$$ -f /dev/null"
HOME_DIR="/tmp/runie_session_io_home_$$"
SESSIONS_DIR="$HOME_DIR/sessions"

trap '$TMUX kill-session -t "$SESSION" 2>/dev/null || true; rm -rf "$HOME_DIR" "$LOG"' EXIT

mkdir -p "$HOME_DIR/.runie"
cat > "$HOME_DIR/.runie/config.toml" <<'TOML'
[model_providers.mock]
base_url = "http://test"
api_key = "testkey"
models = ["echo"]
TOML
mkdir -p "$SESSIONS_DIR"

$TMUX new-session -d -s "$SESSION" -x 80 -y 24 "HOME=$HOME_DIR RUNIE_SESSIONS_DIR=$SESSIONS_DIR RUNIE_MOCK=1 $BINARY"
sleep 2

# Save current session as "foo"
$TMUX send-keys -t "$SESSION" "/"
sleep 0.3
$TMUX send-keys -t "$SESSION" "save"
sleep 0.3
$TMUX send-keys -t "$SESSION" Enter
sleep 0.5
$TMUX send-keys -t "$SESSION" "foo"
sleep 0.2
$TMUX send-keys -t "$SESSION" Enter
sleep 1
$TMUX capture-pane -t "$SESSION" -p >> "$LOG"

# Send a message to change the session
$TMUX send-keys -t "$SESSION" "hello session io"
$TMUX send-keys -t "$SESSION" Enter
sleep 2
$TMUX capture-pane -t "$SESSION" -p >> "$LOG"

# Load the saved session
$TMUX send-keys -t "$SESSION" "/"
sleep 0.3
$TMUX send-keys -t "$SESSION" "load"
sleep 0.3
$TMUX send-keys -t "$SESSION" Enter
sleep 0.5
$TMUX send-keys -t "$SESSION" "foo"
sleep 0.2
$TMUX send-keys -t "$SESSION" Enter
sleep 1
$TMUX capture-pane -t "$SESSION" -p >> "$LOG"

# Delete the saved session
$TMUX send-keys -t "$SESSION" "/"
sleep 0.3
$TMUX send-keys -t "$SESSION" "delete"
sleep 0.3
$TMUX send-keys -t "$SESSION" Enter
sleep 0.5
$TMUX send-keys -t "$SESSION" "foo"
sleep 0.2
$TMUX send-keys -t "$SESSION" Enter
sleep 1
$TMUX capture-pane -t "$SESSION" -p >> "$LOG"

# Quit
$TMUX send-keys -t "$SESSION" C-c
sleep 1

if grep -qiE "panic|thread.*panicked" "$LOG"; then
    echo "ERROR: panic detected in session IO smoke log"
    exit 1
fi

if grep -qE '[0-9]{4}\.[0-9]s' "$LOG"; then
    echo "ERROR: stuck timer detected"
    exit 1
fi

if ! grep -q "Session 'foo' saved\|Session 'foo' loaded\|Session 'foo' deleted" "$LOG"; then
    echo "ERROR: session command confirmations did not render"
    exit 1
fi

echo "Session IO smoke test passed"
