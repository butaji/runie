#!/usr/bin/env bash
set -euo pipefail

# Print a deterministic, file-by-file inventory for parity reviews.
# Usage: scripts/source-inventory.sh [pi-root] [grok-root]
pi_root=${1:-/Users/admin/Code/agents/pi}
grok_root=${2:-/Users/admin/Code/agents/grok-build}

print_tree() {
    local label=$1
    local root=$2
    local path=$3
    printf '[%s]\n' "$label"
    rg --files "$root/$path" | sort | while IFS= read -r file; do
        printf '%s\t%s\n' "$label" "${file#"$root/"}"
    done
}

print_tree pi-agent "$pi_root" packages/agent/src
print_tree pi-ai "$pi_root" packages/ai/src
print_tree grok-pager "$grok_root" crates/codegen/xai-grok-pager/src
print_tree grok-render "$grok_root" crates/codegen/xai-grok-pager-render/src
