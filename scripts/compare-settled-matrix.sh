#!/usr/bin/env bash
set -euo pipefail

left_dir=${1:?usage: compare-settled-matrix.sh GROK_DIR RUNIE_DIR}
right_dir=${2:?usage: compare-settled-matrix.sh GROK_DIR RUNIE_DIR}

python=${PYTHON:-python3}
command -v "$python" >/dev/null || { echo "python is required" >&2; exit 1; }

failed=0
for size in 62x32 80x24 100x30 120x36; do
    left="${left_dir}/${size}.settled.ansi"
    right="${right_dir}/${size}.settled.ansi"
    if [[ ! -f "$left" || ! -f "$right" ]]; then
        echo "missing settled ANSI pair for ${size}: ${left} / ${right}" >&2
        exit 1
    fi
    read -r cols rows <<<"${size/x/ }"
    echo "== ${size} settled ANSI =="
    if ! "$python" scripts/compare-ansi-frames.py "$left" "$right" \
        --cols "$cols" --rows "$rows"; then
        failed=1
    fi
done

exit "$failed"
