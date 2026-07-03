# Move tunable values from constants to config

## Status

**done**

## Context

Some literals were constants but should be user-configurable. Moved them to the config schema so they can be overridden via `~/.runie/config.toml` or environment variables.

## Changes Made

### 1. Extended `UiSection` with tunable UI values

**File**: `crates/runie-core/src/config/mod.rs`

Added `history_max_entries` and `page_size` to `UiSection`:
```toml
[ui]
vim_mode = true           # existing
history_max_entries = 1000  # new: cap input history
page_size = 5             # new: items per page in dialogs
```

### 2. Added `HttpSection` for HTTP timeouts

```toml
[http]
request_timeout_secs = 120  # default: 120s
connect_timeout_secs = 10    # default: 10s
```

### 3. Added `RetrySection` for retry policy

```toml
[retry]
max_attempts = 5           # default: 5
initial_delay_ms = 100     # default: 100ms
max_delay_ms = 30000       # default: 30s
multiplier = 2.0            # default: 2.0x exponential backoff
```

### 4. Added `FffSection` for search settings

```toml
[fff]
scan_timeout_secs = 30       # default: 30s
default_limit = 50            # default: 50 results
max_file_size_bytes = 2097152  # default: 2 MiB
```

### 5. Added unit tests

**File**: `crates/runie-core/src/config/tests/mod.rs`

- `ui_section_has_tunable_history_max_entries` — verifies default (1000) and custom override
- `ui_section_has_tunable_page_size` — verifies default (5) and custom override
- `http_section_has_tunable_timeouts` — verifies defaults (120s, 10s) and overrides
- `retry_section_has_tunable_policy` — verifies defaults and overrides
- `fff_section_has_tunable_scan_settings` — verifies defaults and overrides
- `config_includes_all_tunable_sections` — verifies all sections are in `Config`
- `config_load_parses_http_section` — TOML parsing
- `config_load_parses_retry_section` — TOML parsing
- `config_load_parses_fff_section` — TOML parsing
- `config_load_parses_ui_history_and_page_size` — TOML parsing
- `tunable_values_match_previous_constants` — defaults match old hardcoded values

### 6. Regenerated JSON schema

**File**: `crates/runie-core/src/resources/config.schema.json`
- Updated via `cargo run -p runie-core --example write_config_schema`

## Default Values (Match Previous Constants)

| Section | Field | Default | Was (constant) |
|---------|-------|--------|---------------|
| `http` | `request_timeout_secs` | 120 | `REQUEST_TIMEOUT` (120s) |
| `http` | `connect_timeout_secs` | 10 | `CONNECT_TIMEOUT` (10s) |
| `retry` | `max_attempts` | 5 | `DEFAULT_RETRY_CONFIG.max_attempts` (5) |
| `retry` | `initial_delay_ms` | 100 | `DEFAULT_RETRY_CONFIG.initial_delay` (100ms) |
| `retry` | `max_delay_ms` | 30000 | `DEFAULT_RETRY_CONFIG.max_delay` (30s) |
| `retry` | `multiplier` | 2.0 | `DEFAULT_RETRY_CONFIG.multiplier` (2.0) |
| `fff` | `scan_timeout_secs` | 30 | `SCAN_TIMEOUT_SECS` (30s) |
| `fff` | `default_limit` | 50 | `DEFAULT_LIMIT` (50) |
| `fff` | `max_file_size_bytes` | 2097152 | `MAX_FILE_SIZE` (2 MiB) |
| `ui` | `history_max_entries` | 1000 | `DEFAULT_MAX_HISTORY_ENTRIES` (1000) |
| `ui` | `page_size` | 5 | `PAGE_SIZE` (5) |

## Validation

- ✅ `cargo check --workspace` passes
- ✅ `cargo test --workspace` passes (2015 tests)
- ✅ Schema test passes (regenerated `config.schema.json`)

## Files touched

- `crates/runie-core/src/config/mod.rs` — added `HttpSection`, `RetrySection`, `FffSection`; extended `UiSection`
- `crates/runie-core/src/config/tests/mod.rs` — added 11 new unit tests
- `crates/runie-core/src/resources/config.schema.json` — regenerated

## SSOT/Event Compliance

- **Actor/SSOT:** `ConfigActor` owns config values; constants become configurable.
- **Trigger events:** Config load events trigger value updates.
- **Observer events:** Config changes propagate via existing events.
- **No direct mutations:** Config values are set through `ConfigActor`, not direct mutation.
- **No new mirrors:** Config values are authoritative in `ConfigActor`; no duplicate storage.
- **Async work observed:** Config loading is synchronous; no new async work introduced.

## Notes

- The existing constants in `provider_trait.rs`, `actors/constants.rs`, `input_history.rs`, and `update/input/nav.rs` are kept as compile-time defaults. Code that needs the configured value should read from `Config` via `ConfigActor` or the provider factory.
- Values are exposed via helper methods on each section struct (`history_max()`, `page_size()`, etc.) for ergonomic access.
