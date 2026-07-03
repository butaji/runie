# Finish strum migration for remaining enums

## Status

`done`

## Context

Multiple enums still had manual `FromStr`, `Display`, `as_str` mappings: `commands/dsl/category.rs`, `proto/message/mod.rs`, `provider_event.rs`, `model/state/types.rs`, `agent_phase.rs`, `tool/search/types.rs`, `permissions/mod.rs`.

## Goal

Derive `strum::EnumString`, `Display`, and `IntoStaticStr`; remove manual match tables.

## What was done

### SearchMode (`crates/runie-agent/src/tool/search/types.rs`)
- Added `strum::Display` + `strum::EnumString` derives with `#[strum(serialize_all = "snake_case")]`
- Replaced manual `from_str` inherent method with one that delegates to strum's `FromStr` via `<SearchMode as std::str::FromStr>::from_str`
- Default-to-`Files` on unknown input preserved via `unwrap_or(Self::Files)`
- Added `strum` to `runie-agent/Cargo.toml` workspace deps

### CommandCategory (`crates/runie-core/src/commands/dsl/category.rs`)
- Added `strum::EnumString` with `#[strum(ascii_case_insensitive)]` to the derive macro
- Removed manual `FromStr` impl (conflicts with `EnumString` derive)
- Added `parse_case_insensitive` wrapper that tries strum first, then falls back to legacy aliases ("tool", "help", "unknown", "" → `System`)
- Updated all call sites in `declarative/types.rs` and `declarative/tests.rs` to use `parse_case_insensitive`
- `label()` and `as_str()` methods kept as-is

### Already done before this task
- `proto/message/mod.rs` — `Role` already has `strum::Display` + `strum::EnumString`
- `provider_event.rs` — `StopReason` already has `strum::Display`
- `model/state/types.rs` — `ThinkingLevel` already has `strum::EnumString`
- `permissions/mod.rs` — `PermissionMode` already has `strum::EnumString`

### Not migrated (intentional)
- `agent_phase.rs` — manual `Display` impl uses `tool:{name}` format which strum can't express

## Acceptance Criteria
- [x] Identify all target enums.
- [x] Add strum derives and aliases.
- [x] Delete manual impls (where possible without breaking backward compat).

## Tests

- **Layer 1 — State/Logic:** Unit tests for parsing and display round-trips pass.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** All affected crate tests pass.
- **Live tmux testing session (required):** N/A (changes are pure parsing/serialization).

## Completion Validation

- [x] **Unit tests** — `cargo test --workspace` passes (all 3048+ tests green).
- [x] **E2E tests** — All affected crate tests pass.
- [x] **Live tmux run tests** — N/A (pure parsing change).
