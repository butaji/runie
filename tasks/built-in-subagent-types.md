# Built-in subagent types as declarative data

**Status**: done
**Milestone**: R4
**Category**: Agent
**Priority**: P1

**Depends on**: leader-actor-shared-runtime
**Blocks**: none

## Summary

Ship built-in subagent types (`explore`, `plan`, `verify`, `check-work`) as declarative markdown files with YAML frontmatter. The subagent runner loads and executes them generically; users add custom types by dropping a file.

## File format

```markdown
---
name: explore
description: Fast codebase exploration for patterns and architecture.
prompt_mode: full
model: inherit
permission_mode: default
agents_md: true
---

You are an expert explorer. Search broadly, then narrow down. Use absolute paths.
Never create files unless explicitly requested.
```

Frontmatter fields:
- `name` — subagent type id.
- `description` — when to spawn this subagent.
- `prompt_mode` — `full` or `compact`.
- `model` — concrete model id, `inherit`, or `fast` trait.
- `permission_mode` — `default`, `acceptEdits`, `auto`, `dontAsk`, `bypassPermissions`, `plan`.
- `agents_md` — whether to inject project `AGENTS.md` into context.

The markdown body is the prompt template. Variables are interpolated with `{{variable}}`.

## Acceptance Criteria

- [x] Subagent type files live under `resources/agents/` (bundled defaults) and `~/.runie/agents/` (user overrides).
- [x] A manifest with SHA-256 checksums validates bundled resources at build time.
- [x] `SubagentRegistry` loads all types via `SubagentRegistry::from_builtins()` and `load_user_overrides()`.
- [x] Existing subagent runner dispatches through these definitions via `run_subagent_type()`.
- [x] Users can add a custom subagent type by creating a file; no Rust code changes.
- [x] `cargo check --workspace` is green.

## Remaining (blocked on leader-actor-shared-runtime)

- `AgentTypeRegistered` facts emitted by `ProviderActor` on startup (requires `LeaderActor`).

## Tests

### Layer 1 — State/Logic
- [x] `registry_loads_all_builtin_types` — 4 types loaded.
- [x] `explore_type_has_correct_fields` — name, description, prompt_mode, model, permission_mode, agents_md, body.
- [x] `plan_type_uses_plan_permission_mode` — Plan mode.
- [x] `verify_type_uses_compact_mode_and_auto_permission` — Compact + Auto.
- [x] `interpolate_replaces_variables` — `{{task}}` replacement.
- [x] `interpolate_preserves_unknown_placeholders` — unknown keys stay as-is.
- [x] `user_override_replaces_builtin` — custom type replaces built-in.
- [x] `parse_content_with_full_frontmatter` — all fields parsed.
- [x] `parse_content_minimal_frontmatter_uses_defaults` — defaults applied.
- [x] `parse_content_no_frontmatter_uses_hint_and_content` — no-frontmatter path.
- [x] `parse_content_multi_paragraph_body` — body extraction.
- [x] `permission_mode_from_str_all_variants` — all 6 modes.
- [x] `yaml_line_skips_empty_and_comments` — frontmatter parser.
- [x] `manifest_deserializes` — JSON deserialization.
- [x] `check_file_returns_true_on_match` — checksum match.
- [x] `check_file_returns_false_on_mismatch` — checksum mismatch.

### Layer 2 — Event Handling
- N/A — registry loading, not event handling.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] `explore_subagent_type_runs_with_mock_provider` — declarative type resolves and runs through mock provider.
- [x] `unknown_subagent_type_returns_error` — unknown type returns error.

## Files Changed

- `crates/runie-core/resources/agents/` — new directory with `explore.md`, `plan.md`, `verify.md`, `check-work.md`, `manifest.json`
- `crates/runie-core/src/subagents/` — new module: `mod.rs`, `manifest.rs`
- `crates/runie-core/Cargo.toml` — added `[build-dependencies]`: `sha2`, `hex`, `serde`, `serde_json`
- `crates/runie-core/build.rs` — added manifest checksum validation in `main()`
- `crates/runie-core/src/lib.rs` — added `pub mod subagents`
- `crates/runie-core/src/tests/arch_guardrails.rs` — added `subagents/` to production allow list
- `crates/runie-agent/src/subagent.rs` — added `run_subagent_type()`, `resolve_subagent_type()`, `build_type_command()`, `build_permission_gate()`, `run_subagent_turn_with_gate()`, and Layer 4 tests

## Notes

The `AgentTypeRegistered` fact emission is deferred until `leader-actor-shared-runtime` lands — `SubagentRegistry` is ready for actor integration but the emit call is not wired yet.
