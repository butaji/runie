#!/usr/bin/env bash
set -euo pipefail

cols=${1:?usage: tmux-asciinema-capture.sh COLS ROWS CAST COMMAND PROMPT QUIT_KEY}
rows=${2:?usage: tmux-asciinema-capture.sh COLS ROWS CAST COMMAND PROMPT QUIT_KEY}
cast=${3:?usage: tmux-asciinema-capture.sh COLS ROWS CAST COMMAND PROMPT QUIT_KEY}
command_line=${4:?usage: tmux-asciinema-capture.sh COLS ROWS CAST COMMAND PROMPT QUIT_KEY}
prompt=${5:?usage: tmux-asciinema-capture.sh COLS ROWS CAST COMMAND PROMPT QUIT_KEY}
quit_key=${6:?usage: tmux-asciinema-capture.sh COLS ROWS CAST COMMAND PROMPT QUIT_KEY [ENV_ASSIGNMENTS]}
env_assignments=${7:-}
session="runie-cast-$$"
probe_iterations=600

command -v tmux >/dev/null || { echo "tmux is required" >&2; exit 1; }
command -v asciinema >/dev/null || { echo "asciinema is required" >&2; exit 1; }

cleanup() { tmux kill-session -t "$session" 2>/dev/null || true; }
trap cleanup EXIT INT TERM

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

# Wait for the actual prompt, not merely alternate-screen initialization.
# Grok may spend several seconds rendering its welcome surface first.
ready=0
welcome_advanced=0
for _ in $(seq 1 "$probe_iterations"); do
    screen=$(tmux capture-pane -p -t "$session" 2>/dev/null || true)
    if printf '%s' "$screen" | grep -Fq 'Help improve Grok'; then
        echo "blocked by Grok consent surface; complete consent setup before capturing parity" >&2
        exit 1
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
    echo "timed out waiting for input prompt in ${cols}x${rows}: ${cast}" >&2
    exit 1
fi

# The wide layout can finish its first full paint after the prompt marker is
# visible. Give the crossterm input worker one bounded turn to subscribe before
# injecting the scenario; otherwise the first prompt bytes can be lost only at
# the widest geometry.
sleep 0.2

# Send one ordinary tmux key event per character. A single multi-character
# argument is interpreted as a tmux key name (`Hey`), while `-l` emits a
# paste-like byte sequence that crossterm does not expose as ordinary
# character events to Runie.
while IFS= read -r -n 1 character; do
    if [[ -n "$character" ]]; then
        # Literal mode is required per character: plain tmux arguments can be
        # interpreted as key names, while a burst of crossterm events can be
        # coalesced/dropped before Runie's bounded input worker receives it.
        tmux send-keys -t "$session" -l "$character"
        sleep 0.03
    fi
done <<< "$prompt"
tmux send-keys -t "$session" Enter
# Wait for both the completed-turn marker and the settled input footer so the
# cast does not stop on the first feed update while the status actor is still
# rendering its active phase. This is a bounded external-process probe.
settled=0
for _ in $(seq 1 "$probe_iterations"); do
    screen=$(tmux capture-pane -p -t "$session" 2>/dev/null || true)
    if printf '%s' "$screen" | grep -Fq 'Help improve Grok'; then
        echo "blocked by Grok consent surface; refusing invalid settled capture" >&2
        exit 1
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
    echo "timed out waiting for completed turn in ${cols}x${rows}: ${cast}" >&2
    exit 1
fi
# Preserve the exact settled application frame before the quit key restores
# the shell. The asciinema stream remains useful for animation replay, but its
# final frame is not a reliable settled-state oracle after teardown.
settled_dump="${cast%.cast}.settled.ansi"
tmux capture-pane -e -p -t "$session" -S 0 -E "$((rows - 1))" > "$settled_dump"
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
  --argjson cols "$cols" \
  --argjson rows "$rows" \
  '{captured_at:$captured_at, repo_revision:$repo_revision,
    command:$command, capture_env:$capture_env,
    probe:{prompt:$probe, quit_key:$quit_key},
    grok_path:$grok_path, grok_version:$grok_version,
    capture_tools:{tmux:$tmux_version, asciinema:$asciinema_version},
    terminal:{cols:$cols, rows:$rows, term:$term, colorterm:$colorterm},
    artifacts:{cast:$cast, raw:$raw, settled_ansi:$settled_dump,
      grok_doctor:$doctor_report}}' > "$manifest"
