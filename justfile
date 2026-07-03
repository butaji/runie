# Runie development commands
# Use `just --list` to see all available recipes.

# Default recipe - show help
default:
    just --list

# Run all workspace tests with nextest (120s slow timeout per test)
# Override slow-timeout: just test SLOW_TIMEOUT=60
# Pass through to nextest: just test -- --no-fail-fast
test:
    cargo nextest run --workspace

# Run tests with output visible
test-verbose:
    cargo nextest run --workspace --no-capture

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
    cargo nextest run --workspace && cargo test --workspace --doc

# Run structural linter (file/function/complexity limits)
check-structure:
    python3 scripts/check_structure.py

# Watch mode for TUI crate only
watch-tui:
    cargo watch -x 'check -p runie-tui' -w crates/runie-tui/src
