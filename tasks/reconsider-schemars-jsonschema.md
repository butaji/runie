# Reconsider schemars + jsonschema config validation

**Status**: todo
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

Either (a) move schema generation to a dev/example-only dep and replace runtime validation with a hand-written checker, (b) gate both behind a `validate` feature (off by default), or (c) keep and document why runtime schema validation is required.

## Acceptance Criteria

- [ ] Decision made: EITHER
  - (a) **Move to dev/example** — `schemars` becomes a `[dev-dependencies]` / example-only dep used by `examples/write_config_schema.rs`; `jsonschema` removed entirely; `Config::validate` rewritten as a hand-written checker over the known sections; OR
  - (b) **Feature-gate** — `validate` feature added, both deps optional, default build excludes them; OR
  - (c) **Keep + document** — a concrete reason runtime schema validation is required is written into `config.rs` module docs.
- [ ] If (a) or (b): default `cargo build --workspace` no longer pulls `jsonschema` (and ideally `schemars`).
- [ ] `config.schema.json` stays in the repo and stays regenerable via `cargo run --example write_config_schema` (option a) or the existing recipe.
- [ ] `Config::load_strict` still rejects malformed configs with a clear error message.
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `handwritten_checker_rejects_unknown_provider` — `Config::load_strict` on a TOML with `provider = "nonsense"` returns an error naming the bad field.
- [ ] `handwritten_checker_rejects_bad_truncation` — a non-integer `truncation.max_tokens` is rejected.
- [ ] `handwritten_checker_accepts_valid_config` — the existing valid-config fixture loads cleanly.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_schema_still_regenerates` — `cargo run --example write_config_schema` produces a `config.schema.json` byte-identical (or semantically equal) to the committed one.
- [ ] `smoke_default_build_excludes_jsonschema` — `cargo build --workspace` does not pull `jsonschema`.

## Files touched

- `crates/runie-core/src/config.rs` (rewrite `validate` if option a)
- `crates/runie-core/Cargo.toml` (move `schemars` to dev-deps, remove `jsonschema` if option a)
- `crates/runie-core/examples/write_config_schema.rs` (keep as the schema generator)
- `crates/runie-core/src/config/schema.rs` (schemars derives stay, but only compiled under dev / example)

## Notes

`adopt-layered-config` notes list `Config::validate/validate_toml/load_strict` + `fallback_providers/provider_chain` as the additions. Only the validation half is in scope here; the fallback-provider logic is a separate concern and stays. If option (c), link justification and close as `wontfix`.
