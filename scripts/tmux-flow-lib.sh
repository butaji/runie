#!/bin/bash
# Reusable tmux flow helpers for runie black-box testing.
# Source this file from test scripts (do not execute directly).

set -uo pipefail

TMUX_FLOW_SOCKET="${TMUX_FLOW_SOCKET:-runie_flow_$$}"
TMUX="tmux -L $TMUX_FLOW_SOCKET -f /dev/null"
BINARY="${RUNIE_BINARY:-$(pwd)/target/release/runie}"
FLOW_LOG_DIR="${FLOW_LOG_DIR:-/tmp/runie_flow_logs_$$}"
FLOW_PASS=0
FLOW_FAIL=0

mkdir -p "$FLOW_LOG_DIR"

# Cleanup all sessions created with this socket.
flow_cleanup_all() {
    $TMUX list-sessions -F '#{session_name}' 2>/dev/null | while read -r s; do
        $TMUX kill-session -t "$s" 2>/dev/null || true
    done
    rm -rf "$FLOW_LOG_DIR"
}

flow_cleanup_session() {
    local session="$1"
    $TMUX kill-session -t "$session" 2>/dev/null || true
}

flow_pass() {
    echo "  ✓ $1"
    FLOW_PASS=$((FLOW_PASS + 1))
}

flow_fail() {
    echo "  ✗ FAIL: $1"
    FLOW_FAIL=$((FLOW_FAIL + 1))
}

# Start a fresh runie session in tmux.
# Usage: flow_start <session_name> [extra_env]
flow_start() {
    local session="$1"
    local extra_env="${2:-}"
    flow_cleanup_session "$session"
    sleep 0.2
    $TMUX new-session -d -s "$session" -x 80 -y 24 "env $extra_env $BINARY" 2>/dev/null
    sleep 0.4
    # Nudge the terminal size so crossterm emits a resize event and the app
    # renders its initial frame in a detached tmux session.
    $TMUX resize-window -t "$session" -x 81 -y 25 2>/dev/null || true
    sleep 0.1
    $TMUX resize-window -t "$session" -x 80 -y 24 2>/dev/null || true
    sleep 0.2
}

# Capture the current pane to a log file.
flow_capture() {
    local session="$1"
    local log="$2"
    $TMUX capture-pane -t "$session" -p > "$log" 2>/dev/null
}

# Wait until the pane contains the given grep-compatible pattern.
# Usage: flow_wait_for <session> <pattern> [max_attempts] [sleep_seconds]
flow_wait_for() {
    local session="$1"
    local pattern="$2"
    local attempts="${3:-40}"
    local delay="${4:-0.25}"
    local tmp_log="$FLOW_LOG_DIR/_wait_$$.log"
    for _ in $(seq 1 "$attempts"); do
        flow_capture "$session" "$tmp_log"
        if grep -Eq "$pattern" "$tmp_log" 2>/dev/null; then
            rm -f "$tmp_log"
            return 0
        fi
        sleep "$delay"
    done
    rm -f "$tmp_log"
    return 1
}

# Basic sanity checks every flow should pass.
flow_check_health() {
    local log="$1"
    local label="$2"
    if grep -qiE "panic|thread.*panicked|unreachable" "$log" 2>/dev/null; then
        flow_fail "$label: panic detected"
        return 1
    fi
    if grep -qE '[0-9]{4}\.[0-9]s' "$log" 2>/dev/null; then
        flow_fail "$label: stuck timer detected"
        return 1
    fi
    return 0
}

# Send a sequence of keys. Pass "Enter" for Return, "C-c" etc.
flow_send() {
    local session="$1"
    shift
    $TMUX send-keys -t "$session" "$@"
}

flow_resize() {
    local session="$1"
    local width="$2"
    local height="$3"
    $TMUX resize-window -t "$session" -x "$width" -y "$height" 2>/dev/null || true
}

# Submit a prompt and wait for a pattern (or a default idle signal).
flow_submit_and_wait() {
    local session="$1"
    local prompt="$2"
    local pattern="${3:-Turn completed}"
    local timeout="${4:-60}"
    flow_send "$session" "$prompt"
    flow_send "$session" Enter
    if ! flow_wait_for "$session" "$pattern" "$((timeout * 4))" 0.25; then
        return 1
    fi
    return 0
}

flow_summary() {
    echo ""
    echo "========================================"
    echo "  Results: $FLOW_PASS passed, $FLOW_FAIL failed"
    echo "========================================"
    if [ "$FLOW_FAIL" -gt 0 ]; then
        return 1
    fi
    return 0
}
