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

# Quality assurance: format check, lint, and test.
qa: fmt lint test

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

# Build release binaries (runie-tui and runie CLI)
build-release:
    cargo build --release -p runie-tui -p runie-cli

# Run the TUI (development).
# Pass `--mock` to enable the mock provider without an API key:
#   just tui --mock
#   just tui --mock --mock-model list_dir
#   just tui --mock --mock-model read_file --dry-run
tui *args:
    cargo run -p runie-tui --bin runie-tui -- {{args}}

# Run the TUI with real minimax API (keys from `pass`).
# Creates an isolated $HOME so ~/.runie is not polluted.
# Usage: just tui-live [args...]
tui-live *args:
	#!/usr/bin/env bash
	set -euo pipefail
	TEST_HOME=$(mktemp -d)
	CONFIG_DIR="$TEST_HOME/.runie"
	mkdir -p "$CONFIG_DIR"
	AUTH_FILE="$CONFIG_DIR/auth.json"
	CONFIG_FILE="$CONFIG_DIR/config.toml"

	MINIMAX_KEY=$(pass show minimax/key 2>/dev/null | head -1)
	if [[ -z "$MINIMAX_KEY" ]]; then echo "ERROR: minimax key not found in pass"; exit 1; fi

	GOOGLE_KEY=$(pass show google/key 2>/dev/null | head -1)
	ANTHROPIC_KEY=$(pass show anthropic/key 2>/dev/null | head -1)

	printf '{"minimax": {"token": "%s"}, "google": {"token": "%s"}, "anthropic": {"token": "%s"}}\n' \
		"$MINIMAX_KEY" "$GOOGLE_KEY" "$ANTHROPIC_KEY" > "$AUTH_FILE"

	# NOTE: printf avoids heredoc/TOML-parsing issues in just 1.56.0
	printf '%s\n' \
		'[mode]' \
		'active = "single"' \
		'' \
		'[models]' \
		'default = "MiniMax-M2.7-highspeed"' \
		'' \
		'[model_providers.minimax]' \
		'base_url = "https://api.minimaxi.chat/v1"' \
		'models = ["MiniMax-M2.7-highspeed", "MiniMax-M3"]' \
		'' \
		'[model_providers.google]' \
		'base_url = "https://generativelanguage.googleapis.com/v1beta"' \
		'models = ["gemini-2.0-flash", "gemini-2.5-pro"]' > "$CONFIG_FILE"

	export HOME="$TEST_HOME"
	export MINIMAX_API_KEY="$MINIMAX_KEY"
	export RUNIE_AUTH_FILE="$AUTH_FILE"
	export NO_COLOR=""
	unset XDG_CONFIG_HOME XDG_DATA_HOME XDG_CACHE_HOME XDG_STATE_HOME

	cargo run -p runie-tui --bin runie-tui -- {{args}}; RC=$?
	rm -rf "$TEST_HOME"
	exit $RC

# Same as tui-live but starts in swarm/delegation mode.
tui-live-swarm *args:
	#!/usr/bin/env bash
	set -euo pipefail
	TEST_HOME=$(mktemp -d)
	CONFIG_DIR="$TEST_HOME/.runie"
	mkdir -p "$CONFIG_DIR"
	AUTH_FILE="$CONFIG_DIR/auth.json"
	CONFIG_FILE="$CONFIG_DIR/config.toml"

	MINIMAX_KEY=$(pass show minimax/key 2>/dev/null | head -1)
	if [[ -z "$MINIMAX_KEY" ]]; then echo "ERROR: minimax key not found in pass"; exit 1; fi

	printf '{"minimax": {"token": "%s"}}\n' "$MINIMAX_KEY" > "$AUTH_FILE"

	# NOTE: printf avoids heredoc/TOML-parsing issues in just 1.56.0
	printf '%s\n' \
		'[mode]' \
		'active = "swarm"' \
		'' \
		'[models]' \
		'default = "MiniMax-M2.7-highspeed"' \
		'' \
		'[model_providers.minimax]' \
		'base_url = "https://api.minimaxi.chat/v1"' \
		'models = ["MiniMax-M2.7-highspeed", "MiniMax-M3"]' > "$CONFIG_FILE"

	export HOME="$TEST_HOME"
	export MINIMAX_API_KEY="$MINIMAX_KEY"
	export RUNIE_AUTH_FILE="$AUTH_FILE"
	export NO_COLOR=""
	unset XDG_CONFIG_HOME XDG_DATA_HOME XDG_CACHE_HOME XDG_STATE_HOME

	cargo run -p runie-tui --bin runie-tui -- {{args}}; RC=$?
	rm -rf "$TEST_HOME"
	exit $RC

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
