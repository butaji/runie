# Deserialize declarative command YAML with typed structs

**Status**: done
**Note**: Verified 2026-06-29 — YAML deserialization with typed structs implemented.
**Milestone**: R6
**Category**: Core / Declarative DSL
**Priority**: P2

**Depends on**: use-pulldown-cmark-frontmatter-for-resource-loader
**Blocks**: move-built-in-slash-commands-to-declarative-yaml

## Description

`declarative::loader::parse_command_yaml` manually walks `serde_yaml::Mapping` and `parse_triggers` parses trigger lists as flat strings. Add typed `#[derive(Deserialize)]` structs and deserialize `triggers` as a real YAML list.

## Acceptance Criteria

- [x] Define `DeclarativeCommandYaml` (or similar) with `serde::Deserialize`.
- [x] Deserialize `triggers` as `Vec<Trigger>`.
- [x] Delete `parse_category` — replaced with `CommandCategory::from_str` using `std::str::FromStr`.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `declarative_command_yaml_deserializes` — a YAML command spec parses correctly.
- [x] `declarative_command_triggers_as_list` — triggers are a YAML list, not flat strings.
- [x] `command_category_from_str_round_trip` — case-insensitive category parsing with FromStr.

### Layer 2 — Event Handling
- [x] N/A.

### Layer 3 — Rendering
- [x] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] N/A.

## Files touched

- `crates/runie-core/src/declarative/loader.rs`
- `crates/runie-core/src/declarative/types.rs`
- `crates/runie-core/src/commands/dsl/category.rs`

## Notes

- Coordinate with `replace-remaining-custom-parsers-and-macros-with-strum.md` for `CommandCategory`.
