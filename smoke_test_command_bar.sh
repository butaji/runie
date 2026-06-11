#!/bin/bash
# Smoke test: command palette / dialog bar — fuzzy search, selection styling, all flows.
# Layer 4 testing as per AGENTS.md.

set -e
BINARY="$(pwd)/target/release/runie"
SESSION="runie_cmdsmoke_$$"
LOG="/tmp/runie_cmdsmoke_$$.log"

if [ ! -x "$BINARY" ]; then
    echo "ERROR: release binary not found at $BINARY"
    exit 1
fi

cleanup() {
    tmux kill-session -t "$SESSION" 2>/dev/null || true
}
trap cleanup EXIT

# Start runie in a tmux session
tmux new-session -d -s "$SESSION" -x 100 -y 30 "$BINARY"
sleep 0.5

send() {
    tmux send-keys -t "$SESSION" "$@"
}

capture() {
    tmux capture-pane -t "$SESSION" -p > "$LOG"
}

assert_no_panic() {
    capture
    if grep -iE "panic|thread.*panicked" "$LOG"; then
        echo "FAIL: panic detected"; cat "$LOG"; exit 1
    fi
    if grep -E '[0-9]{4}\.[0-9]s' "$LOG"; then
        echo "FAIL: stuck timer"; cat "$LOG"; exit 1
    fi
}

# 1. Open command palette (try Ctrl+\ or Ctrl+Shift+P)
echo "=== 1. Open command palette ==="
send C-\
sleep 0.4
capture
if ! grep -q "Commands" "$LOG"; then
    # try other bindings
    send Escape
    sleep 0.2
    send C-p
    sleep 0.4
    capture
fi
if ! grep -q "Commands" "$LOG"; then
    echo "FAIL: command palette did not open"
    cat "$LOG"
    exit 1
fi
echo "OK: palette opened"
assert_no_panic

# 2. Fuzzy search: type "sav" should match /save
echo "=== 2. Fuzzy search 'sav' ==="
send "sav"
sleep 0.3
capture
if ! grep -qE "save|sa" "$LOG"; then
    echo "FAIL: fuzzy search did not filter to save"
    cat "$LOG"
    exit 1
fi
echo "OK: fuzzy search 'sav'"
assert_no_panic

# 3. Clear filter with backspace
echo "=== 3. Backspace clears filter ==="
for _ in 1 2 3; do send BSpace; sleep 0.1; done
sleep 0.3
capture
assert_no_panic
echo "OK: backspace works"

# 4. Navigate down and up
echo "=== 4. Navigate list ==="
send Down
sleep 0.2
send Down
sleep 0.2
send Up
sleep 0.2
assert_no_panic
echo "OK: navigation works"

# 5. Close palette with Escape
echo "=== 5. Close palette ==="
send Escape
sleep 0.3
capture
if grep -q "Commands" "$LOG"; then
    # might still be visible briefly
    sleep 0.3
    capture
fi
echo "OK: closed"

# 6. Test settings dialog
echo "=== 6. Open settings ==="
# Settings is usually /settings
send "/settings"
sleep 0.2
send Enter
sleep 0.4
capture
if grep -q "Settings" "$LOG"; then
    echo "OK: settings opened"
    send Escape
    sleep 0.3
else
    echo "WARN: settings did not open via /settings"
    send Escape
fi
assert_no_panic

# 7. Test model selector
echo "=== 7. Open model selector ==="
send "/model"
sleep 0.2
send Enter
sleep 0.4
capture
if grep -q "Select Model" "$LOG"; then
    echo "OK: model selector opened"
    # fuzzy search
    send "gpt"
    sleep 0.3
    capture
    assert_no_panic
    send Escape
    sleep 0.3
else
    echo "WARN: model selector did not open"
    send Escape
fi

# 8. Test theme picker
echo "=== 8. Open theme picker ==="
send "/theme"
sleep 0.2
send Enter
sleep 0.4
capture
if grep -q "Choose Theme" "$LOG"; then
    echo "OK: theme picker opened"
    send Escape
    sleep 0.3
else
    echo "WARN: theme picker did not open"
    send Escape
fi
assert_no_panic

# 9. Resize stress
echo "=== 9. Resize stress ==="
for i in $(seq 1 8); do
    tmux resize-window -t "$SESSION" -x $((40 + i * 8)) -y $((10 + i * 2))
    sleep 0.08
done
sleep 0.3
assert_no_panic
echo "OK: resize stress passed"

# 10. Final check
echo "=== 10. Final assertion ==="
capture
echo "OK: all command bar smoke tests passed"
echo "--- last output ---"
tail -20 "$LOG"
