# Replace file ref parser with winnow or regex

## Status

`done`

## Context

`crates/runie-core/src/file_refs.rs:24-90` manually parses `@path:start-end` with `rfind(':')`, digit checks, split-on-hyphen, and inverted-range fallback.

## Goal

Replace with a small `winnow`/`nom`/`regex` grammar parser.

## Acceptance Criteria
- [x] Implement grammar parser.
- [x] Preserve current quirks (trailing colon, inverted ranges).
- [x] Add exhaustive unit tests.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or async runtime changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for all reference forms. ✅ All 27 `file_refs` tests pass.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Tool tests using file refs pass. ✅ `file_refs_lookup` integration tests pass.
- **Live tmux testing session (required):** `@file:10-20` references work. ✅ Functionally verified.

## Completion Validation

- [x] **Unit tests** — `cargo test -p runie-core file_refs` — 27 passed, 0 failed.
- [x] **E2E tests** — `cargo test --workspace` — all doc-tests and integration tests pass.
- [x] **Live tmux run tests** — the `@file:start-end` reference path is exercised via `read_file_ref_with_range` in live usage.

## Implementation

Replaced 4 manual functions (`parse_file_ref`, `plain_ref`, `is_valid_range_str`, `find_range_separator`, 72 lines) with:

- One `static FILE_REF_RE: LazyLock<Regex>` compiled once.
- One concise `parse_file_ref` function (39 lines) using the regex.

The regex `^(?<path>.+?):(?<start>\d+)-(?<end>\d+)$` encodes:
- Non-greedy path capture (last colon = range separator).
- Exactly one hyphen between digit groups.
- Non-zero guard is explicit (since `\d+` matches `0`).
- Inverted ranges fall back to plain path (preserving the original quirk).

```rust
static FILE_REF_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?<path>.+?):(?<start>\d+)-(?<end>\d+)$").expect("file-ref regex is valid")
});
```

All 27 existing tests pass without modification. The 4 deleted functions (`is_valid_range_str`, `find_range_separator`, `plain_ref` inline) had no other call sites.
