#!/usr/bin/env bash
set -euo pipefail

cols=${1:?usage: tmux-asciinema-capture.sh COLS ROWS CAST COMMAND PROMPT QUIT_KEY}
rows=${2:?usage: tmux-asciinema-capture.sh COLS ROWS CAST COMMAND PROMPT QUIT_KEY}
cast=${3:?usage: tmux-asciinema-capture.sh COLS ROWS CAST COMMAND PROMPT QUIT_KEY}
command_line=${4:?usage: tmux-asciinema-capture.sh COLS ROWS CAST COMMAND PROMPT QUIT_KEY}
prompt=${5:?usage: tmux-asciinema-capture.sh COLS ROWS CAST COMMAND PROMPT QUIT_KEY}
quit_key=${6:?usage: tmux-asciinema-capture.sh COLS ROWS CAST COMMAND PROMPT QUIT_KEY [ENV_ASSIGNMENTS]}
env_assignments=${7:-}
resize_schedule=${8:-}
session="runie-cast-$$"
probe_iterations=600

command -v tmux >/dev/null || { echo "tmux is required" >&2; exit 1; }
command -v asciinema >/dev/null || { echo "asciinema is required" >&2; exit 1; }

cleanup() { tmux kill-session -t "$session" 2>/dev/null || true; }
trap cleanup EXIT INT TERM

invalid_report="${cast%.cast}.invalid.json"
reject_capture() {
    reason=$1
    pane_dump="${cast%.cast}.invalid.ansi"
    tmux capture-pane -e -p -t "$session" -S 0 -E "$((rows - 1))" > "$pane_dump" 2>/dev/null || true
    jq -n \
      --arg reason "$reason" \
      --arg cast "$cast" \
      --arg pane_dump "$pane_dump" \
      --arg command "$command_line" \
      --arg prompt "$prompt" \
      --argjson cols "$cols" \
      --argjson rows "$rows" \
      '{valid:false, reason:$reason, command:$command, probe:$prompt,
        terminal:{cols:$cols, rows:$rows}, artifacts:{cast:$cast, pane_dump:$pane_dump}}' \
      > "$invalid_report"
    echo "invalid parity capture: $reason (see $invalid_report)" >&2
    exit 1
}

# Start with a shell hold so the PTY is resized before asciinema allocates its
# recording child. Launching asciinema in the initial command can make it
# inherit tmux's default 80×24 size before `resize-window` takes effect.
tmux new-session -d -s "$session" -x "$cols" -y "$rows" \
  "bash -c 'read -r launch; eval \"\$launch\"'"
tmux resize-window -t "$session" -x "$cols" -y "$rows"
# Grok's terminal probe distinguishes emitted color from color surviving the
# multiplexer.  A private tmux server must advertise RGB explicitly or tmux
# can downgrade/strip 24-bit SGR even when TERM/COLORTERM request truecolor.
# Keep this scoped to the capture session; never mutate the user's tmux config.
tmux set-option -t "$session" -as terminal-features ",*:RGB"
# Make color capability part of every captured artifact. Callers may override
# either value through the validated assignment list, but a capture must not
# silently inherit a terminal-default palette from the host shell.
# The host environment may set NO_COLOR (this workspace does). Grok's own
# doctor reports that as a hard color disable, so parity captures must make
# the intended color contract explicit rather than inheriting it silently.
record_command="env -u NO_COLOR TERM=xterm-256color COLORTERM=truecolor"
capture_term="xterm-256color"
capture_colorterm="truecolor"
if [[ -n "$env_assignments" ]]; then
    validated_assignments=""
    for assignment in $env_assignments; do
        if [[ "$assignment" != [A-Za-z_][A-Za-z0-9_]*=* ]]; then
            echo "invalid environment assignment: ${assignment}" >&2
            exit 1
        fi
        validated_assignments+="${assignment} "
        case "$assignment" in
            TERM=*) capture_term=${assignment#TERM=} ;;
            COLORTERM=*) capture_colorterm=${assignment#COLORTERM=} ;;
        esac
    done
    record_command+=" ${validated_assignments}"
fi
record_command+=" $command_line"
escaped_command=${record_command//\'/\'\\\'\'}
quoted_command="'${escaped_command}'"
# Input capture is intentionally disabled for parity recordings. With the
# installed asciinema/tmux combination it forwards control keys but swallows
# ordinary character events from crossterm applications such as Runie; frame
# and ANSI capture remain unchanged, and the prompt/quit interaction is driven
# by the private tmux PTY itself.
tmux send-keys -t "$session" -l \
  "asciinema rec --window-size ${cols}x${rows} --overwrite --output-format asciicast-v2 --command ${quoted_command} '$cast'"
tmux send-keys -t "$session" Enter

resize_pid=""
resize_report="${cast%.cast}.resize.json"
printf '%s\n' '{"valid":true,"observed":[]}' > "$resize_report"

# Wait for the actual prompt, not merely alternate-screen initialization.
# Grok may spend several seconds rendering its welcome surface first.
ready=0
welcome_advanced=0
for _ in $(seq 1 "$probe_iterations"); do
    screen=$(tmux capture-pane -p -t "$session" 2>/dev/null || true)
    if printf '%s' "$screen" | grep -Fq 'Help improve Grok'; then
        reject_capture "grok_consent_surface"
    fi
    # The welcome surface also contains `❯` and `Grok 4.5`; those are not an
    # editable working prompt. Grok advances from that surface with Enter.
    if (( ! welcome_advanced )) \
        && printf '%s' "$screen" | grep -Fq 'Coming from Codex' \
        && printf '%s' "$screen" | grep -Fq 'Grok 4.5'; then
        tmux send-keys -t "$session" Enter
        welcome_advanced=1
    fi
    # Require a working prompt footer so captures cannot type into welcome and
    # still be labeled as a settled scenario run.
    if printf '%s' "$screen" | grep -Fq 'Shift+Tab' \
        || printf '%s' "$screen" | grep -Fq 'Enter:send' \
        || printf '%s' "$screen" | grep -Fq 'Type your message' \
        || { printf '%s' "$screen" | grep -Fq 'Grok 4.5' \
             && printf '%s' "$screen" | grep -Fq '❯'; }; then
        ready=1
        break
    fi
    sleep 0.1
done
if (( ! ready )); then
    tmux capture-pane -e -p -t "$session" -S 0 -E "$((rows - 1))" \
        > "${cast%.cast}.timeout.ansi" 2>/dev/null || true
    reject_capture "input_prompt_timeout"
fi

# The wide layout can finish its first full paint after the prompt marker is
# visible. Give the crossterm input worker one bounded turn to subscribe before
# injecting the scenario; otherwise the first prompt bytes can be lost only at
# the widest geometry.
# Let the owned async input worker complete its subscription after the first
# prompt frame. The prompt marker and the mailbox readiness are separate
# events; a short fixed grace period prevents the first character from being
# lost on narrow and freshly resized panes.
sleep 1.0

# Send the complete prompt literally. Runie's FIFO input boundary preserves
# every key event, so a paste-like tmux send no longer risks overwriting input
# in the application. The exact visible prompt remains the acceptance gate.
tmux send-keys -t "$session" -l -- "$prompt"
acknowledged=0
for _ in $(seq 1 80); do
    screen=$(tmux capture-pane -p -t "$session" 2>/dev/null || true)
    if printf '%s' "$screen" | grep -Fq "$prompt"; then
        acknowledged=1
        break
    fi
    sleep 0.05
done
if [[ "$acknowledged" != 1 ]]; then
    reject_capture "prompt_key_timeout"
fi
injected_screen=$(tmux capture-pane -p -t "$session" 2>/dev/null || true)
if ! printf '%s' "$injected_screen" | grep -Fq "$prompt"; then
    reject_capture "prompt_injection_mismatch"
fi
tmux send-keys -t "$session" Enter

# Optional YAML-driven mid-capture resize schedule. Its clock starts after the
# submitted prompt so startup/welcome layout cannot consume a feed scenario's
# resize budget. The schedule is validated by capture-scenario.sh and scoped
# to this private tmux session.
if [[ -n "$resize_schedule" ]]; then
    (
        previous_ms=0
        observed='[]'
        IFS=';' read -ra entries <<< "$resize_schedule"
        for entry in "${entries[@]}"; do
            IFS=',' read -r at_ms resize_cols resize_rows <<< "$entry"
            delay_ms=$((at_ms - previous_ms))
            (( delay_ms > 0 )) && sleep "0.$(printf '%03d' "$delay_ms")"
            tmux resize-window -t "$session" -x "$resize_cols" -y "$resize_rows" 2>/dev/null || exit 0
            actual=$(tmux display-message -p -t "$session" '#{window_width},#{window_height}' 2>/dev/null || printf '')
            if [[ "$actual" != "$resize_cols,$resize_rows" ]]; then
                jq -n --arg expected "$resize_cols,$resize_rows" --arg actual "$actual" \
                    '{valid:false,expected:$expected,actual:$actual}' > "$resize_report"
                exit 1
            fi
            observed=$(jq --arg at "$at_ms" --arg geometry "$actual" \
                '. + [{at_ms:($at|tonumber),geometry:$geometry}]' <<< "$observed")
            previous_ms=$at_ms
        done
        jq -n --argjson observed "$observed" '{valid:true,observed:$observed}' > "$resize_report"
    ) &
    resize_pid=$!
fi
# Wait for both the completed-turn marker and the settled input footer so the
# cast does not stop on the first feed update while the status actor is still
# rendering its active phase. This is a bounded external-process probe.
settled=0
for _ in $(seq 1 "$probe_iterations"); do
    screen=$(tmux capture-pane -p -t "$session" 2>/dev/null || true)
    if printf '%s' "$screen" | grep -Fq 'Help improve Grok'; then
        reject_capture "grok_consent_surface"
    fi
    if printf '%s' "$screen" | grep -Fq 'Worked for' \
        && printf '%s' "$screen" | grep -Fq 'Shift+Tab' \
        && ! printf '%s' "$screen" | grep -Fq 'Esc:cancel'; then
        settled=1
        break
    fi
    sleep 0.1
done
if (( ! settled )); then
    tmux capture-pane -e -p -t "$session" -S 0 -E "$((rows - 1))" \
        > "${cast%.cast}.timeout.ansi" 2>/dev/null || true
    reject_capture "settled_turn_timeout"
fi
# Preserve the exact settled application frame before the quit key restores
# the shell. The asciinema stream remains useful for animation replay, but its
# final frame is not a reliable settled-state oracle after teardown.
settled_dump="${cast%.cast}.settled.ansi"
tmux capture-pane -e -p -t "$session" -S 0 -E "$((rows - 1))" > "$settled_dump"

if [[ -n "$resize_pid" ]]; then
    if ! wait "$resize_pid"; then
        reject_capture "resize_observation_failed"
    fi
fi
tmux send-keys -t "$session" "$quit_key"

# The caller controls interaction through the session name while recording.
# A command that exits naturally (or receives its quit key) closes asciinema.
shutdown_iterations=100
for _ in $(seq 1 "$shutdown_iterations"); do
    if ! tmux has-session -t "$session" 2>/dev/null; then
        break
    fi
    sleep 0.1
done
if tmux has-session -t "$session" 2>/dev/null; then
    tmux kill-session -t "$session"
fi
if [[ -n "$resize_pid" ]]; then
    kill "$resize_pid" 2>/dev/null || true
    wait "$resize_pid" 2>/dev/null || true
fi

# Emit the raw replay stream beside the timed cast for terminal-parser tools.
asciinema convert -f raw "$cast" "${cast%.cast}.raw"

# Preserve the provenance required for a trustworthy parity decision beside
# every cast. Keep credentials and shell environment out of the manifest.
manifest="${cast%.cast}.meta.json"
doctor_report="${cast%.cast}.grok-doctor.json"
repo_revision=$(git rev-parse HEAD 2>/dev/null || printf 'unknown')
grok_path=$(command -v grok 2>/dev/null || printf '')
grok_version=$([[ -n "$grok_path" ]] && "$grok_path" --version 2>/dev/null | head -1 || printf '')
tmux_version=$(tmux -V 2>/dev/null || printf '')
asciinema_version=$(asciinema --version 2>/dev/null || printf '')
# Preserve the capability probe with the capture. A failed probe is recorded as
# JSON rather than aborting the capture: unavailable color is explicit evidence
# and cannot be mistaken for a theme-palette oracle.
doctor_command=(env -u NO_COLOR "TERM=$capture_term" "COLORTERM=$capture_colorterm" "$grok_path" doctor --json)
if [[ -n "$grok_path" ]] && "${doctor_command[@]}" > "$doctor_report" 2>/dev/null; then
    :
else
    printf '%s\n' '{"status":"unavailable","reason":"grok doctor probe failed"}' > "$doctor_report"
fi
jq -n \
  --arg captured_at "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  --arg repo_revision "$repo_revision" \
  --arg command "$command_line" \
  --arg capture_env "$env_assignments" \
  --arg grok_path "$grok_path" \
  --arg grok_version "$grok_version" \
  --arg tmux_version "$tmux_version" \
  --arg asciinema_version "$asciinema_version" \
  --arg term "$capture_term" \
  --arg colorterm "$capture_colorterm" \
  --arg cast "$cast" \
  --arg raw "${cast%.cast}.raw" \
  --arg settled_dump "$settled_dump" \
  --arg doctor_report "$doctor_report" \
  --arg probe "$prompt" \
  --arg quit_key "$quit_key" \
  --arg resize_schedule "$resize_schedule" \
  --arg resize_report "$resize_report" \
  --argjson cols "$cols" \
  --argjson rows "$rows" \
  '{captured_at:$captured_at, repo_revision:$repo_revision,
    command:$command, capture_env:$capture_env,
    probe:{prompt:$probe, quit_key:$quit_key}, resize_schedule:$resize_schedule,
    grok_path:$grok_path, grok_version:$grok_version,
    capture_tools:{tmux:$tmux_version, asciinema:$asciinema_version},
    terminal:{cols:$cols, rows:$rows, term:$term, colorterm:$colorterm},
    artifacts:{cast:$cast, raw:$raw, settled_ansi:$settled_dump,
      grok_doctor:$doctor_report, resize_report:$resize_report}}' > "$manifest"
