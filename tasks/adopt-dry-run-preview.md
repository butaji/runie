# Adopt Dry-Run Preview Mode

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Priority**: P2

**Depends on**: (none)
**Blocks**: (none)

## Description

Implement `--dry-run` / `--preview` mode that validates configuration and shows what would happen without executing:

```
runie --dry-run "list files"
```

Output:
```
✓ Config valid
✓ Provider: anthropic/claude-sonnet-4-20250514
✓ Tools: read, write, edit, bash, glob, grep, search
✓ Skills: 12 loaded from ~/.runie/skills/
✓ MCP servers: none configured
✓ Permissions: auto mode (file writes require approval)
⚠ No model calls made (dry-run)
```

Reference: `~/Code/agents/openharness/` dry-run implementation

## Acceptance Criteria

- [ ] `--dry-run` flag parsed and validated.
- [ ] Config loading without execution.
- [ ] Provider resolution without API calls.
- [ ] Tool registration validated.
- [ ] Skill loading validated.
- [ ] Permission mode displayed.
- [ ] Returns `ready` / `warning` / `blocked` status.
- [ ] No LLM API calls made.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `dry_run_validates_config` — invalid config returns error.
- [ ] `dry_run_resolves_provider` — provider resolution without API call.
- [ ] `dry_run_loads_skills` — skills loaded but not executed.
- [ ] `dry_run_no_llm_calls` — verify no API calls made.

### Layer 2 — Event Handling
- [ ] `dry_run_flag_triggers_preview_mode` — flag activates dry-run path.

### Layer 3 — Rendering
- [ ] `dry_run_output_shows_status` — preview output rendered.

### Layer 4 — Smoke / Crash
- [ ] Smoke test: `runie --dry-run "help"` runs without panic.

## Files touched

- `crates/runie-core/src/dry_run.rs` (new)
- `crates/runie-term/src/main.rs` — add flag
- `crates/runie-tui/src/app.rs` — add flag

## Notes

Useful for debugging configuration issues and CI validation.
