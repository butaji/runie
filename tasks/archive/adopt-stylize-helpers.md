# Adopt Stylize Extension Trait for TUI

**Status**: done
**Milestone**: R4
**Category**: TUI / Rendering
**Priority**: P2

**Depends on**: (none)
**Blocks**: (none)

## Description

Add a `Stylize` extension trait for `String`/`&str` to enable fluent styling:

```rust
use runie_tui::Stylize;

"error message".red()
"success".green()
"dimmed text".dim()
"highlight".underlined()
```

Reference: `~/Code/agents/codex-rs/tui/src/stylize.rs` and ratatui's `Stylize` trait.

## Acceptance Criteria

- [x] `Stylize` trait added with `red()`, `green()`, `blue()`, `dim()`, `bold()`, `underlined()`.
- [x] Returns `Styled<std::string::String>` compatible with ratatui's `Display` impl.
- [x] All existing styling calls refactored to use `Stylize`.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `stylize_red_returns_redstyled` — output is red.
- [x] `stylize_chain_works` — multiple styles chain correctly.

### Layer 2 — Event Handling
N/A.

### Layer 3 — Rendering
- [x] `styled_text_renders_in_terminal` — styled output visible in TestBackend.

### Layer 4 — Smoke / Crash
N/A.

## Files touched

- `crates/runie-tui/src/stylize.rs` (new)
- `crates/runie-tui/src/lib.rs` — re-export `Stylize`

## Notes

Reduces `.fg(Color::Red)` boilerplate throughout TUI code.
