# Split `tick_animation` (25 lines) into Focused Methods

**Status**: stale
**Milestone**: R3
**Category**: Core Architecture
**Priority**: P2

## Resolution

Not started. `tick_animation` in `crates/runie-core/src/model/cache.rs` is 25 lines.
The task was written when it was 45 lines. It has since been partially refactored — the 4
sub-concerns (animation frame, speed window, input flash, transient expiry) are called from
the orchestrator. `animate_tokens` is a separate method. The 40-line linter cap is now
enforced via `build.rs`. This is now within acceptable limits.

Archived in tasks/archive/ as stale — low-value mechanical split that doesn't improve behavior.
