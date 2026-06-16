#!/bin/bash
mode="${1:-run}"

# Default to the stable toolchain. Override via RUNIE_TOOLCHAIN env var.
TOOLCHAIN="${RUNIE_TOOLCHAIN:-stable}"
RUSTUP_TOOLCHAIN="$TOOLCHAIN" cargo --version >/dev/null 2>&1 || true

# Resolve the actual toolchain bin dir from rustup.
RUSTUP_BIN=""
if command -v rustup >/dev/null 2>&1; then
    TC_DIR=$(rustup which --toolchain "$TOOLCHAIN" cargo 2>/dev/null | xargs dirname 2>/dev/null)
    if [ -n "$TC_DIR" ] && [ -x "$TC_DIR/cargo" ]; then
        RUSTUP_BIN="$TC_DIR"
    fi
fi

# Prepend toolchain bin (so selected cargo wins) and $HOME/.cargo/bin (for cargo-watch).
export PATH="$RUSTUP_BIN:$HOME/.cargo/bin:$PATH"

# Tell cargo to use the selected toolchain even if invoked via a non-proxy binary.
export RUSTUP_TOOLCHAIN="$TOOLCHAIN"

if ! command -v cargo-watch > /dev/null 2>&1; then
    echo "cargo-watch not found. Install: cargo install cargo-watch"
    exit 1
fi

# Parse flags
export RUNIE_MOCK=""
export RUNIE_MOCK_DELAY=""
case "$mode" in
  run|run-delay|fast|fast-delay)
    # dev.sh enables the mock provider. Production (no dev.sh) has no
    # mock fallback — the app requires a real provider or prompts login.
    export RUNIE_MOCK=1
    ;;
esac
case "$mode" in
  run-delay|fast-delay)
    export RUNIE_MOCK_DELAY=1
    ;;
esac

case "$mode" in
  run|run-delay)
    echo "[dev] Hot reload (release, RUNIE_MOCK=1). Ctrl+C to stop."
    cargo watch -x 'run --release -p runie-tui' -w crates
    ;;
  fast|fast-delay)
    echo "[dev] Hot reload (debug, RUNIE_MOCK=1). Ctrl+C to stop."
    cargo watch -x 'run -p runie-tui' -w crates
    ;;
  test)
    cargo test --all
    ;;
  smoke)
    echo "[dev] Running smoke tests..."
    cargo build --release -p runie-tui 2>&1 | tail -2
    ./scripts/smoke-tmux.sh
    echo "[dev] All smoke tests passed!"
    ;;
  *)
    echo "Usage: \$0 [run|run-delay|fast|fast-delay|test|smoke]"
    echo ""
    echo "Modes:"
    echo "  run        - release build, mock enabled, no streaming delays"
    echo "  run-delay  - release build, mock enabled, 0.5s-3s delays between chunks"
    echo "  fast       - debug build, mock enabled, no streaming delays"
    echo "  fast-delay - debug build, mock enabled, 0.5s-3s delays between chunks"
    echo "  test       - run all tests"
    echo "  smoke      - release build + tmux smoke test"
    echo ""
    echo "Without dev.sh: production mode. No mock provider. The app requires"
    echo "a real provider configured or auto-opens the login dialog on startup."
    ;;
esac
