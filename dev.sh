#!/usr/bin/env bash
set -euo pipefail

export PATH="$HOME/.cargo/bin:$PATH"

mode="${1:-run}"

if ! command -v cargo-watch &> /dev/null; then
    echo "[dev] cargo-watch not found. Install it:"
    echo "      cargo install cargo-watch"
    echo ""
    echo "[dev] Falling back to one-shot build + run (no hot reload)."
    case "$mode" in
      run)  RUNIE_MOCK_DELAY=1 cargo run --release ;;
      test) cargo test --all ;;
      fast) cargo run ;;
    esac
    exit 0
fi

watch_flags="-w crates -w Cargo.toml -w Cargo.lock"

case "$mode" in
  run)
    echo "[dev] Hot reload active. Edit any .rs file to restart."
    echo "[dev] Ctrl+C to stop."
    RUNIE_MOCK_DELAY=1 cargo watch -x 'run --release' $watch_flags
    ;;
  test)
    cargo test --all
    ;;
  fast)
    echo "[dev] Hot reload active (debug). Edit any .rs file to restart."
    echo "[dev] Ctrl+C to stop."
    cargo watch -x run $watch_flags
    ;;
  *)
    echo "Usage: $0"
    echo "  (no args)  hot reload release build with mock delays"
    echo "  test       run all tests"
    echo "  fast       hot reload debug build"
    ;;
esac
