#!/bin/bash
mode="${1:-run}"
export PATH="$HOME/.cargo/bin:$PATH"

if ! command -v cargo-watch > /dev/null 2>&1; then
    echo "cargo-watch not found. Install: cargo install cargo-watch"
    exit 1
fi

# Parse flags
export RUNIE_MOCK_DELAY=""
case "$mode" in
  run-delay|fast-delay)
    export RUNIE_MOCK_DELAY=1
    ;;
esac

case "$mode" in
  run|run-delay)
    echo "[dev] Hot reload (release). Ctrl+C to stop."
    cargo watch -x 'run --release -p runie-term' -w crates
    ;;
  fast|fast-delay)
    echo "[dev] Hot reload (debug). Ctrl+C to stop."
    cargo watch -x 'run -p runie-term' -w crates
    ;;
  test)
    cargo test --all
    ;;
  smoke)
    echo "[dev] Running smoke tests..."
    cargo build --release -p runie 2>/dev/null
    ./scripts/smoke-tab-completion.sh
    ./scripts/smoke-turn-complete.sh
    echo "[dev] All smoke tests passed!"
    ;;
  *)
    echo "Usage: \$0 [run|run-delay|fast|fast-delay|test]"
    echo ""
    echo "Modes:"
    echo "  run        - release build, no mock delays"
    echo "  run-delay  - release build, random 0.5s-3s delays between mock chunks"
    echo "  fast       - debug build, no mock delays"
    echo "  fast-delay - debug build, random 0.5s-3s delays between mock chunks"
    echo "  test       - run all tests"
    ;;
esac
