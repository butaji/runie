#!/bin/bash
mode="${1:-run}"

# Ensure we use the rustup-managed toolchain (project requires nightly for
# the cranelift codegen flag in .cargo/config.toml). If the active `cargo`
# isn't the rustup proxy, prepend the toolchain bin from rustup settings.
# Override via RUNIE_TOOLCHAIN env var.
TOOLCHAIN="${RUNIE_TOOLCHAIN:-nightly}"
RUSTUP_TOOLCHAIN="$TOOLCHAIN" cargo --version >/dev/null 2>&1 || true

# Resolve the actual toolchain bin dir from rustup (e.g. nightly-aarch64-apple-darwin/bin)
RUSTUP_BIN=""
if command -v rustup >/dev/null 2>&1; then
    TC_DIR=$(rustup which --toolchain "$TOOLCHAIN" cargo 2>/dev/null | xargs dirname 2>/dev/null)
    if [ -n "$TC_DIR" ] && [ -x "$TC_DIR/cargo" ]; then
        RUSTUP_BIN="$TC_DIR"
    fi
fi

# Prepend toolchain bin (so nightly cargo wins) and $HOME/.cargo/bin (for cargo-watch).
export PATH="$RUSTUP_BIN:$HOME/.cargo/bin:$PATH"

# Tell cargo to use the nightly toolchain even if invoked via a non-proxy binary.
export RUSTUP_TOOLCHAIN="$TOOLCHAIN"

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
    cargo build --release -p runie-term 2>&1 | tail -2
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
