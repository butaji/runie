#!/bin/bash
# Regenerate event taxonomy files from taxonomy.json.
#
# This script should be run when modifying crates/runie-core/src/event/taxonomy.json.
# Generated files are committed to git, so this script only needs to be run manually
# when making changes to the taxonomy.
#
# Usage:
#   ./scripts/generate-event-taxonomy.sh
#
# Requirements:
#   - Rust toolchain (cargo)
#   - Run from repository root

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
CRATE_DIR="$ROOT_DIR/crates/runie-core"

echo "Regenerating event taxonomy from taxonomy.json..."
echo ""

# The generation is done by build.rs when cargo builds the crate.
# We trigger a rebuild by touching taxonomy.json.
touch "$CRATE_DIR/src/event/taxonomy.json"

# Run cargo check to trigger build.rs generation
cd "$ROOT_DIR"
cargo check -p runie-core 2>&1 | grep -E "(generated|error|warning:.*generated)" || true

echo ""
echo "Event taxonomy regenerated. Please review and commit the changes in:"
echo "  $CRATE_DIR/src/event/generated/"
