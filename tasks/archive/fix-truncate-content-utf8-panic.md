# Fix truncate_content UTF-8 panic

**Status**: todo
**Milestone**: R4
**Category**: Tools
**Priority**: P0

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-engine/src/tool/search/modes.rs:145` defines `truncate_content` using `&content[..max_len]` — a raw byte slice that panics on multi-byte UTF-8 boundaries (any emoji/CJK in a matched search line). A char-boundary-safe version already exists at `crates/runie-core/src/tool/format.rs:159` (`truncate_to_bytes` with `while !s.is_char_boundary(end)` loop). This is a latent correctness bug masquerading as duplication.

## Acceptance Criteria

- [ ] `truncate_content` in `search/modes.rs` replaced with a call to `runie_core::tool::format::truncate_output` (or `truncate_to_bytes`).
- [ ] Local `truncate_content` fn deleted.
- [ ] A search result line containing a multi-byte character (e.g. `line with emoji 😀`) truncates without panic.
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `truncate_content_multibyte_no_panic` — truncate a string with an emoji at the cut boundary; assert no panic and valid UTF-8 result.
- [ ] `truncate_content_short_string_unchanged` — string shorter than max is returned as-is.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_search_with_unicode_results` — a search tool call returning CJK/emoji lines does not crash the turn.

## Files touched

- `crates/runie-engine/src/tool/search/modes.rs`

## Notes

This is the only finding with a real correctness risk. Highest priority despite small size.
