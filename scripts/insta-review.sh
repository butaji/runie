#!/bin/bash
# Insta snapshot review script
# Usage: ./scripts/insta-review.sh

set -e

echo "Running insta tests..."
cargo insta test

echo ""
echo "Checking for pending snapshots..."
cargo insta test --accept 2>/dev/null || {
    echo "No pending snapshots or all accepted."
}

echo ""
echo "Reviewing snapshots..."
cargo insta review || {
    echo "No snapshots to review or review completed."
}
