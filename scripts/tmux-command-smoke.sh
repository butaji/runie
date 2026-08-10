#!/usr/bin/env bash
set -u

socket=${1:-runie-command-smoke}
binary=${2:-target/debug/runie}
labels=(
  "New Session" "Keyboard Shortcuts" "Changelog" "Copy Last Response" "Session Info"
  "Select Model" "Select Theme" "Manage Providers" "Scoped Models" "Set Session Name"
  "Compact Context" "Fork Session" "Select Tree Entry" "Export Session" "Import Session"
  "Clone Session" "Resume Session" "Share Session" "Help" "Context Info" "Settings"
  "Doctor" "Rewind Session" "Prompt History" "Find Transcript" "Jump Transcript" "Recap"
  "Set Reasoning Effort" "Always Approve" "Automatic Approval" "Plan Mode" "View Plan"
  "Login" "Logout" "Reload" "Trust Project" "Skills" "Hooks" "Plugins" "MCP Servers"
  "Memory" "Remember" "Goal" "Workflow" "Workflows" "Loop" "Deep Research" "Feedback" "Usage"
)
parameterized=(
  "Select Theme" "Set Session Name" "Compact Context" "Fork Session" "Select Tree Entry"
  "Export Session" "Import Session" "Clone Session" "Resume Session" "Help" "Settings" "Doctor"
  "Rewind Session" "Prompt History" "Find Transcript" "Jump Transcript" "Set Reasoning Effort"
  "Always Approve" "Automatic Approval" "Plan Mode" "Login" "Logout" "Trust Project" "Remember"
  "Goal" "Workflow" "Loop" "Deep Research" "Feedback" "Usage"
)

is_parameterized() {
  local candidate
  for candidate in "${parameterized[@]}"; do
    [[ "$candidate" == "$1" ]] && return 0
  done
  return 1
}

capture() { tmux -L "$socket" capture-pane -p -t smoke 2>/dev/null || true; }

wait_for() {
  local pattern=$1
  for _ in $(seq 1 100); do
    if capture | rg -q "$pattern"; then return 0; fi
    sleep 0.05
  done
  return 1
}

passed=0
failed=0
for label in "${labels[@]}"; do
  tmux -L "$socket" -f /dev/null kill-session -t smoke 2>/dev/null || true
  tmux -L "$socket" -f /dev/null new-session -d -s smoke -x 120 -y 36 \
    "env -u NO_COLOR TERM=xterm-256color COLORTERM=truecolor $binary" >/dev/null 2>&1 || {
      echo "FAIL $label: launch"; ((failed += 1)); continue;
    }
  tmux -L "$socket" resize-window -t smoke -x 120 -y 36 >/dev/null 2>&1
  if ! wait_for 'Enter:send|Type your message|Shift\\+Tab'; then
    echo "FAIL $label: startup"; ((failed += 1)); continue
  fi
  tmux -L "$socket" send-keys -t smoke C-p
  if ! wait_for 'Commands'; then
    echo "FAIL $label: palette-open"; ((failed += 1)); continue
  fi
  for ((index = 0; index < ${#label}; index += 1)); do
    tmux -L "$socket" send-keys -t smoke -l -- "${label:index:1}"
    sleep 0.04
  done
  tmux -L "$socket" send-keys -t smoke Enter
  transitioned=0
  for _ in $(seq 1 100); do
    screen=$(capture)
    if ! printf '%s' "$screen" | rg -q 'Commands' \
      || printf '%s' "$screen" | rg -q 'Command Parameters|Session Info|Themes|Models'; then
      transitioned=1
      break
    fi
    sleep 0.05
  done
  if [[ $transitioned != 1 ]]; then
    echo "FAIL $label: transition-timeout"; ((failed += 1)); continue
  fi
  if is_parameterized "$label"; then
    for character in s m o k e; do
      tmux -L "$socket" send-keys -t smoke -l -- "$character"
      sleep 0.04
    done
    tmux -L "$socket" send-keys -t smoke Enter
    sleep 0.5
  fi
  if capture | rg -qi 'panic|thread .* panicked|unknown command|fatal error'; then
    echo "FAIL $label: runtime-error"; ((failed += 1))
  else
    echo "PASS $label"; ((passed += 1))
  fi
done
tmux -L "$socket" -f /dev/null kill-session -t smoke 2>/dev/null || true
echo "SUMMARY passed=$passed failed=$failed"
((failed == 0))
