# Semantic Theme Tokens

**Status**: done
**Milestone**: R3
**Category**: TUI / Rendering
**Priority**: P2

## Description

Runie themes currently use ad-hoc color lookups. Research from Gemini CLI
(`Theme` with `status.error`, `ui.active`, `background.input`), Kimi Code
(`ColorPalette` with CI guard), and Aider (semantic color map) shows that
semantic tokens make themes consistent and accessible.

## Acceptance Criteria

- [x] `crates/runie-tui/src/theme.rs` defines a `SemanticTokens` struct with semantic
  tokens:
  - `text_primary`, `text_secondary`, `text_accent`, `text_link`
  - `background_base`, `background_input`, `background_message_user`, `background_message_assistant`
  - `border_default`, `status_success`, `status_error`, `status_warning`, `status_info`
  - `code_background`, `code_foreground`, `tool_running`, `tool_success`, `tool_error`
- [x] Built-in themes use semantic tokens via `SemanticTokens::from_theme()`.
- [ ] `opaline` theme loading maps legacy palette keys to semantic tokens with
  a migration note.
- [ ] A lint/test rejects direct `Color::Red`, `Color::Cyan`, etc. outside the
  theme module (allow listed exceptions for ANSI escape parsing).
- [ ] Contrast ratio check for light themes (WCAG AA for normal text).
- [x] `cargo build --workspace` succeeds.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `builtin_themes_define_all_tokens` — every token has a value.
- [ ] `light_theme_contrast_passes_wcag_aa` — foreground/background contrast
  ≥ 4.5 for text tokens.
- [ ] `no_hardcoded_colors_outside_theme` — grep-based test.

### Layer 2 — Event Handling
N/A.

### Layer 3 — Rendering
- [x] `user_message_uses_background_message_user` — TestBackend cell has the
  expected background color.
- [x] `error_status_uses_status_error` — error line uses the error token.

## Notes

**Files touched:**
- `crates/runie-tui/src/theme.rs`
- `crates/runie-tui/src/tests/theme.rs`
- `crates/runie-core/build.rs` (add lint)

**Out of scope:**
- User-defined theme editor UI.
- Animated gradients.
