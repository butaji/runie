#!/bin/bash
# Insta snapshot review script
# Usage: ./scripts/insta-review.sh
#
# Runs insta tests and prompts for review of any new/changed snapshots.
# Does NOT auto-accept snapshots.

set -e

echo "Running insta tests..."
cargo insta test --no-quiet

echo ""
echo "Pending snapshots (if any) — review with: cargo insta review"
PENDING=$(cargo insta pending-snapshots --quiet 2>/dev/null || true)
if [ -n "$PENDING" ]; then
    echo "$PENDING"
    echo ""
    read -p "Open interactive review now? [y/N] " review
    if [[ "$review" =~ ^[Yy]$ ]]; then
        cargo insta review
    else
        echo "Skipping interactive review. Run 'cargo insta review' manually."
    fi
else
    echo "No pending snapshots."
fi
