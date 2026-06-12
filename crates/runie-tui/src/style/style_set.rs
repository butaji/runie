//! Centralized style sets for consistent theming.

use ratatui::style::{Color, Style};

use crate::theme::{ThemeColors, ThemeWrapper};

/// Centralized style sets for consistent theming.
///
/// # Example
///
/// ```ignore
/// use crate::style::StyleSet;
///
/// fn render(area: Rect, buf: &mut Buffer, theme: &ThemeWrapper) {
///     let styles = StyleSet::from_theme(theme);
///     // Use styles.user_msg, styles.assistant_msg, etc.
/// }
/// ```
#[derive(Debug, Clone)]
pub struct StyleSet {
    /// User message style (question/user input)
    pub user_msg: Style,

    /// Assistant message style (AI responses)
    pub assistant_msg: Style,

    /// System message style (meta information)
    pub system_msg: Style,

    /// Error message style
    pub error_msg: Style,

    /// Tool running style (in progress)
    pub tool_running: Style,

    /// Tool complete style (finished)
    pub tool_complete: Style,

    /// Header/title style
    pub header: Style,

    /// Input bar style
    pub input_bar: Style,

    /// Status bar style
    pub status_bar: Style,

    /// Default border style
    pub border: Style,

    /// Muted/secondary text style
    pub muted: Style,

    /// Accent/highlight style
    pub accent: Style,

    /// Primary text style
    pub text_primary: Style,

    /// Secondary text style
    pub text_secondary: Style,

    /// Dimmed text style
    pub text_dim: Style,

    /// Success indicator style
    pub success: Style,

    /// Warning indicator style
    pub warning: Style,

    /// Code/path text style
    pub code: Style,

    /// Thought bubble style
    pub thought: Style,

    /// Plan step style
    pub plan: Style,

    /// Permission request style
    pub permission: Style,
}

impl StyleSet {
    /// Build a StyleSet from a theme.
    ///
    /// Uses ThemeColors extracted from the ThemeWrapper to ensure
    /// consistency across all style definitions.
    pub fn from_theme(theme: &ThemeWrapper) -> Self {
        let c = ThemeColors::from(theme);

        Self {
            user_msg: Style::default().fg(c.accent_user),
            assistant_msg: Style::default().fg(c.accent_assistant),
            system_msg: Style::default().fg(c.accent_system),
            error_msg: Style::default().fg(c.accent_error),
            tool_running: Style::default().fg(c.accent_running),
            tool_complete: Style::default().fg(c.accent_tool),
            header: Style::default().fg(c.text_primary).bold(),
            input_bar: Style::default().fg(c.text_primary),
            status_bar: Style::default().fg(c.text_secondary),
            border: Style::default().fg(c.border_unfocused),
            muted: Style::default().fg(c.text_muted),
            accent: Style::default().fg(c.accent_primary),
            text_primary: Style::default().fg(c.text_primary),
            text_secondary: Style::default().fg(c.text_secondary),
            text_dim: Style::default().fg(c.text_dim),
            success: Style::default().fg(c.success),
            warning: Style::default().fg(c.warning),
            code: Style::default().fg(c.command),
            thought: Style::default().fg(c.accent_thinking),
            plan: Style::default().fg(c.accent_plan),
            permission: Style::default().fg(c.warning),
        }
    }

    /// Build a StyleSet with custom colors (for testing or overrides).
    pub fn with_colors(
        text: Color,
        accent: Color,
        muted: Color,
        success: Color,
        error: Color,
    ) -> Self {
        Self {
            user_msg: Style::default().fg(accent),
            assistant_msg: Style::default().fg(text),
            system_msg: Style::default().fg(muted),
            error_msg: Style::default().fg(error),
            tool_running: Style::default().fg(accent),
            tool_complete: Style::default().fg(success),
            header: Style::default().fg(text).bold(),
            input_bar: Style::default().fg(text),
            status_bar: Style::default().fg(muted),
            border: Style::default().fg(muted),
            muted: Style::default().fg(muted),
            accent: Style::default().fg(accent),
            text_primary: Style::default().fg(text),
            text_secondary: Style::default().fg(muted),
            text_dim: Style::default().fg(muted),
            success: Style::default().fg(success),
            warning: Style::default().fg(Color::Yellow),
            code: Style::default().fg(accent),
            thought: Style::default().fg(Color::Magenta),
            plan: Style::default().fg(Color::Cyan),
            permission: Style::default().fg(Color::Yellow),
        }
    }

    /// Get the default StyleSet (uses default theme).
    pub fn default_styles() -> Self {
        Self::with_colors(
            Color::White,
            Color::Cyan,
            Color::DarkGray,
            Color::Green,
            Color::Red,
        )
    }
}

impl Default for StyleSet {
    fn default() -> Self {
        Self::default_styles()
    }
}

#[cfg(test)]
mod tests {
    use ratatui::style::Color;

    use super::StyleSet;

    #[test]
    fn test_default_styles() {
        let styles = StyleSet::default_styles();
        assert_eq!(styles.user_msg.fg, Some(Color::Cyan));
        assert_eq!(styles.assistant_msg.fg, Some(Color::White));
    }

    #[test]
    fn test_with_colors() {
        let styles = StyleSet::with_colors(
            Color::Blue,
            Color::Yellow,
            Color::Gray,
            Color::Green,
            Color::Red,
        );
        assert_eq!(styles.text_primary.fg, Some(Color::Blue));
        assert_eq!(styles.accent.fg, Some(Color::Yellow));
    }
}
