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

# Validate the source-backed Grok TUI component manifest.
parity-check:
    python3 scripts/validate-parity-index.py

# Run YAML e2e scenarios from crates/runie-tui/tests/e2e/*.yaml.
# Add or edit YAML files freely — no rebuild required.
e2e:
    cargo run -q -p runie-tui --bin runie-tui-e2e

# Run a single e2e scenario by filename.
e2e-one FILE:
    cargo run -q -p runie-tui --bin runie-tui-e2e -- {{FILE}}

# Capture one Herdr pane with full visible-grid ANSI attributes and metadata.
# Example: just herdr-dump w73:p38 /tmp/runie-hey
herdr-dump PANE PREFIX:
    scripts/herdr-dump.sh {{PANE}} {{PREFIX}}

# Compare two full-fidelity Herdr ANSI frames cell-by-cell.
herdr-compare LEFT RIGHT COLS="" ROWS="":
    scripts/compare-ansi-frames.py {{LEFT}} {{RIGHT}} {{ if COLS != "" { "--cols " + COLS } else { "" } }} {{ if ROWS != "" { "--rows " + ROWS } else { "" } }}

# Record a fixed-geometry application as a timed asciinema PTY cast.
# The private tmux session is isolated and removed when the command exits.
tmux-cast COLS ROWS CAST COMMAND PROMPT QUIT_KEY ENV="":
    scripts/tmux-asciinema-capture.sh {{COLS}} {{ROWS}} {{CAST}} {{COMMAND}} {{PROMPT}} {{QUIT_KEY}} {{ENV}}

# Replay and compare two asciinema casts through vt100 cell-by-cell.
cast-compare LEFT RIGHT:
    cargo run -q -p runie-tui --bin cast_compare -- {{LEFT}} {{RIGHT}}

cast-dump LEFT RIGHT:
    cargo run -q -p runie-tui --bin cast_compare -- --dump {{LEFT}} {{RIGHT}}

# Capture one application over the four parity reference geometries.
capture-matrix DIR COMMAND QUIT_KEY ENV="":
    scripts/capture-matrix.sh {{DIR}} {{COMMAND}} {{QUIT_KEY}} {{ENV}}

# Compare Grok and Runie casts at every supported geometry.
compare-matrix GROK_DIR RUNIE_DIR:
    scripts/compare-matrix.sh {{GROK_DIR}} {{RUNIE_DIR}}

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

# Full sweep: fmt-check + clippy + lint + test + parity manifest.
ci: fmt-check clippy lint test parity-check
    @echo "all green"
