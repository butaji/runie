# Harness Skill: Hashline Edit Tool

**Status**: done
**Milestone**: R3
**Category**: Tools
**Priority**: P0

**Depends on**: harness-skill-framework, tool-registry-trait
**Blocks**: (none)

## Description

Replace the exact-string `search`/`replace` edit tool with a line-addressed format based on short content hashes. Research (Can Bölük’s oh-my-pi) shows that this single schema change can improve edit success by +5–17.5 pp and cut output tokens by ~20%, especially for models that struggle with whitespace-sensitive diffs.

When this skill is enabled, the `edit_file` tool accepts a list of line references (`line:hash`) for deletions/insertions/replacements. The server rejects references whose hash no longer matches, preventing stale edits.

## Acceptance Criteria

- [ ] A new `edit_file` schema variant (Hashline) is available when the skill is enabled.
- [ ] The server computes per-line short hashes and rejects mismatches with a clear error.
- [ ] The skill can be disabled via `~/.runie/config.toml` to fall back to exact-string search/replace.
- [ ] Existing edit behavior is preserved when the skill is disabled.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `hashline_parses_edit_request` — a Hashline edit request deserializes correctly.
- [ ] `hashline_rejects_stale_reference` — editing a line whose hash changed returns an error.
- [ ] `hashline_applies_clean_edit` — a valid Hashline edit mutates the file as expected.

### Layer 2 — Event Handling
- [ ] `hashline_edit_emits_tool_events` — the agent turn emits `ToolCallStart`/`ToolCallEnd` around a Hashline edit.

### Layer 3 — Rendering
- [ ] `hashline_edit_renders_in_tool_card` — the TUI shows the edit operation with line references.

### Layer 4 — Smoke / Crash
- [ ] `smoke_hashline_edit` — run binary, request an edit, verify the file changes.

## Files touched

- `crates/runie-core/src/skills/hashline_edit.rs`
- `crates/runie-core/src/tool/edit_file.rs`
- `crates/runie-agent/src/parser.rs`
- `crates/runie-agent/src/turn.rs`

## Notes

- Hash function should be cheap and stable across platforms (e.g., xxhash or fnv on trimmed line content).
- The skill registers the Hashline schema variant via the Skill framework; do not hard-code it in the agent turn.
- See `docs/adr/0022-harness-middleware-plugins.md`.
