#!/bin/bash
mode="${1:-run}"
export PATH="$HOME/.cargo/bin:$PATH"

if ! command -v cargo-watch > /dev/null 2>&1; then
    echo "cargo-watch not found. Install: cargo install cargo-watch"
    exit 1
fi

case "$mode" in
  run)
    echo "[dev] Hot reload (release). Ctrl+C to stop."
    RUNIE_MOCK_DELAY=1 cargo watch -x 'run --release --bin runie' -w crates
    ;;
  fast)
    echo "[dev] Hot reload (debug). Ctrl+C to stop."
    cargo watch -x 'run --bin runie' -w crates
    ;;
  test)
    cargo test --all
    ;;
  *)
    echo "Usage: \$0 [run|fast|test]"
    ;;
esac
