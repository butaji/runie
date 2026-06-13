# Semantic Theme Tokens

**Status**: todo
**Milestone**: R3
**Category**: TUI Rendering
**Priority**: P2

## Description

Runie themes currently use ad-hoc color lookups. Research from Gemini CLI
(`Theme` with `status.error`, `ui.active`, `background.input`), Kimi Code
(`ColorPalette` with CI guard), and Aider (semantic color map) shows that
semantic tokens make themes consistent and accessible.

## Acceptance Criteria

- [ ] `crates/runie-tui/src/theme.rs` defines a `Theme` struct with semantic
  tokens:
  ```rust
  pub struct Theme {
      pub text_primary: Color,
      pub text_secondary: Color,
      pub text_accent: Color,
      pub text_link: Color,
      pub background_base: Color,
      pub background_input: Color,
      pub background_message_user: Color,
      pub background_message_assistant: Color,
      pub border_default: Color,
      pub status_success: Color,
      pub status_error: Color,
      pub status_warning: Color,
      pub status_info: Color,
      pub code_background: Color,
      pub code_foreground: Color,
      pub tool_running: Color,
      pub tool_success: Color,
      pub tool_error: Color,
  }
  ```
- [ ] All built-in themes (`BUILTIN_THEMES`) converted to semantic tokens.
- [ ] `opaline` theme loading maps legacy palette keys to semantic tokens with
  a migration note.
- [ ] A lint/test rejects direct `Color::Red`, `Color::Cyan`, etc. outside the
  theme module (allow listed exceptions for ANSI escape parsing).
- [ ] Contrast ratio check for light themes (WCAG AA for normal text).
- [ ] `cargo build --workspace` succeeds.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `builtin_themes_define_all_tokens` — every token has a value.
- [ ] `light_theme_contrast_passes_wcag_aa` — foreground/background contrast
  ≥ 4.5 for text tokens.
- [ ] `no_hardcoded_colors_outside_theme` — grep-based test.

### Layer 2 — Event Handling
N/A.

### Layer 3 — Rendering
- [ ] `user_message_uses_background_message_user` — TestBackend cell has the
  expected background color.
- [ ] `error_status_uses_status_error` — error line uses the error token.

## Notes

**Files touched:**
- `crates/runie-tui/src/theme.rs`
- `crates/runie-tui/src/tests/theme.rs`
- `crates/runie-core/build.rs` (add lint)

**Out of scope:**
- User-defined theme editor UI.
- Animated gradients.
