# Enforce advertised file/function/complexity limits

**Status**: done
**Milestone**: R7
**Category**: Architecture / Testing
**Priority**: P2

**Depends on**: remove-direct-appstate-mutation-from-tui-handlers
**Blocks**: none

## Description

`AGENTS.md` claims the build enforces 500-line files, 40-line functions, and complexity ≤10, but `crates/runie-core/build.rs` only checks `AppState` field access and manifest checksums. Many production functions already violate the advertised limits.

## Root Cause

The linter was either removed or never fully implemented, while the documentation still claims strict enforcement.

## Acceptance Criteria

- [x] Either update `AGENTS.md` to match reality or implement the advertised checks in `build.rs`/CI.
- [x] If enforcing, fix or split the worst existing violations so the build passes.
- [x] If relaxing, document the rationale and remove the false claim.
- [x] `cargo test --workspace` passes.
- [x] `cargo check --workspace` has no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] N/A — resolution was to document limits as aspirational (not enforced) per AGENTS.md GUIDELINES section. The Layer 1 test `build_script_enforces_limits` would only be needed if enforcement was implemented.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] N/A — build/CI concern.

## Files touched

- `AGENTS.md` — Updated to document file/function/complexity limits as aspirational (GUIDELINES section)

## Validation

1. ✓ `AGENTS.md` correctly documents the limits as aspirational/guidelines
2. ✓ `cargo check --workspace` passes
3. ✓ `cargo test --workspace` passes

## Resolution

**Chosen approach:** Relax — updated `AGENTS.md` to document these limits as aspirational/guidelines, not enforced.

The limits (file ≤500 lines, function ≤40 lines, complexity ≤10) are now clearly documented in the **GUIDELINES (Not Enforced)** section of `AGENTS.md`. This is the correct approach because:

1. Enforcing these limits would require massive refactoring of working production code
2. The existing `build.rs` already enforces meaningful guardrails (AppState field access, magic numbers)
3. The aspirational targets remain as guidance for future development

**Remaining violations:** The known violations noted below still exist but are now explicitly documented as acceptable under the aspirational guidelines:
- `apply_to` (≈208 lines, complexity 30) — core message handling logic
- `parse` (281 lines, complexity 36) — diff parsing
- `check_destructive_tokens` (194 lines, complexity 81) — bash safety checks
- `render_thought_marker` (264 lines, complexity 36) — TUI thought rendering
- `render_canonical_diff` (241 lines, complexity 20) — TUI diff rendering

These functions work correctly and refactoring them for size would introduce risk without clear benefit.
