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
# Make color capability part of every captured artifact. Callers may override
# either value through the validated assignment list, but a capture must not
# silently inherit a terminal-default palette from the host shell.
record_command="TERM=xterm-256color COLORTERM=truecolor"
if [[ -n "$env_assignments" ]]; then
    validated_assignments=""
    for assignment in $env_assignments; do
        if [[ "$assignment" != [A-Za-z_][A-Za-z0-9_]*=* ]]; then
            echo "invalid environment assignment: ${assignment}" >&2
            exit 1
        fi
        validated_assignments+="${assignment} "
    done
    record_command+=" ${validated_assignments}"
fi
record_command+="$command_line"
escaped_command=${record_command//\'/\'\\\'\'}
quoted_command="'${escaped_command}'"
tmux send-keys -t "$session" -l \
  "asciinema rec --capture-input --window-size ${cols}x${rows} --overwrite --output-format asciicast-v2 --command ${quoted_command} '$cast'"
tmux send-keys -t "$session" Enter

# Wait for the actual prompt, not merely alternate-screen initialization.
# Grok may spend several seconds rendering its welcome surface first.
ready=0
welcome_advanced=0
for _ in $(seq 1 "$probe_iterations"); do
    screen=$(tmux capture-pane -p -t "$session" 2>/dev/null || true)
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
        || printf '%s' "$screen" | grep -Fq 'Type your message'; then
        ready=1
        break
    fi
    sleep 0.1
done
if (( ! ready )); then
    echo "timed out waiting for input prompt in ${cols}x${rows}: ${cast}" >&2
    exit 1
fi

tmux send-keys -t "$session" -l "$prompt"
tmux send-keys -t "$session" Enter
# Wait for both the completed-turn marker and the settled input footer so the
# cast does not stop on the first feed update while the status actor is still
# rendering its active phase. This is a bounded external-process probe.
settled=0
for _ in $(seq 1 "$probe_iterations"); do
    screen=$(tmux capture-pane -p -t "$session" 2>/dev/null || true)
    if printf '%s' "$screen" | grep -Fq 'Worked for' \
        && printf '%s' "$screen" | grep -Fq 'Shift+Tab' \
        && ! printf '%s' "$screen" | grep -Fq 'Esc:cancel'; then
        settled=1
        break
    fi
    sleep 0.1
done
if (( ! settled )); then
    echo "timed out waiting for completed turn in ${cols}x${rows}: ${cast}" >&2
    exit 1
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

# Emit the raw replay stream beside the timed cast for terminal-parser tools.
asciinema convert -f raw "$cast" "${cast%.cast}.raw"
