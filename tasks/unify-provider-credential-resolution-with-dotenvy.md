# Unify provider credential resolution with `dotenvy`

**Status**: todo
**Milestone**: R2
**Category**: Provider / Configuration
**Priority**: P1

**Depends on**: route-cli-config-through-configactor
**Blocks**: none

## Description

Provider credential resolution is duplicated between `crates/runie-provider/src/config/mod.rs` and `crates/runie-core/src/config/provider_config.rs`. Both implement `{PROVIDER}_API_KEY`/`{PROVIDER}_BASE_URL` env-var fallback, and the provider resolver also re-implements a `.env` parser. `goose` and `jcode` use `dotenvy` to load `.env` once. Runie should load `.env` centrally and delegate provider config resolution to the canonical `ProviderConfig` implementation.

## Acceptance Criteria

- [ ] Add `dotenvy` to `runie-core` or workspace dependencies.
- [ ] Load `.env` once at startup (CLI or `Leader::start`), before config is read.
- [ ] Delete the custom `.env` parser in `runie-provider/src/config/mod.rs`.
- [ ] Consolidate env-var fallback logic into one place (`runie-core/src/config/provider_config.rs` or `actors/config/file_helpers.rs`).
- [ ] `runie-provider` reads resolved credentials through the shared config, not by re-implementing lookups.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `dotenvy_loads_provider_api_key` — a `.env` file with `OPENAI_API_KEY` is reflected in `Config`.
- [ ] `env_var_overrides_config_file` — env vars still win over config file values.
- [ ] `custom_env_parser_removed` — no custom `.env` parsing code remains in `runie-provider`.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `crates/runie-provider/src/config/mod.rs`
- `crates/runie-core/src/config/provider_config.rs`
- `crates/runie-core/src/actors/config/file_helpers.rs`
- `crates/runie-core/Cargo.toml`
- `crates/runie-cli/src/main.rs` or `crates/runie-core/src/headless_runtime.rs` (central `.env` load)

## Notes

- `ctx7` confirms `dotenvy` is the recommended maintained fork of `dotenv`.
- This task should land after `route-cli-config-through-configactor.md` so credential resolution happens inside `RactorConfigActor` rather than in multiple places.
- `jcode` keeps provider credentials out of `config.toml` (env vars / provider env files) and stores MCP config separately; Runie should follow the same rule.
