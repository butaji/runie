# p32 — Actor-owned tool-row identity

Status: in_progress (2026-08-06)

## Fresh replay audit (2026-08-06)

The source-to-event audit was repeated against the current implementation. The
YAML runner first records the loop bus, then appends only declarative control
events; ordinary `tool_call` entries are provider assistant events and do not
directly append scrollback rows. During replay,
`scrollback_messages_for_event(ToolExecutionStart)` publishes only the typed
name and default display mode. The actual row-producing `ToolStart` therefore
comes from the lifecycle adapter, while compatibility-created rows can still
share the same tool-call ID. The mixed activity oracle passes unchanged:

```text
cargo run -p runie-tui --bin runie-tui-e2e -- \
  crates/runie-tui/tests/e2e/visual-activity-mixed.yaml
1 passed, 0 failed
```

This is evidence for event provenance, not completion of row ownership. The
next implementation must carry an opaque reducer-owned row identity from the
start intent through update/end messages; matching by `tool_call_id` remains
unsafe for compatibility-seeded replays.

## Completion-output boundary (2026-08-06)

A first implementation experiment carried a reducer token to the live header
and correctly settled that header. The existing Grok oracle then failed: Grok's
specialized cards retain the short header (`List .`, `Run cargo test`) and put
the completed preview (`List . (3 entries)`, `Run cargo test → ✓`) in the
output stream. The experiment was reverted after the event-sequence gate
reported the mismatch. Row identity and completion-output placement are
therefore separate contracts; the next fixture must assert both before the
token is reintroduced.

## Header-row targeting checkpoint (2026-08-06)

The reducer now resolves `ToolUpdate` and `ToolEnd` against the newest matching
semantic header (`Tool`, `ToolRunning`, or `ToolError`) instead of the newest
line with the call ID. This prevents a streamed output line from being
rewritten as a completion header and matches Grok's block model, where the
header owns cardinality/status text and output rows remain body content.
The mixed, truncated, and tool-update YAML oracles were updated to assert the
source-aligned projection; the full visual snapshot suite remains green.
This is a useful ownership boundary, but not yet the final opaque row-token
design required for duplicate compatibility seeds.

## Objective

Give each live `ToolExecutionStart` a reducer-owned row identity distinct from
compatibility/YAML seed rows, then settle that exact row on
`ToolExecutionEnd`. The identity must survive updates, errors, display-mode
changes, and dense-group projection.

## Reducer-owned identity checkpoint (2026-08-06)

`Scrollback` now records the reducer line index created by each live
`ToolStart` in `live_tool_headers`. Updates and completion first resolve that
owned index, validate its semantic header and call ID, and only then fall back
to the legacy newest-header lookup. This prevents a compatibility/YAML seed
with the same provider call ID from receiving a live completion. The
`visual-tool-row-identity.yaml` fixture covers the duplicate-ID sequence and
asserts both rows through the YAML state/transcript contract.
The same fixture now asserts the ordered header-token projection directly as
`tool_header_row_ids: [null, 0]`, making compatibility-versus-live ownership
visible in the no-recompile YAML layer.

The opaque token is now stored on the renderer-independent `Line` and
projected through `ToolBlock`; compatibility rows have no token. Provider call
IDs remain only the external event lookup key at the actor boundary, while
the reducer mutation itself validates the token-bearing semantic header.

## Duplicate live-ID hardening (2026-08-06)

Compatibility/provider replays can present the same external call ID in more
than one live start. `Scrollback` now stores an ordered vector of opaque row
tokens per call ID instead of overwriting one token. Updates and completion
target the newest token and pop only that token, allowing an older live row to
settle independently afterward. A unit scenario covers two live starts,
update/end of the newest row, then end of the older row. The provider ID is
therefore only a lookup key, never the row identity.

## Evidence

`visual-activity-mixed.yaml` is the regression oracle. It contains multiple
tool lifecycle events with stable call IDs and asserts both the original
semantic headers and completed output. Experiments that classified every
`ToolStart` as running or settled every matching call ID corrupted those
assertions, proving that call ID alone is not row ownership.

## Acceptance criteria

- Live and compatibility paths publish one explicit actor intent each.
- `ToolEnd` changes only the live row belonging to its start event.
- A replay containing a compatibility seed cannot leave a stale running row.
- YAML state and full-screen assertions remain unchanged and green.
- No renderer-side mutation or second mutable ownership map is introduced.
- A YAML scenario covers start → update → end, error, and duplicate-seed cases.

## Implementation order

1. Add an opaque row identity to the actor-owned feed projection.
2. Carry it through tool start/update/end messages and typed `ToolBlock` state.
3. Remove compatibility-only row mutation from the live adapter.
4. Add the duplicate-seed YAML oracle and run the complete local gate.
