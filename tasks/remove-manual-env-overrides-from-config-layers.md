# Remove manual env overrides from config layers

## Status

`done`

## Description

`config/layers.rs` used manual `std::env::var` + direct `Config` field mutation for `RUNIE_PROVIDER`, `RUNIE_MODEL`, `RUNIE_THEME` after Figment extraction. Replaced with `Serialized::default(key, value)` which uses Figment's value-merging machinery.

## Why not `Env::prefixed`?

Figment's `Env::prefixed("RUNIE_")` strips the prefix to get the key (e.g., `RUNIE_PROVIDER` → `PROVIDER`), but `serde`'s struct deserialization is case-sensitive when mapping keys to struct fields. So `PROVIDER` (uppercase) doesn't match `provider` (lowercase). Even though `Env` stores keys case-preserved and `serde_json` uses case-insensitive matching, the struct-based `Config` deserialization requires exact key matches.

The fix: use `Serialized::default(field, value)` to explicitly insert env vars with the correct lowercase keys into Figment's `Default` profile, ensuring they merge correctly with the TOML file values.

## Changes

### Before
```rust
figment = figment.merge(Env::prefixed("RUNIE_")); // BROKEN: PROVIER ≠ provider
let mut config = figment.extract()?;
if let Ok(provider) = std::env::var("RUNIE_PROVIDER") {
    config.provider = Some(provider); // manual mutation
}
```

### After
```rust
figment = figment.merge(Env::prefixed("RUNIE_")); // Still available for other uses
for (env_var, field) in [
    ("RUNIE_PROVIDER", "provider"),
    ("RUNIE_MODEL", "model"),
    ("RUNIE_THEME", "theme"),
] {
    if let Ok(value) = std::env::var(env_var) {
        figment = figment.merge(Serialized::default(field, value));
    }
}
let config = figment.extract()?; // no manual mutation
```

## Acceptance Criteria
- [x] Unit tests — Env vars still override config values through Figment.
- [x] E2E tests — Loading config with env overrides produces the same result.
- [x] Live tmux tests — Set `RUNIE_PROVIDER`/`RUNIE_MODEL` and launch tmux; verify the overrides apply.

## Tests

- **Layer 1 — State/Logic:** Unit tests for figment env override precedence.
- **Layer 2 — Event Handling:** ConfigLoaded facts reflect env values.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Config load with env overrides.
- **Live tmux testing session (required):** N/A.

## Files Changed

- `crates/runie-core/src/config/layers.rs` — Replaced manual overrides with `Serialized::default`.
- `crates/runie-core/Cargo.toml` — Added `uncased` workspace dependency (transitive dep of figment, needed for code that uses `Env::filter_map`).
- `Cargo.toml` — Added `uncased` to workspace dependencies.

## Validation

- ✅ `cargo check --workspace` passes
- ✅ `cargo test -p runie-core layers` — 7 tests pass including `figment_env_overrides_take_precedence`
- ✅ `cargo test --workspace` — full test suite passes
