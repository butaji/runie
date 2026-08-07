#!/usr/bin/env bash
set -euo pipefail

prefix=${1:?usage: capture-matrix.sh OUTPUT_DIR COMMAND QUIT_KEY [PROMPT] [ENV_ASSIGNMENTS]}
command_line=${2:?usage: capture-matrix.sh OUTPUT_DIR COMMAND QUIT_KEY [PROMPT] [ENV_ASSIGNMENTS]}
quit_key=${3:?usage: capture-matrix.sh OUTPUT_DIR COMMAND QUIT_KEY [PROMPT] [ENV_ASSIGNMENTS]}
prompt=${4:-Hey}
env_assignments=${5:-}
# Preserve the pre-scenario API (`OUTPUT COMMAND QUIT_KEY ENV_ASSIGNMENTS`).
# Environment assignments are unambiguous because they must contain `=` and
# begin with a shell identifier; a prompt such as `foo=bar` is intentionally
# required to use the new explicit fifth argument for this rare edge case.
if [[ -z "$env_assignments" && "$prompt" =~ ^[A-Za-z_][A-Za-z0-9_]*= ]]; then
    env_assignments=$prompt
    prompt=Hey
fi
mkdir -p "$prefix"

# The matrix intentionally spans narrow, normal, wide, and the Herdr-sized
# viewport. Width-dependent wrapping and height-dependent chrome must agree.
sizes=("62 32" "80 24" "100 30" "120 36")
failed=0
for size in "${sizes[@]}"; do
    read -r cols rows <<<"$size"
    stem="${prefix}/${cols}x${rows}"
    if ! scripts/tmux-asciinema-capture.sh "$cols" "$rows" "${stem}.cast" \
        "$command_line" "$prompt" "$quit_key" "$env_assignments"; then
        failed=1
    fi
done
exit "$failed"
