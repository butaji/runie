//! Semantic theme tokens and default theme TOML.
//! Separated from theme.rs to keep it under the 500-line limit.

use ratatui::style::Color;

// ============================================================================
// Semantic Tokens
// ============================================================================

/// Semantic theme tokens for consistent styling across the application.
/// These map to opaline's token system internally.
#[derive(Clone, Debug)]
pub struct SemanticTokens {
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

impl SemanticTokens {
    /// Extract semantic tokens from the given theme.
    pub fn from_theme(theme: &opaline::Theme) -> Self {
        Self {
            text_primary: theme.color("text.primary").into(),
            text_secondary: theme.color("text.secondary").into(),
            text_accent: theme.color("accent.primary").into(),
            text_link: theme
                .try_color("text.link")
                .map(Into::into)
                .unwrap_or_else(|| theme.color("accent.secondary").into()),
            background_base: theme
                .try_color("bg.base")
                .map(Into::into)
                .unwrap_or_else(|| theme.color("bg.elevated").into()),
            background_input: theme
                .try_color("bg.input")
                .map(Into::into)
                .unwrap_or_else(|| theme.color("bg.elevated").into()),
            background_message_user: theme
                .try_color("bg.user")
                .map(Into::into)
                .unwrap_or_else(|| theme.color("bg.elevated").into()),
            background_message_assistant: theme
                .try_color("bg.agent")
                .map(Into::into)
                .unwrap_or_else(|| theme.color("bg.base").into()),
            border_default: theme.color("border.unfocused").into(),
            status_success: theme.color("success").into(),
            status_error: theme.color("error").into(),
            status_warning: theme.color("warning").into(),
            status_info: theme.color("info").into(),
            code_background: theme.color("bg.code").into(),
            code_foreground: theme.color("text.primary").into(),
            tool_running: theme.color("text.dim").into(),
            tool_success: theme.color("success").into(),
            tool_error: theme.color("error").into(),
        }
    }
}

// ============================================================================
// Default Theme TOML
// ============================================================================

/// Default theme TOML loaded from a resource file.
pub const DEFAULT_THEME_TOML: &str =
    include_str!("../resources/themes/runie.toml");
