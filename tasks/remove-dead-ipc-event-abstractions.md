# Remove dead IPC/event-shaping abstractions

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

Several IPC and event-shaping abstractions are no longer used by the active runtime path. `crates/runie-protocol/src/event.rs` (114 lines) is referenced only by `ipc.rs` and a protocol test. `crates/runie-core/src/ipc.rs` (56 lines) defines `CoreIpc` and `TuiQueueEnds`, which have no live callers. `crates/runie-core/src/channels.rs` (386 lines) defines `TextChannel`, `ToolCallChannel`, `ReasoningChannel`, and `ChannelDecoder`, which are consumed only by `bus.rs`'s `subscribe_channel` and by internal tests. This task removes these dead modules, updates all remaining callers and exports, and merges any genuinely useful ideas from `channels.rs` into the TUI render path if they simplify the codebase.

## Acceptance Criteria

- [ ] Delete `crates/runie-protocol/src/event.rs` and remove its module declaration and exports from `crates/runie-protocol/src/lib.rs`.
- [ ] Delete `crates/runie-core/src/ipc.rs` and remove `CoreIpc` / `TuiQueueEnds` from the `runie-core` public API.
- [ ] Delete `crates/runie-core/src/channels.rs` or merge its useful concepts into the TUI render path; remove the `subscribe_channel` entry point in `bus.rs` if it becomes unused.
- [ ] Update all call sites, re-exports, tests, and documentation that reference the deleted types.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `no_ipc_types_in_public_api` — enumerates the `runie-core` and `runie-protocol` public exports and asserts that `CoreIpc`, `TuiQueueEnds`, `TextChannel`, `ToolCallChannel`, `ReasoningChannel`, and `ChannelDecoder` are no longer present.

### Layer 2 — Event Handling
- [ ] N/A — the removed types are not part of the live event-dispatch path.

### Layer 3 — Rendering
- [ ] N/A unless useful `channels.rs` helpers are merged into TUI rendering, in which case add `channel_facts_render_without_decoder` — verifies that the same TUI output is produced after the merge.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `bus_facts_still_reach_tui_without_channel_decoder` — runs a full provider replay turn that emits text, tool-call, and reasoning facts and asserts that the TUI still receives and renders them correctly after `ChannelDecoder` is removed.

## Files touched

- `crates/runie-protocol/src/event.rs` (delete)
- `crates/runie-protocol/src/lib.rs`
- `crates/runie-core/src/ipc.rs` (delete)
- `crates/runie-core/src/channels.rs` (delete or partially merge)
- `crates/runie-core/src/bus.rs`
- `crates/runie-core/src/lib.rs`
- Any tests or documentation referencing the deleted types.

## Notes

- If `channels.rs` helpers turn out to be valuable for TUI rendering, extract the minimal logic into `crates/runie-tui/src/render/` rather than keeping a dead module in `runie-core`.
- Rejected alternative: marking the modules `#[doc(hidden)]` and leaving them in place. That would preserve dead code and still require maintenance; deletion keeps the architecture honest.
- Out of scope: redesigning the live bus protocol or adding new IPC mechanisms. Only remove unused abstractions.
