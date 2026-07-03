# Define JSONL compaction threshold and policy

## Status

`done`

## Description

Before adopting snapshot+journal JSONL, define when to compact (file size, event count, turn count) and the policy for preserving/exporting the original journal.

### Defined Thresholds

| Constant | Value | Rationale |
|---------|-------|----------|
| `COMPACT_FILE_SIZE_BYTES` | 10 MB | Typical JSONL for 500 events ≈ 2-5 MB; 10 MB gives headroom |
| `COMPACT_EVENT_COUNT` | 500 | ~100-200 turns of average conversation; triggers before OOM |
| `COMPACT_TURN_COUNT` | 50 | Prevents unbounded context overhead from long sessions |
| `COMPACT_TARGET_EVENTS` | 100 | Keeps recent context window after summarisation |

### Compaction Policy

`CompactionPolicy::ArchiveToSidecar` (default):
- Original journal is preserved as `<session_id>.journal/turn_N.jsonl`
- Compacted file contains a synthetic `SessionCompacted` event + recent window
- Original events are never deleted (safe but uses more disk)

`CompactionPolicy::DiscardOriginal`:
- Original journal is dropped after summarisation
- Compacted file contains summary + recent window
- Saves disk space at the cost of losing original event history

## Implementation

Added to `crates/runie-core/src/session/store.rs`:
- `COMPACT_FILE_SIZE_BYTES`, `COMPACT_EVENT_COUNT`, `COMPACT_TURN_COUNT`, `COMPACT_TARGET_EVENTS` constants
- `CompactionPolicy` enum with `ArchiveToSidecar` and `DiscardOriginal` variants
- Unit tests verifying threshold ordering and default policy

## Acceptance criteria

- [x] Define compaction thresholds. — Done; `COMPACT_FILE_SIZE_BYTES` (10 MB), `COMPACT_EVENT_COUNT` (500), `COMPACT_TURN_COUNT` (50), `COMPACT_TARGET_EVENTS` (100) in `session/store.rs`.
- [x] Define preservation policy. — Done; `CompactionPolicy` enum with `ArchiveToSidecar` (default) and `DiscardOriginal` variants.
- [x] Unit tests. — Done; `compaction_thresholds_are_reasonable` and `compaction_policy_default_is_archive` pass.

## Tests

### Unit tests
- N/A.

### E2E tests
- N/A.

### Live tmux tests
- N/A.
