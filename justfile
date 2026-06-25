# Runie development commands
# Use `just --list` to see all available recipes.

# Default recipe - show help
default:
    just --list

# Run all workspace tests
test:
    cargo test --workspace

# Run tests with output visible
test-verbose:
    cargo test --workspace -- --nocapture

# Run clippy lints
lint:
    cargo clippy --all-targets --all-features -- -D warnings

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

# Run the schema generator example to regenerate config.schema.json
write-config-schema:
    cargo run -p runie-core --example write_config_schema --features schema -- config.schema.json

# Run clippy with auto-fix suggestions
lint-fix:
    cargo clippy --all-targets --all-features -- -D warnings -A clippy::all

# Clean build artifacts
clean:
    cargo clean

# Check entire workspace
check:
    cargo check --workspace

# Run dev mode (mock provider)
dev:
    ./dev.sh

# Run verify-tests script (same as CI)
verify-tests:
    ./scripts/verify-tests.sh

# Watch mode for TUI crate only
watch-tui:
    cargo watch -x 'check -p runie-tui' -w crates/runie-tui/src
