# Delete config_reload re-export shim

**Status**: todo
**Milestone**: R4
**Category**: Configuration
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-core/src/config_reload/` is a 2-file backward-compat re-export shim. `mod.rs` (7 lines) is pure `pub use types::{...}`; `types.rs` (21 lines) re-exports `crate::config::{config_path, Config, ConfigChange, TruncationSection}` plus 3 type aliases (`TelemetrySection`/`PromptsSection`/`UiSection`) all `#[allow(dead_code)]` with zero callers. Only 4 real users of the re-export exist. The doc comment admits it is legacy.

## Acceptance Criteria

- [ ] 4 callers rewritten to `crate::config::X` / `runie_core::config::X`: `runie-print/src/main.rs:66,93`, `runie-core/src/update/system.rs:300`, `runie-core/src/state/session.rs:58,97`.
- [ ] `crates/runie-core/src/config_reload/` deleted.
- [ ] `pub mod config_reload;` removed from `crates/runie-core/src/lib.rs`.
- [ ] 3 dead type aliases gone.
- [ ] `rg "config_reload" crates/` returns zero hits.
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- N/A — import-path cleanup.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_config_load_after_shim_removal` — config still loads from disk via the canonical path.

## Files touched

- `crates/runie-core/src/config_reload/mod.rs`
- `crates/runie-core/src/config_reload/types.rs`
- `crates/runie-core/src/lib.rs`
- `crates/runie-print/src/main.rs`
- `crates/runie-core/src/update/system.rs`
- `crates/runie-core/src/state/session.rs`
- `crates/runie-agent/src/truncate.rs` (doc comment)

## Notes

Pure legacy import path. No behavior change.
