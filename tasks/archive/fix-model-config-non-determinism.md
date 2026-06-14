# Fix Non-Deterministic `selected_models` in Login Flow

**Status**: stale
**Milestone**: R3
**Category**: Configuration
**Priority**: P2

## Resolution

Not started. `crates/runie-core/src/login_flow/state.rs` still uses `HashSet<String>` for
`selected_models`. The non-deterministic iteration order means saving config rewrites the
`models` array in a different order each time. The fix is mechanical: replace `HashSet`
with `BTreeSet` or add a `selection_order: Vec<String>` field.

Not done. Marked stale — it's a minor cosmetic issue (git diff noise in config.toml) that
doesn't affect functionality. Can be revisited when the login flow is next touched.

Archived in tasks/archive/ as stale.
