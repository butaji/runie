#!/usr/bin/env bash
set -u

socket=${1:-runie-command-smoke}
binary=${2:-target/debug/runie}
socket_path=${RUNIE_TMUX_SOCKET:-/tmp/${socket}.sock}
tmux_run() { tmux -S "$socket_path" "$@"; }
labels=(
  "New Session" "Keyboard Shortcuts" "Changelog" "Copy Last Response" "Session Info"
  "Select Model" "Select Theme" "Manage Providers" "Scoped Models" "Set Session Name"
  "Compact Context" "Fork Session" "Select Tree Entry" "Export Session" "Import Session"
  "Clone Session" "Resume Session" "Share Session" "Help" "Context Info" "Settings"
  "Doctor" "Rewind Session" "Prompt History" "Find Transcript" "Jump Transcript" "Recap"
  "Set Reasoning Effort" "Always Approve" "Automatic Approval" "Deny Tools" "Plan Mode" "View Plan"
  "Login" "Logout" "Reload" "Trust Project" "Skills" "Hooks" "Plugins" "MCP Servers" "Close MCP Servers" "Reconnect MCP Servers"
  "Memory" "Remember" "Goal" "Workflow" "Workflows" "Sessions" "Session History" "Session History Query" "Undo Session" "Questions" "Active Jobs" "Cancel All Jobs" "Cancel Running Jobs" "Clear Finished Jobs" "Completed Jobs" "Failed Jobs" "Cancelled Jobs" "Git Status" "Git Diff" "Git Review" "Git Worktrees" "Git Conflicts" "MCP Notifications" "Clear MCP Notifications" "Ready MCP Servers" "Failed MCP Servers" "Busy MCP Servers" "Closed MCP Servers" "Stdio MCP Servers" "HTTP MCP Servers" "Loop" "Deep Research" "Feedback" "Usage" "Usage Chart" "Context Policy" "Enable Context Policy" "Disable Context Policy" "Set Context Reserve" "Set Context Keep Recent"
  "Background Jobs" "Jobs" "Job Output" "Job Output Facts" "Cancel Queued Jobs"
)
parameterized=(
  "Select Theme" "Set Session Name" "Compact Context" "Fork Session" "Select Tree Entry" "Usage Chart"
  "Export Session" "Import Session" "Clone Session" "Resume Session" "Help" "Settings" "Doctor"
  "Rewind Session" "Prompt History" "Find Transcript" "Jump Transcript" "Set Reasoning Effort"
  "Always Approve" "Automatic Approval" "Plan Mode" "Login" "Logout" "Trust Project" "Remember" "Undo Session" "Set Context Reserve" "Set Context Keep Recent"
  "Goal" "Workflow" "Loop" "Deep Research" "Feedback" "Usage" "Background Jobs" "Questions" "Jobs" "Completed Jobs" "Failed Jobs" "Cancelled Jobs" "Busy MCP Servers" "Closed MCP Servers" "Stdio MCP Servers" "HTTP MCP Servers" "Git Status" "Git Diff" "Git Review" "Git Worktrees" "Git Conflicts" "Job Output" "Job Output Facts" "Session History Query"
)

is_parameterized() {
  local candidate
  for candidate in "${parameterized[@]}"; do
    [[ "$candidate" == "$1" ]] && return 0
  done
  return 1
}

capture() { tmux_run capture-pane -p -t smoke 2>/dev/null || true; }

wait_for() {
  local pattern=$1
  for _ in $(seq 1 200); do
    if capture | rg -q "$pattern"; then return 0; fi
    sleep 0.05
  done
  return 1
}

passed=0
failed=0
for label in "${labels[@]}"; do
  tmux_run -f /dev/null kill-session -t smoke 2>/dev/null || true
  tmux_run -f /dev/null new-session -d -s smoke -x 120 -y 36 \
    "env -u NO_COLOR TERM=xterm-256color COLORTERM=truecolor $binary" >/dev/null 2>&1 || {
      echo "FAIL $label: launch"; ((failed += 1)); continue;
    }
  tmux_run resize-window -t smoke -x 120 -y 36 >/dev/null 2>&1
  if ! wait_for 'Enter:send|Type your message|Shift\\+Tab'; then
    echo "FAIL $label: startup"; ((failed += 1)); continue
  fi
  tmux_run send-keys -t smoke C-p
  if ! wait_for 'Commands'; then
    echo "FAIL $label: palette-open"; ((failed += 1)); continue
  fi
  for ((index = 0; index < ${#label}; index += 1)); do
    tmux_run send-keys -t smoke -l -- "${label:index:1}"
    sleep 0.08
  done
  tmux_run send-keys -t smoke Enter
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
    argument=(s m o k e)
    if [[ "$label" == "Questions" ]]; then
      argument=(c l e a r)
    elif [[ "$label" == "Usage" ]]; then
      argument=(l a s t 3)
    elif [[ "$label" == "Jobs" ]]; then
      argument=(s c h e d u l e r)
    elif [[ "$label" == "Git Conflicts" ]]; then
      argument=(c o n f l i c t s)
    elif [[ "$label" == "Session History Query" ]]; then
      argument=(a c t i v e _ t o o l s)
    elif [[ "$label" == "Usage Chart" ]]; then
      argument=(c o s t)
    elif [[ "$label" == "Set Context Reserve" ]]; then
      argument=(1 2 3)
    elif [[ "$label" == "Set Context Keep Recent" ]]; then
      argument=(4 2)
    elif [[ "$label" == "Job Output" ]]; then
      argument=(o u t p u t 0 t a i l 2)
    fi
    for character in "${argument[@]}"; do
      tmux_run send-keys -t smoke -l -- "$character"
      sleep 0.08
    done
    tmux_run send-keys -t smoke Enter
    sleep 0.5
    if [[ "$label" == "Background Jobs" ]] && ! wait_for 'Background job smoke not found'; then
      echo "FAIL $label: expected-query-result"
      ((failed += 1))
      continue
    fi
    if [[ "$label" == "Jobs" ]] && ! wait_for 'scheduler queued:'; then
      echo "FAIL $label: expected-scheduler-result"
      ((failed += 1))
      continue
    fi
    if [[ "$label" == "MCP Servers" ]] && ! wait_for 'No MCP stdio servers'; then
      echo "FAIL $label: expected-empty-status"
      ((failed += 1))
      continue
    fi
    if [[ "$label" == "Cancel Queued Jobs" ]] && ! wait_for 'Cancelled 0 queued tool batch'; then
      echo "FAIL $label: expected-queued-cancel-result"
      ((failed += 1))
      continue
    fi
    if [[ "$label" == "Close MCP Servers" ]] && ! wait_for 'Closed 0 MCP server'; then
      echo "FAIL $label: expected-close-result"
      ((failed += 1))
      continue
    fi
    if [[ "$label" == "Reconnect MCP Servers" ]] && ! wait_for 'Reconnected 0 MCP server'; then
      echo "FAIL $label: expected-reconnect-result"
      ((failed += 1))
      continue
    fi
    if [[ "$label" == "Ready MCP Servers" ]] && ! wait_for 'No MCP ready servers'; then
      echo "FAIL $label: expected-empty-status"
      ((failed += 1))
      continue
    fi
    if [[ "$label" == "Failed MCP Servers" ]] && ! wait_for 'No MCP failed servers'; then
      echo "FAIL $label: expected-empty-status"
      ((failed += 1))
      continue
    fi
    if [[ "$label" == "Questions" ]] && ! wait_for 'User-question history cleared'; then
      echo "FAIL $label: expected-clear-result"
      ((failed += 1))
      continue
    fi
    if [[ "$label" == "Git Conflicts" ]] && ! wait_for 'Git conflicts:'; then
      echo "FAIL $label: expected-conflict-result"
      ((failed += 1))
      continue
    fi
    if [[ "$label" == "Session History Query" ]] && ! wait_for 'No matching session history'; then
      echo "FAIL $label: expected-query-result"
      ((failed += 1))
      continue
    fi
    if [[ "$label" == "Cancel All Jobs" ]] && ! wait_for 'Cancelled 0 all background job'; then
      echo "FAIL $label: expected-cancel-result"
      ((failed += 1))
      continue
    fi
    if [[ "$label" == "Cancel Running Jobs" ]] && ! wait_for 'Cancelled 0 running background job'; then
      echo "FAIL $label: expected-cancel-result"
      ((failed += 1))
      continue
    fi
    if [[ "$label" == "Clear Finished Jobs" ]] && ! wait_for 'Cleared 0 finished background job'; then
      echo "FAIL $label: expected-clear-result"
      ((failed += 1))
      continue
    fi
    if [[ "$label" == "Set Context Reserve" ]] && ! wait_for 'Context reserve tokens: 123'; then
      echo "FAIL $label: expected-context-result"
      ((failed += 1))
      continue
    fi
    if [[ "$label" == "Set Context Keep Recent" ]] && ! wait_for 'Context keep-recent tokens: 42'; then
      echo "FAIL $label: expected-context-result"
      ((failed += 1))
      continue
    fi
  fi
  if capture | rg -qi 'panic|thread .* panicked|unknown command|fatal error'; then
    echo "FAIL $label: runtime-error"; ((failed += 1))
  else
    echo "PASS $label"; ((passed += 1))
  fi
done

# Direct slash-command cases cover no-argument branches that are distinct
# from the parameterized palette entries.
while IFS='|' read -r label command expected; do
  tmux_run -f /dev/null kill-session -t smoke 2>/dev/null || true
  tmux_run -f /dev/null new-session -d -s smoke -x 120 -y 36 \
    "env -u NO_COLOR TERM=xterm-256color COLORTERM=truecolor $binary" >/dev/null 2>&1 || {
      echo "FAIL $label: launch"; ((failed += 1)); continue;
    }
  tmux_run resize-window -t smoke -x 120 -y 36 >/dev/null 2>&1
  if ! wait_for 'Enter:send|Type your message|Shift\\+Tab'; then
    echo "FAIL $label: startup"; ((failed += 1)); continue
  fi
  tmux_run send-keys -t smoke -l -- "$command"
  tmux_run send-keys -t smoke Enter
  if wait_for "$expected" && ! capture | rg -qi 'panic|thread .* panicked|unknown command|fatal error'; then
    echo "PASS $label"; ((passed += 1))
  else
    echo "FAIL $label: command-result"; ((failed += 1))
  fi
done <<'CASES'
Resume Picker|/resume|Command Parameters
Context Summary|/context|compaction: none
Session History Rows|/sessions history|No session history
CASES

# The quit path is a TUI-only terminal case: it must end the session rather
# than transition to another overlay.
tmux_run -f /dev/null new-session -d -s smoke -x 120 -y 36 \
  "env -u NO_COLOR TERM=xterm-256color COLORTERM=truecolor $binary" >/dev/null 2>&1
if wait_for 'Enter:send|Type your message|Shift\\+Tab'; then
  tmux_run send-keys -t smoke -l -- '/quit'
  tmux_run send-keys -t smoke Enter
  exited=0
  for _ in $(seq 1 60); do
    if ! tmux_run has-session -t smoke 2>/dev/null; then
      exited=1
      break
    fi
    sleep 0.05
  done
  if [[ $exited == 1 ]]; then
    echo "PASS Quit"; ((passed += 1))
  else
    echo "FAIL Quit: session-alive"; ((failed += 1))
  fi
else
  echo "FAIL Quit: startup"; ((failed += 1))
fi
tmux_run -f /dev/null kill-session -t smoke 2>/dev/null || true
echo "SUMMARY passed=$passed failed=$failed"
((failed == 0))
