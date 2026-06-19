#!/bin/bash
# Fast UI-only tmux flows using the mock provider.
set -uo pipefail

cd "$(dirname "$0")/.."
source scripts/tmux-flow-lib.sh

FLOW_HOME="/tmp/runie_ui_flows_home_$$"
mkdir -p "$FLOW_HOME/.runie"
cat > "$FLOW_HOME/.runie/config.toml" <<'TOML'
provider = "mock"
[model_providers.mock]
base_url = "http://test"
api_key = "testkey"
models = ["echo"]
[models]
default = "echo"
TOML

export HOME="$FLOW_HOME"
export RUNIE_BINARY="$(pwd)/target/release/runie"
export RUNIE_SESSIONS_DIR="$FLOW_HOME/sessions"

cleanup() {
    flow_cleanup_all
    rm -rf "$FLOW_HOME"
}
trap cleanup EXIT

echo "========================================"
echo "  UI-Only Flow Suite"
echo "========================================"

SESSION="runie_ui_flows_$$"
LOG="$FLOW_LOG_DIR/flow.log"

run_flow() {
    local id="$1"
    local label="$2"
    local log="$FLOW_LOG_DIR/${id}.log"
    flow_capture "$SESSION" "$log"
    if flow_check_health "$log" "$label"; then
        flow_pass "$label"
    fi
}

# Dialog open/close flows
for dialog in "palette" "model" "settings"; do
    flow_start "$SESSION"
    case "$dialog" in
        palette) flow_send "$SESSION" C-p ;;
        model)   flow_send "$SESSION" C-l ;;
        settings)
            flow_send "$SESSION" C-p
            sleep 0.3
            flow_send "$SESSION" "settings"
            sleep 0.2
            flow_send "$SESSION" Enter
            ;;
    esac
    sleep 0.6
    flow_capture "$SESSION" "$LOG"
    if [ "$dialog" = "palette" ] && grep -q "Commands" "$LOG"; then
        run_flow "dlg_${dialog}_open" "Open $dialog dialog"
    elif [ "$dialog" = "model" ] && grep -q "Select Model" "$LOG"; then
        run_flow "dlg_${dialog}_open" "Open $dialog dialog"
    elif [ "$dialog" = "settings" ] && grep -q "Settings" "$LOG"; then
        run_flow "dlg_${dialog}_open" "Open $dialog dialog"
    else
        flow_fail "Open $dialog dialog"
    fi
    flow_send "$SESSION" Escape
    sleep 0.4
    run_flow "dlg_${dialog}_close" "Close $dialog dialog"
done

# Input editing flows
flow_start "$SESSION"
flow_send "$SESSION" "hello world"
flow_send "$SESSION" C-w
flow_send "$SESSION" Enter
sleep 0.5
run_flow "edit_delete_word" "Ctrl+W deletes word"

flow_start "$SESSION"
flow_send "$SESSION" "abcdef"
flow_send "$SESSION" C-a
flow_send "$SESSION" "X"
flow_send "$SESSION" C-e
flow_send "$SESSION" "Y"
flow_send "$SESSION" Enter
sleep 0.5
run_flow "edit_cursor_home_end" "Ctrl+A/E move cursor"

flow_start "$SESSION"
flow_send "$SESSION" "line1"
flow_send "$SESSION" C-j
flow_send "$SESSION" "line2"
flow_send "$SESSION" Enter
sleep 0.5
run_flow "edit_newline" "Ctrl+J inserts newline"

# Slash command flows
for cmd in "help" "trust" "untrust" "readonly" "new" "reset" "history"; do
    flow_start "$SESSION"
    flow_send "$SESSION" "/$cmd"
    sleep 0.3
    flow_send "$SESSION" Enter
    sleep 0.5
    run_flow "slash_$cmd" "/$cmd command"
done

# Session flows
flow_start "$SESSION"
flow_send "$SESSION" "test message"
flow_send "$SESSION" Enter
sleep 0.5
flow_send "$SESSION" "/save tmuxflow"
sleep 0.3
flow_send "$SESSION" Enter
sleep 0.5
flow_capture "$SESSION" "$LOG"
if grep -q "saved" "$LOG"; then
    run_flow "session_save" "Save session"
else
    flow_fail "Save session"
fi

flow_start "$SESSION"
flow_send "$SESSION" "/load tmuxflow"
sleep 0.3
flow_send "$SESSION" Enter
sleep 0.5
run_flow "session_load" "Load session form"

flow_start "$SESSION"
flow_send "$SESSION" "/sessions"
sleep 0.3
flow_send "$SESSION" Enter
sleep 0.5
run_flow "session_list" "List sessions"

# Trust/readonly flows
flow_start "$SESSION"
flow_send "$SESSION" "/trust"
sleep 0.3
flow_send "$SESSION" Enter
sleep 0.5
flow_capture "$SESSION" "$LOG"
if grep -q "trusted\|Trusted" "$LOG"; then
    run_flow "trust_cmd" "/trust marks project trusted"
else
    flow_fail "/trust marks project trusted"
fi

flow_start "$SESSION"
flow_send "$SESSION" "/untrust"
sleep 0.3
flow_send "$SESSION" Enter
sleep 0.5
flow_capture "$SESSION" "$LOG"
if grep -q "🔒\|RO" "$LOG"; then
    run_flow "untrust_cmd" "/untrust shows read-only"
else
    flow_fail "/untrust shows read-only"
fi

# Misc flows
flow_start "$SESSION"
flow_send "$SESSION" "hello"
flow_send "$SESSION" Enter
sleep 0.5
flow_send "$SESSION" C-e
sleep 0.3
run_flow "toggle_expand" "Toggle expand/collapse"

flow_start "$SESSION"
flow_send "$SESSION" "hello"
flow_send "$SESSION" Enter
sleep 0.5
flow_send "$SESSION" C-l
sleep 0.3
run_flow "collapse_all" "Ctrl+L collapse all"

flow_start "$SESSION"
flow_send "$SESSION" "hello"
flow_send "$SESSION" Enter
sleep 0.5
flow_resize "$SESSION" 20 5
sleep 0.3
run_flow "resize_small" "Resize to small window"

flow_start "$SESSION"
flow_send "$SESSION" "hello"
flow_send "$SESSION" Enter
sleep 0.5
flow_resize "$SESSION" 200 60
sleep 0.3
run_flow "resize_large" "Resize to large window"

flow_summary
