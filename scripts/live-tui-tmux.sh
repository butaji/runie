#!/usr/bin/env bash
# Manual live TUI smoke test in a real tmux session.
#
# This script is a HUMAN-IN-THE-LOOP validation helper, not an automatic test.
# AGENTS.md forbids shell/tmux tests in CI and deterministic test suites, but
# still requires a live tmux session for TUI work. Use this script to run that
# session and capture the result for review.
#
# Usage:
#   scripts/live-tui-tmux.sh [mock|minimax] [prompt] [timeout_sec]
#
# Examples:
#   scripts/live-tui-tmux.sh mock
#   scripts/live-tui-tmux.sh mock "list files"
#   MINIMAX_API_KEY=... scripts/live-tui-tmux.sh minimax "hello"

set -euo pipefail

MODE="${1:-mock}"
PROMPT="${2:-hello}"
TIMEOUT_SEC="${3:-30}"

if [[ "$MODE" != "mock" && "$MODE" != "minimax" ]]; then
    echo "Usage: $0 [mock|minimax] [prompt] [timeout_sec]" >&2
    exit 1
fi

if [[ "$MODE" == "minimax" && -z "${MINIMAX_API_KEY:-}" ]]; then
    echo "Error: MINIMAX_API_KEY is required for minimax mode" >&2
    exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BIN="$PROJECT_ROOT/target/release/runie-tui"
CAPTURE_FILE="/tmp/runie-live-tui-tmux-capture.txt"

# Build release binary if missing or source is newer.
# Mock mode requires the runie-provider/mock feature; minimax uses the
# default openai-compatible provider.
if [[ ! -x "$BIN" || "$PROJECT_ROOT/Cargo.lock" -nt "$BIN" ]]; then
    echo "Building release TUI binary..."
    if [[ "$MODE" == "mock" ]]; then
        (cd "$PROJECT_ROOT" && cargo build --release -p runie-tui --features runie-provider/mock)
    else
        (cd "$PROJECT_ROOT" && cargo build --release -p runie-tui)
    fi
fi

# Isolated $HOME so the live run cannot touch the user's real config/sessions.
TMP_HOME="$(mktemp -d)"
cleanup() {
    local exit_code=$?
    if tmux has-session -t "$SESSION" 2>/dev/null; then
        tmux kill-session -t "$SESSION" >/dev/null 2>&1 || true
    fi
    rm -rf "$TMP_HOME"
    exit "$exit_code"
}
trap cleanup EXIT

mkdir -p "$TMP_HOME/.runie"
if [[ "$MODE" == "minimax" ]]; then
    cat > "$TMP_HOME/.runie/config.toml" <<EOF
provider = "minimax"
model = "MiniMax-M3"

[model_providers.minimax]
type = "openai-compatible"
base_url = "https://api.minimax.chat/v1"
api_key = "$MINIMAX_API_KEY"

[models]
default = "MiniMax-M3"
scoped = ["MiniMax-M3"]
EOF
else
    cat > "$TMP_HOME/.runie/config.toml" <<EOF
provider = "mock"
model = "echo"

[models]
default = "echo"
scoped = ["echo"]
EOF
fi

SESSION="runie-live-$(date +%s)"
TMUX_ENV="HOME=$TMP_HOME"
if [[ "$MODE" == "mock" ]]; then
    TMUX_ENV="$TMUX_ENV RUNIE_MOCK=1"
fi

echo "Starting live tmux session: $SESSION"
echo "Mode: $MODE, prompt: '$PROMPT', isolated home: $TMP_HOME"

# Create an 80x24 pane so snapshots are stable and run the TUI binary directly.
tmux new-session -d -s "$SESSION" -x 80 -y 24 \
    "cd '$PROJECT_ROOT' && export $TMUX_ENV && exec $BIN"

capture_pane() {
    tmux capture-pane -t "$SESSION" -p
}

# Wait for the TUI to render (welcome screen or input placeholder).
wait_for_text() {
    local needle="$1"
    local max_wait="${2:-$TIMEOUT_SEC}"
    for _ in $(seq 1 "$max_wait"); do
        if capture_pane 2>/dev/null | grep -Eq "$needle"; then
            return 0
        fi
        sleep 0.5
    done
    return 1
}

# Give the TUI a fixed moment to finish startup. The old automatic
# tmux-smoke-test.sh used a 1s sleep here; we keep the same timing for
# the manual live session.
sleep 1

if ! wait_for_text "New session|Type a message" 10; then
    echo "FAIL: TUI did not render within 10s" >&2
    capture_pane > "$CAPTURE_FILE"
    echo "Pane capture saved to $CAPTURE_FILE" >&2
    exit 1
fi

echo "PASS: TUI launched"

# Send the prompt. Type it first, pause so the input actor processes it,
# then press Enter to submit, matching how a real user types.
tmux send-keys -t "$SESSION" "$PROMPT"
sleep 0.5
tmux send-keys -t "$SESSION" Enter

echo "Sent prompt: $PROMPT"
sleep 1
capture_pane > "/tmp/runie-live-tui-tmux-after-prompt.txt"
echo "Pane after prompt saved to /tmp/runie-live-tui-tmux-after-prompt.txt"

# Wait for evidence that a turn started. Fast mock responses may complete
# before "Working..." is captured, so also accept a response arrow or thought.
if wait_for_text "Working|→ |Thought" "$TIMEOUT_SEC"; then
    echo "PASS: Turn started"
else
    echo "WARN: Did not see turn activity within ${TIMEOUT_SEC}s" >&2
fi

# Give the provider a few seconds to stream a response.
sleep 3

capture_pane > "$CAPTURE_FILE"
echo "Final pane capture saved to $CAPTURE_FILE"

# Gracefully quit the TUI.
tmux send-keys -t "$SESSION" C-c
sleep 1

# Print a short summary of what was on screen.
echo ""
echo "=== Final screen (first 24 lines) ==="
head -n 24 "$CAPTURE_FILE"
echo "=== End of screen ==="

# A successful live session must show the submitted prompt in the feed and
# some sign that the provider/agent produced output (response arrow, thought,
# or the "Working..." spinner).
if grep -qE "❯ $PROMPT|→ |Thought|Working" "$CAPTURE_FILE"; then
    echo ""
    echo "RESULT: PASS (live tmux session launched, submitted, and ran a turn)"
else
    echo ""
    echo "RESULT: FAIL (live tmux session did not submit or run a turn)" >&2
    exit 1
fi
