#!/usr/bin/env bash
set -euo pipefail

pane_id=${1:?usage: herdr-dump.sh PANE_ID OUTPUT_PREFIX}
prefix=${2:?usage: herdr-dump.sh PANE_ID OUTPUT_PREFIX}

command -v herdr >/dev/null || { echo "herdr is required" >&2; exit 1; }

mkdir -p "$(dirname "$prefix")"

# Keep the terminal's complete visible grid and SGR sequences.  Do not use
# `recent`: it is a scrollback-oriented projection and is unsuitable for
# cell-for-cell comparisons.
herdr pane read "$pane_id" \
  --source visible \
  --format ansi \
  --raw > "${prefix}.ansi"

# Snapshot supplies the authoritative pane id, viewport, cwd, and terminal
# identity alongside the raw frame.
herdr pane get "$pane_id" > "${prefix}.pane.json"
herdr api snapshot > "${prefix}.herdr.json"

python3 - "$prefix" <<'PY'
import json
import pathlib
import sys

prefix = pathlib.Path(sys.argv[1])
pane = json.loads((prefix.with_suffix(".pane.json")).read_text())
snapshot = json.loads((prefix.with_suffix(".herdr.json")).read_text())
info = pane.get("result", {}).get("pane", {})
snapshot_data = snapshot.get("result", {}).get("snapshot", {})
rectangle = {}
for layout in snapshot_data.get("layouts", []):
    for candidate in layout.get("panes", []):
        if candidate.get("pane_id") == info.get("pane_id"):
            rectangle = candidate.get("rect", {})
            break
payload = {
    "pane_id": info.get("pane_id"),
    "terminal_id": info.get("terminal_id"),
    "terminal_title": info.get("terminal_title"),
    "cwd": info.get("cwd"),
    "viewport_rows": info.get("scroll", {}).get("viewport_rows"),
    "cols": rectangle.get("width"),
    "rows": rectangle.get("height"),
}
(prefix.with_suffix(".meta.json")).write_text(json.dumps(payload, indent=2) + "\n")
PY

printf '%s\n' \
  "wrote ${prefix}.ansi" \
  "wrote ${prefix}.pane.json" \
  "wrote ${prefix}.herdr.json" \
  "wrote ${prefix}.meta.json"
