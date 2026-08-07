#!/usr/bin/env bash
set -euo pipefail

prefix=${1:?usage: capture-matrix.sh OUTPUT_DIR COMMAND QUIT_KEY [PROMPT] [ENV_ASSIGNMENTS]}
command_line=${2:?usage: capture-matrix.sh OUTPUT_DIR COMMAND QUIT_KEY [PROMPT] [ENV_ASSIGNMENTS]}
quit_key=${3:?usage: capture-matrix.sh OUTPUT_DIR COMMAND QUIT_KEY [PROMPT] [ENV_ASSIGNMENTS]}
prompt=${4:-Hey}
env_assignments=${5:-}
mkdir -p "$prefix"

# The matrix intentionally spans narrow, normal, wide, and the Herdr-sized
# viewport. Width-dependent wrapping and height-dependent chrome must agree.
sizes=("62 32" "80 24" "100 30" "120 36")
for size in "${sizes[@]}"; do
    read -r cols rows <<<"$size"
    stem="${prefix}/${cols}x${rows}"
    scripts/tmux-asciinema-capture.sh "$cols" "$rows" "${stem}.cast" \
        "$command_line" "$prompt" "$quit_key" "$env_assignments"
done
