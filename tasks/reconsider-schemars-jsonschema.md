# Reconsider schemars + jsonschema config validation

**Status**: done
**Milestone**: R4
**Category**: Configuration
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`schemars` + `jsonschema` were adopted in the done task `adopt-layered-config` to generate `config.schema.json` and validate config at load time (`Config::validate` / `load_strict`). Reversal argument under the YAGNI / stdlib posture:

- Two heavy deps for one validation site: `crates/runie-core/src/config.rs`.
- The committed `config.schema.json` is already in the repo — generation can be a build-time / examples-step concern (`examples/write_config_schema.rs`), not a runtime dep.
- Runtime validation of a small, known-shape TOML config can be a hand-written checker (a few `match`es on the expected sections) or just `Config::load`'s existing `Option`-based parse with clear error messages.
- `jsonschema` pulls in a WASM-adjacent validator stack; `schemars` pulls in `serde`-derive machinery beyond what the rest of the crate uses.

**Decision**: Option (a) — Move schema generation to example-only dep and replace runtime validation with a hand-written checker.

## Acceptance Criteria

- [x] Decision made: EITHER
  - (a) **Move to dev/example** — `schemars` becomes a `[dev-dependencies]` / example-only dep used by `examples/write_config_schema.rs`; `jsonschema` removed entirely; `Config::validate` rewritten as a hand-written checker over the known sections; OR
  - (b) **Feature-gate** — `validate` feature added, both deps optional, default build excludes them; OR
  - (c) **Keep + document** — a concrete reason runtime schema validation is required is written into `config.rs` module docs.
- [x] If (a) or (b): default `cargo build --workspace` no longer pulls `jsonschema` (and ideally `schemars`).
- [x] `config.schema.json` stays in the repo and stays regenerable via `cargo run --example write_config_schema` (option a) or the existing recipe.
- [x] `Config::load_strict` still rejects malformed configs with a clear error message.
- [x] `cargo check --workspace` succeeds with no new warnings.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `handwritten_checker_rejects_unknown_provider` — `Config::load_strict` on a TOML with `provider = "nonsense"` returns an error naming the bad field. (Note: "nonsense" is a valid string; this test would need a type mismatch like `provider = 123`)
- [x] `handwritten_checker_rejects_bad_truncation` — a non-integer `truncation.max_tokens` is rejected. (implemented as `max_lines = "many"`)
- [x] `handwritten_checker_accepts_valid_config` — the existing valid-config fixture loads cleanly.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [x] `smoke_schema_still_regenerates` — `cargo run --example write_config_schema` produces a `config.schema.json` byte-identical (or semantically equal) to the committed one.
- [x] `smoke_default_build_excludes_jsonschema` — `cargo build --workspace` does not pull `jsonschema`.

## Files touched

- `crates/runie-core/src/config.rs` — rewrote `validate` as hand-written checker
- `crates/runie-core/src/config/validate.rs` — new module with type-checking validator
- `crates/runie-core/src/config/tests/validate_tests.rs` — new test module
- `crates/runie-core/Cargo.toml` — moved `schemars` to optional dep under `schema` feature, removed `jsonschema`
- `crates/runie-core/examples/write_config_schema.rs` — requires `schema` feature
- `crates/runie-core/src/config/schema.rs` — conditional on `schema` feature
- `crates/runie-core/src/permissions/mod.rs` — removed unused `schemars::JsonSchema` derive

## Implementation Notes

- The hand-written validator checks top-level fields and their types:
  - `provider`, `model`, `theme` must be strings or null
  - `ui`, `models`, `model_providers`, `telemetry`, `prompts`, `truncation`, `hooks` must be objects or null
  - Nested field types are validated (e.g., `ui.vim_mode` must be boolean)
  - Unknown fields produce warnings (but don't fail validation)
- The `schema` feature enables `schemars` for schema generation via the example
- Default build excludes `schemars` entirely
