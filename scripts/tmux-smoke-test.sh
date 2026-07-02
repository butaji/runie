#!/usr/bin/env bash
set -euo pipefail

# TUI smoke tests using tmux.
# Usage:
#   scripts/tmux-smoke-test.sh [mock|minimax]
#
# Environment:
#   MINIMAX_API_KEY - required for minimax mode
#   RUNIE_BIN       - path to runie TUI binary (default: ./target/release/runie)

MODE="${1:-mock}"
RUNIE_BIN="${RUNIE_BIN:-./target/release/runie}"
SESSION="runie_smoke_$$"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TMP_HOME="$(mktemp -d)"
trap 'rm -rf "$TMP_HOME"; tmux kill-session -t "$SESSION" >/dev/null 2>&1 || true' EXIT

mkdir -p "$TMP_HOME/.runie"

cat > "$TMP_HOME/.runie/config.toml" <<'TOML'
provider = "mock"

[models]
default = "echo"

[model_providers.mock]
base_url = "http://mock"
api_key = ""
TOML

if [[ "$MODE" == "minimax" ]]; then
  if [[ -z "${MINIMAX_API_KEY:-}" ]]; then
    echo "ERROR: MINIMAX_API_KEY is required for minimax mode"
    exit 1
  fi
  cat > "$TMP_HOME/.runie/config.toml" <<TOML
provider = "minimax"

[models]
default = "minimax/text-01"

[model_providers.minimax]
base_url = "https://api.minimaxi.chat/v1"
api_key = "$MINIMAX_API_KEY"
TOML
fi

if [[ ! -x "$RUNIE_BIN" ]]; then
  echo "Building release TUI binary..."
  (cd "$PROJECT_ROOT" && cargo build --release -p runie-tui)
fi

run_tmux_scenario() {
  local name="$1"
  local input="$2"
  local expected="$3"
  local wait_seconds="${4:-5}"

  echo "--- Scenario: $name ---"

  tmux new-session -d -x 80 -y 24 -s "$SESSION" \
    "cd '$PROJECT_ROOT' && HOME='$TMP_HOME' RUST_LOG=warn RUNIE_MOCK=1 '$RUNIE_BIN' 2>'$TMP_HOME/${name}.err'"

  sleep 1

  # Type the user input and submit it.
  tmux send-keys -t "$SESSION" "$input"
  sleep 0.5
  tmux send-keys -t "$SESSION" Enter

  sleep "$wait_seconds"

  # Capture pane contents before quitting.
  tmux capture-pane -t "$SESSION" -p > "$TMP_HOME/${name}.txt"

  # Quit the TUI.
  tmux send-keys -t "$SESSION" q
  sleep 0.5
  tmux send-keys -t "$SESSION" C-c || true
  sleep 0.5
  tmux kill-session -t "$SESSION" >/dev/null 2>&1 || true

  if grep -q "$expected" "$TMP_HOME/${name}.txt"; then
    echo "PASS: found expected text '$expected'"
    return 0
  else
    echo "FAIL: did not find expected text '$expected'"
    echo "--- captured output ---"
    cat "$TMP_HOME/${name}.txt"
    echo "--- stderr ---"
    cat "$TMP_HOME/${name}.err" 2>/dev/null || true
    echo "--- end output ---"
    return 1
  fi
}

# Like run_tmux_scenario but quits during the wait (for testing Ctrl+C during active turn).
run_tmux_scenario_quit_during() {
  local name="$1"
  local input="$2"
  local expected="$3"
  local wait_before_quit="${4:-3}"

  echo "--- Scenario: $name ---"

  tmux new-session -d -x 80 -y 24 -s "$SESSION" \
    "cd '$PROJECT_ROOT' && HOME='$TMP_HOME' RUST_LOG=warn RUNIE_MOCK=1 '$RUNIE_BIN' 2>'$TMP_HOME/${name}.err'"

  sleep 1

  # Type the user input and submit it.
  tmux send-keys -t "$SESSION" "$input"
  sleep 0.5
  tmux send-keys -t "$SESSION" Enter

  # Wait for the turn to start, then quit with Ctrl+C.
  sleep "$wait_before_quit"
  tmux send-keys -t "$SESSION" C-c
  sleep 1

  # Session may have already exited cleanly after Ctrl+C; try kill-session.
  tmux kill-session -t "$SESSION" >/dev/null 2>&1 || true

  # Check that the process exited cleanly (no panic in stderr).
  if [[ -s "$TMP_HOME/${name}.err" ]]; then
    local panic
    panic=$(grep -i "panicked\|thread.*panicked\|abort\|SIGABRT" "$TMP_HOME/${name}.err" || true)
    if [[ -n "$panic" ]]; then
      echo "FAIL: process panicked on Ctrl+C during active turn"
      echo "--- stderr ---"
      cat "$TMP_HOME/${name}.err"
      echo "--- end output ---"
      return 1
    fi
  fi

  echo "PASS: Ctrl+C quit cleanly during active turn"
  return 0
}

FAILED=0

case "$MODE" in
  mock)
    # Verify the TUI launches and reaches the input prompt.
    run_tmux_scenario "launch" "" "Type a message to start" 2 || FAILED=1
    # Verify a user message is accepted and a turn starts (Working...).
    run_tmux_scenario "hello" "hello" "Working" 6 || FAILED=1
    # Verify Ctrl+C quits cleanly during an active turn.
    run_tmux_scenario_quit_during "quit_during_hello" "hello" "Working" 3 || FAILED=1
    # Verify tool-marker prompts start a turn.
    run_tmux_scenario "list_files" "list files" "Working" 6 || FAILED=1
    run_tmux_scenario "native_tool" "native tool" "Run bash" 6 || FAILED=1
  ;;
  minimax)
    run_tmux_scenario "minimax_hello" "hello" "Working" 25 || FAILED=1
    ;;
  *)
    echo "Unknown mode: $MODE"
    exit 1
    ;;
esac

if [[ "$FAILED" -eq 0 ]]; then
  echo "All tmux smoke tests passed."
  exit 0
else
  echo "Some tmux smoke tests failed."
  exit 1
fi
