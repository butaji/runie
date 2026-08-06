#!/usr/bin/env bash
set -euo pipefail

left_dir=${1:?usage: compare-matrix.sh GROK_DIR RUNIE_DIR}
right_dir=${2:?usage: compare-matrix.sh GROK_DIR RUNIE_DIR}

command -v cargo >/dev/null || { echo "cargo is required" >&2; exit 1; }

for size in 62x32 80x24 100x30 120x36; do
    left="${left_dir}/${size}.cast"
    right="${right_dir}/${size}.cast"
    if [[ ! -f "$left" || ! -f "$right" ]]; then
        echo "missing paired cast for ${size}: ${left} / ${right}" >&2
        exit 1
    fi
    echo "== ${size} =="
    cargo run -q -p runie-tui --bin cast_compare -- "$left" "$right"
done
