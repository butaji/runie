# Output Accumulator

**Status**: done
**Milestone**: R2
**Category**: Safety / Tools

## Description

Replace static truncation with an incremental accumulator. Tracks streaming tool output with bounded memory — rolling tail buffer + temp file fallback.

**Prior art:**
- **pi** (`output-accumulator.ts`): rolling tail buffer (2x max bytes), temp file when limits exceeded

## Architecture

```rust
pub struct OutputAccumulator {
    max_lines: usize,
    max_bytes: usize,
    buffer: Vec<u8>,           // rolling tail buffer (2x max_bytes)
    temp_file: Option<tempfile::NamedTempFile>,
    total_lines: usize,
    total_bytes: usize,
    strategy: TruncateStrategy,
}

#[derive(Clone, Copy)]
pub enum TruncateStrategy {
    Head,  // Keep beginning (for read, grep, find, ls)
    Tail,  // Keep end (for bash)
}

pub struct AccumulatedOutput {
    pub content: String,
    pub was_truncated: bool,
    pub total_lines: usize,
    pub total_bytes: usize,
}

impl OutputAccumulator {
    pub fn new(policy: &TruncationPolicy, strategy: TruncateStrategy) -> Self;
    pub fn append(&mut self, chunk: &[u8]);
    pub fn snapshot(&self) -> AccumulatedOutput;
}
```

## Acceptance Criteria

- [x] Accepts chunks incrementally during tool execution
- [x] Rolling tail buffer keeps last 2x max_bytes in memory
- [x] Switches to temp file when output exceeds 2x limit
- [x] `snapshot()` returns truncated view + metadata
- [x] Never splits lines mid-content
- [x] Per-tool strategy: head for reads, tail for bash
- [x] Replaces static truncation in all tools

## Files

| File | Description |
|------|-------------|
| `crates/runie-agent/src/accumulator.rs` | New: `OutputAccumulator` |
| `crates/runie-agent/src/tools.rs` | Use accumulator in tool execution |

## Tests

### Layer 1 — State/Logic
- [x] `accumulator_tracks_total` — append increases counters
- [x] `snapshot_tail_returns_end` — bash strategy
- [x] `snapshot_head_returns_start` — read strategy
- [x] `never_splits_lines` — complete lines only
- [x] `small_output_no_truncation` — under limit returns full
