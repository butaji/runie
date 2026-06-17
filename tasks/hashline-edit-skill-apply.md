# HashlineEditSkill Should Apply Validated Edits

**Status**: todo
**Milestone**: R3
**Category**: Tools
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`HashlineEditSkill::on_tool_call` detects hashline-formatted `edit_file` calls, validates hashes, and then returns `ToolCallResult::Continue`. The actual `edit_file` tool still runs with the legacy search/replace logic. The skill therefore provides no behavioral benefit beyond validation.

## Acceptance Criteria

- [ ] After validation, the skill applies the edits.
- [ ] The skill returns `SkipWithOutput` with the diff or final content.
- [ ] The legacy edit tool is not re-executed for hashline calls.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `hashline_edit_applies_valid_edits` — valid hashline edits modify the file.
- [ ] `hashline_edit_skips_legacy_path` — legacy search/replace is bypassed.

### Layer 2 — Event Handling
- [ ] `hashline_edit_emits_tool_result_event` — result event contains the applied diff.

### Layer 3 — Rendering
N/A.

### Layer 4 — Smoke / Crash
N/A.

## Files touched

- `crates/runie-core/src/harness_skills/hashline_edit.rs`

## Notes

Coordinate with `legacy-tool-enum-removal` so the edit tool path is consistent after cleanup.
