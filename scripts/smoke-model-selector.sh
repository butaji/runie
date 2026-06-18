#!/bin/bash
set -euo pipefail

BINARY="${1:-./target/release/runie}"
SESSION="runie_model_selector_$$"
LOG="/tmp/runie_model_selector_$$.log"
TMUX="tmux -L runie_model_selector_$$ -f /dev/null"
HOME_DIR="/tmp/runie_model_selector_home_$$"

trap '$TMUX kill-session -t "$SESSION" 2>/dev/null || true; rm -rf "$HOME_DIR" "$LOG"' EXIT

mkdir -p "$HOME_DIR/.runie"
cat > "$HOME_DIR/.runie/config.toml" <<'TOML'
[model_providers.mock]
base_url = "http://test"
api_key = "testkey"
models = ["echo", "gpt-4o-mini"]
TOML

$TMUX new-session -d -s "$SESSION" -x 80 -y 24 "HOME=$HOME_DIR RUNIE_MOCK=1 $BINARY"
sleep 2

# Open model selector
$TMUX send-keys -t "$SESSION" "/model"
$TMUX send-keys -t "$SESSION" Enter
sleep 1
$TMUX capture-pane -t "$SESSION" -p >> "$LOG"

# Filter to a specific model
$TMUX send-keys -t "$SESSION" "gpt-4o-mini"
sleep 0.5
$TMUX capture-pane -t "$SESSION" -p >> "$LOG"

# Select first match
$TMUX send-keys -t "$SESSION" Enter
sleep 1
$TMUX capture-pane -t "$SESSION" -p >> "$LOG"

# Quit
$TMUX send-keys -t "$SESSION" C-c
sleep 1

if grep -qiE "panic|thread.*panicked" "$LOG"; then
    echo "ERROR: panic detected in model selector smoke log"
    exit 1
fi

if grep -qE '[0-9]{4}\.[0-9]s' "$LOG"; then
    echo "ERROR: stuck timer detected"
    exit 1
fi

if ! grep -q "gpt-4o-mini" "$LOG"; then
    echo "ERROR: model selector did not render filter results"
    exit 1
fi

echo "Model selector smoke test passed"
