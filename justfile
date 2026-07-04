# Runie development commands
# Use `just --list` to see all available recipes.

# Default recipe - show help
default:
    just --list

# Run all workspace tests single-threaded to avoid tmux resource contention.
# Tests spawn tmux sessions which can conflict when run in parallel.
test:
    cargo test --workspace -- --test-threads=1

# Run tests with output visible
test-verbose:
    cargo test --workspace -- --test-threads=1 --nocapture

# Run doctests separately (nextest skips doctests by default)
test-doc:
    cargo test --workspace --doc

# Run all tests including doctests (for final verification)
test-all: test test-doc

# Run clippy lints
lint:
    cargo clippy --workspace --lib --bins -- -D warnings

# Check formatting
fmt:
    cargo fmt --all -- --check

# Format code
fmt-fix:
    cargo fmt --all

# Build all workspace crates
build:
    cargo build --workspace

# Build release binary
build-release:
    cargo build --release -p runie-tui

# Run the TUI (development).
# Pass `--mock` to enable the mock provider without an API key:
#   just tui --mock
#   just tui --mock --mock-model list_dir
#   just tui --mock --mock-model read_file --dry-run
tui *args:
    cargo run -p runie-tui --bin runie-tui -- {{args}}

# Run the schema generator example to regenerate config.schema.json
write-config-schema:
    cargo run -p runie-core --example write_config_schema -- config.schema.json

# Run clippy with auto-fix suggestions
lint-fix:
    cargo clippy --fix --workspace --allow-dirty --allow-staged -- -D warnings

# Clean build artifacts
clean:
    cargo clean

# Check entire workspace
check:
    cargo check --workspace

# Run all tests (same as CI)
verify-tests:
    cargo test --workspace -- --test-threads=1 && cargo test --workspace --doc

# Watch mode for TUI crate only
watch-tui:
    cargo watch -x 'check -p runie-tui' -w crates/runie-tui/src
