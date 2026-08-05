default:
    @just --list

# Run the TUI binary.
tui:
    cargo run -p runie-tui --bin runie

# Build everything.
build:
    cargo build --workspace

# Check everything (faster than build, runs lints).
check:
    cargo check --workspace --all-targets

# Run all tests.
test:
    cargo test --workspace

# Run all tests via cargo-nextest (parallel, per-test process).
test-fast:
    cargo nextest run --workspace

# Re-run only failing tests.
test-failed:
    cargo nextest run --workspace --no-fail-fast --failure-output=immediate-final 2>&1 | tail -200 || true

# Run only runie-core tests.
test-core:
    cargo test -p runie-core

# Run only runie-tui tests.
test-tui:
    cargo test -p runie-tui

# Run YAML e2e scenarios from crates/runie-tui/tests/e2e/*.yaml.
# Add or edit YAML files freely — no rebuild required.
e2e:
    cargo run -q -p runie-tui --bin runie-tui-e2e

# Run a single e2e scenario by filename.
e2e-one FILE:
    cargo run -q -p runie-tui --bin runie-tui-e2e -- {{FILE}}

# Format the code.
fmt:
    cargo fmt --all

# Check formatting (CI mode).
fmt-check:
    cargo fmt --all -- --check

# Run clippy.
clippy:
    cargo clippy --workspace --all-targets -- -D warnings

# Run the project linter.
lint:
    cargo run -p lint-check

# Full sweep: fmt-check + clippy + lint + test.
ci: fmt-check clippy lint test
    @echo "all green"