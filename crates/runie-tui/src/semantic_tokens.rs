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
            text_link: theme.try_color("text.link").map(Into::into).unwrap_or_else(|| theme.color("accent.secondary").into()),
            background_base: theme.try_color("bg.base").map(Into::into).unwrap_or_else(|| theme.color("bg.elevated").into()),
            background_input: theme.try_color("bg.input").map(Into::into).unwrap_or_else(|| theme.color("bg.elevated").into()),
            background_message_user: theme.try_color("bg.user").map(Into::into).unwrap_or_else(|| theme.color("bg.elevated").into()),
            background_message_assistant: theme.try_color("bg.agent").map(Into::into).unwrap_or_else(|| theme.color("bg.base").into()),
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

/// Default theme TOML string.
pub const DEFAULT_THEME_TOML: &str = r##"
[meta]
name = "Runie"
author = "runie"
variant = "dark"
version = "1.0"
description = "Dark base with vibrant orange accents"

[palette]
orange_500 = "#EE6902"
orange_400 = "#F5853F"
orange_600 = "#D45A00"
orange_300 = "#F9A85F"
amber_400 = "#F5A623"
amber_300 = "#F9C846"
coral_400 = "#E85577"
green_400 = "#4ADE80"
red_400 = "#EF4444"
blue_400 = "#60A5FA"
lime_400 = "#A3E635"

bg_code = "#1E1920"
bg_highlight = "#2A202E"
bg_elevated = "#201820"
bg_active = "#302830"
bg_selection = "#3A2E38"

text_primary = "#EDE8E3"
text_secondary = "#C4BEB7"
text_muted = "#8A8580"
text_dim = "#5C5854"

[tokens]
"text.primary" = "text_primary"
"text.secondary" = "text_secondary"
"text.muted" = "text_muted"
"text.dim" = "text_dim"

"bg.code" = "bg_code"
"bg.highlight" = "bg_highlight"
"bg.elevated" = "bg_elevated"
"bg.active" = "bg_active"
"bg.selection" = "bg_selection"
"bg.user" = "bg_elevated"

"accent.primary" = "orange_500"
"accent.secondary" = "amber_400"
"accent.tertiary" = "coral_400"
"accent.deep" = "orange_600"

success = "green_400"
error = "red_400"
warning = "amber_400"
info = "blue_400"

"border.focused" = "orange_500"
"border.unfocused" = "text_dim"

"code.keyword" = "amber_400"
"code.function" = "orange_500"
"code.string" = "lime_400"
"code.number" = "amber_300"
"code.comment" = "text_dim"
"code.type" = "blue_400"
"code.line_number" = "text_dim"

[styles]
keyword = { fg = "accent.primary", bold = true }
line_number = { fg = "code.line_number" }
cursor_line = { bg = "bg.highlight" }
selected = { fg = "accent.secondary", bg = "bg.highlight" }
active_selected = { fg = "accent.primary", bg = "bg.active", bold = true }
focused_border = { fg = "border.focused" }
unfocused_border = { fg = "border.unfocused" }
success_style = { fg = "success" }
error_style = { fg = "error" }
warning_style = { fg = "warning" }
info_style = { fg = "info" }
dimmed = { fg = "text.dim" }
muted = { fg = "text.muted" }
inline_code = { fg = "success", bg = "bg.code" }
"##;
