#!/bin/bash
set -euo pipefail

BINARY="${1:-./target/release/runie}"

run_once() {
    local attempt="${1:-1}"
    SESSION="runie_onboarding_$$_${attempt}"
    LOG="/tmp/runie_onboarding_$$_${attempt}.log"
    TMUX="tmux -L runie_onboarding_$$_${attempt} -f /dev/null"
    TMP_HOME="/tmp/runie_onboarding_home_$$_${attempt}"

    rm -rf "$TMP_HOME"
    mkdir -p "$TMP_HOME/.runie"

    cleanup() {
        $TMUX kill-session -t "$SESSION" 2>/dev/null || true
        rm -f "$LOG"
        rm -rf "$TMP_HOME"
    }
    trap cleanup EXIT

    wait_for_text() {
        local text="$1"
        local attempts="${2:-40}"
        for _ in $(seq 1 "$attempts"); do
            $TMUX capture-pane -t "$SESSION" -p > "$LOG" 2>/dev/null || true
            if grep -Eq "$text" "$LOG"; then
                return 0
            fi
            sleep 0.25
        done
        return 1
    }

    $TMUX new-session -d -s "$SESSION" -x 80 -y 24 "env HOME=$TMP_HOME $BINARY"
    # Give tmux time to start the pane before the first capture.
    for _ in $(seq 1 20); do
        if $TMUX list-panes -t "$SESSION" >/dev/null 2>&1; then
            break
        fi
        sleep 0.1
    done

    if ! wait_for_text "Choose a provider"; then
        return 1
    fi

    # Escape on a non-closable first-run dialog should not close it.
    $TMUX send-keys -t "$SESSION" Escape
    sleep 0.5

    if ! wait_for_text "Choose a provider" 10; then
        return 1
    fi

    # Begin the onboarding flow: filter to MiniMax, select it, enter a key, and submit.
    $TMUX send-keys -t "$SESSION" "minimax"
    sleep 0.5
    $TMUX send-keys -t "$SESSION" Enter
    sleep 0.5
    $TMUX send-keys -t "$SESSION" "sk-test"
    $TMUX send-keys -t "$SESSION" Enter

    # Without a real validation hook or RUNIE_MOCK, the flow reaches the validating
    # panel and eventually reports a validation failure. We only assert that the UI
    # remains responsive and does not panic during onboarding.
    if ! wait_for_text "Verifying MiniMax|Could not verify key|Login to MiniMax|MiniMax API key" 20; then
        return 1
    fi

    if grep -qiE "panic|thread.*panicked" "$LOG"; then
        return 1
    fi

    $TMUX send-keys -t "$SESSION" C-c
    sleep 1

    if grep -qiE "panic|thread.*panicked" "$LOG"; then
        return 1
    fi

    if grep -qE '[0-9]{4}\.[0-9]s' "$LOG"; then
        return 1
    fi

    return 0
}

if run_once 1; then
    echo "Onboarding smoke test passed"
    exit 0
fi

echo "Retrying onboarding smoke test..."
if run_once 2; then
    echo "Onboarding smoke test passed (on retry)"
    exit 0
fi

echo "ERROR: onboarding smoke test failed"
exit 1
