//! Centralized style module for all visual constants, style builders, format templates, and layout values.
//!
//! # Architecture
//!
//! - [`layout`] constants for spacing and dimensions
//! - [`format`] templates for consistent message formatting
//! - [`box_chars`] drawing characters for borders and panels
//! - [`selection`] characters for UI state indicators
//! - [`StyleSet`] builder for themed styles
//! - [`helpers`] for area calculations

use ratatui::{
    layout::Rect,
    style::{Color, Style},
};

use crate::glyphs;
use crate::theme::{ThemeColors, ThemeWrapper};

// ─── Re-exports from glyphs ──────────────────────────────────────────────────

/// User message prompt chevron (matches input box) - re-exported from glyphs
pub use glyphs::CHEVRON;

/// Chevron with trailing space (for prompts) - re-exported from glyphs
pub use glyphs::CHEVRON_WITH_SPACE;

/// Assistant idle/dot indicator - re-exported from glyphs
pub use glyphs::DOT;

/// Assistant response bullet (ring operator) - re-exported from glyphs
pub use glyphs::ASSISTANT_BULLET;

/// Thought duration diamond - re-exported from glyphs
pub use glyphs::THOUGHT_MARKER;

/// Tool call bullet - re-exported from glyphs
pub use glyphs::TOOL_BULLET;

/// Separator line character - re-exported from glyphs
pub use glyphs::SEPARATOR;

/// Error indicator - re-exported from glyphs
pub use glyphs::ERROR_MARKER;

/// Plan step pending arrow - re-exported from glyphs
pub use glyphs::PLAN_PENDING;

/// Plan step active connector - re-exported from glyphs
pub use glyphs::PLAN_ACTIVE;

/// Rewind/reset indicator - re-exported from glyphs
pub use glyphs::REWIND;

/// Interrupt/stop indicator - re-exported from glyphs
pub use glyphs::INTERRUPT;

/// Braille spinner frames - re-exported from glyphs
pub use glyphs::SPINNER_FRAMES;

/// Reverse braille spinner - re-exported from glyphs
pub use glyphs::SPINNER_FRAMES_REVERSE;

/// Get current spinner frame from animation tick - re-exported from glyphs
pub use glyphs::spinner_frame;

/// Get reverse spinner frame from animation tick - re-exported from glyphs
pub use glyphs::spinner_frame_reverse;

/// Diamond shape - re-exported from glyphs
pub use glyphs::DIAMOND;

/// Bullet shape - re-exported from glyphs
pub use glyphs::BULLET;

/// Gauge empty - re-exported from glyphs
pub use glyphs::GAUGE_EMPTY;

/// Gauge full - re-exported from glyphs
pub use glyphs::GAUGE_FULL;

/// Checkmark (complete) - re-exported from glyphs
pub use glyphs::CHECK_MARKER;

/// Streaming cursor block - re-exported from glyphs
pub use glyphs::CURSOR_BLOCK;

/// Pulse fill character - re-exported from glyphs
pub use glyphs::PULSE_FILL;

// ─── Section 1: Layout Constants ─────────────────────────────────────────────

pub mod layout {
    //! Layout constants for spacing, dimensions, and sizing.

    /// Horizontal padding (area.x + 2)
    pub const PADDING_X: u16 = 2;

    /// Vertical padding (area.y + 1)
    pub const PADDING_Y: u16 = 1;

    /// Total width padding (2 * PADDING_X)
    pub const PADDING_WIDTH: u16 = 4;

    /// Total height padding (2 * PADDING_Y)
    pub const PADDING_HEIGHT: u16 = 2;

    /// Indent from margin_x (margin_x + 3 = 5 from edge)
    pub const MESSAGE_INDENT: u16 = 3;

    /// Home menu content width
    pub const MENU_WIDTH: u16 = 40;

    /// Home menu content height
    pub const MENU_HEIGHT: u16 = 14;

    /// Sidebar width
    pub const SIDEBAR_WIDTH: u16 = 28;

    /// Activity panel width
    pub const ACTIVITY_PANEL_WIDTH: u16 = 30;

    /// Subagent panel width
    pub const SUBAGENT_PANEL_WIDTH: u16 = 50;

    /// Search overlay height
    pub const SEARCH_OVERLAY_HEIGHT: u16 = 10;

    /// Max tree indent depth
    pub const MAX_TREE_DEPTH: usize = 5;

    /// Interrupt fade duration in milliseconds
    pub const FADE_DURATION_MS: f64 = 500.0;

    /// Agent list item height
    pub const AGENT_ITEM_HEIGHT: u16 = 4;

    /// Y offset for list content
    pub const LIST_START_Y: u16 = 3;

    /// Tab spacing
    pub const TAB_SPACING: u16 = 3;

    /// Border width
    pub const BORDER_WIDTH: u16 = 1;

    /// Default inner margin
    pub const INNER_MARGIN: u16 = 1;

    /// Status bar height
    pub const STATUS_BAR_HEIGHT: u16 = 1;

    /// Top bar height
    pub const TOP_BAR_HEIGHT: u16 = 1;

    /// Input bar minimum height
    pub const INPUT_BAR_MIN_HEIGHT: u16 = 3;

    /// Tool call item height
    pub const TOOL_ITEM_HEIGHT: u16 = 3;

    /// Panel separator width
    pub const SEPARATOR_WIDTH: u16 = 1;

    /// Scrollbar width
    pub const SCROLLBAR_WIDTH: u16 = 1;

    /// Modal corner radius (for border styling)
    pub const MODAL_CORNER_RADIUS: u16 = 0;
}

// ─── Section 2: Format Templates ─────────────────────────────────────────────

pub mod format {
    //! Format string templates for consistent message rendering.

    /// Collapsed thought bubble format: marker, content, duration, end marker
    pub const THOUGHT_COLLAPSED_FMT: &str = "{} {} Thought for {:.1}s {}";

    /// Expanded thought bubble format (same as collapsed, differs in content)
    pub const THOUGHT_EXPANDED_FMT: &str = "{} {} Thought for {:.1}s {}";

    /// System message prefix: 3 spaces + marker + space
    pub const SYSTEM_PREFIX: &str = "   {} ";

    /// Error prefix: 3 spaces + exclamation
    pub const ERROR_PREFIX: &str = "   ! ";

    /// User message chevron format: indent + chevron + space
    pub const USER_CHEVRON: &str = "{} {} ";

    /// Assistant message bullet format: 3 spaces + bullet + space
    pub const ASSISTANT_BULLET_FMT: &str = "   {} ";

    /// Tool running format: indent, name, status, duration
    pub const TOOL_RUNNING_FMT: &str = "{} Run {} {} {:.1}s";

    /// Tool complete format: indent, status, name, arrow, duration
    pub const TOOL_COMPLETE_FMT: &str = "{} {} {} → {} {:.1}s";

    /// Version badge format
    pub const VERSION_BADGE_FMT: &str = "{} Beta";

    /// Status separator between status items
    pub const STATUS_SEPARATOR: &str = "  │  ";

    /// Session tree node format
    pub const TREE_NODE_FMT: &str = "{} {} ";

    /// Permission request format
    pub const PERMISSION_FMT: &str = "   {} {}";

    /// Plan step pending format
    pub const PLAN_STEP_PENDING_FMT: &str = "   ▸ {}";

    /// Plan step complete format
    pub const PLAN_STEP_COMPLETE_FMT: &str = "   ✓ {}";
}

// ─── Section 3: Box Drawing Characters ───────────────────────────────────────

pub mod box_chars {
    //! Box drawing characters for borders, panels, and decorative frames.

    /// Top-left corner (rounded)
    pub const TL: char = '╭';

    /// Top-right corner (rounded)
    pub const TR: char = '╮';

    /// Bottom-left corner (rounded)
    pub const BL: char = '╰';

    /// Bottom-right corner (rounded)
    pub const BR: char = '╯';

    /// Horizontal line
    pub const H: char = '─';

    /// Vertical line
    pub const V: char = '│';

    /// Top-left corner (square, for modals using ┌┐└┘)
    pub const TL_ALT: char = '┌';

    /// Top-right corner (square)
    pub const TR_ALT: char = '┐';

    /// Bottom-left corner (square)
    pub const BL_ALT: char = '└';

    /// Bottom-right corner (square)
    pub const BR_ALT: char = '┘';

    /// Heavy horizontal line
    pub const H_HEAVY: char = '━';

    /// Heavy vertical line
    pub const V_HEAVY: char = '┃';

    /// Double horizontal line
    pub const H_DOUBLE: char = '═';

    /// Double vertical line
    pub const V_DOUBLE: char = '║';

    /// Mixed: left heavy, right light (vertical)
    pub const V_LEFT_HEAVY: char = '┨';

    /// Mixed: top-left heavy corners
    pub const T_LEFT_HEAVY: char = '┏';

    /// Mixed: top-right heavy
    pub const T_RIGHT_HEAVY: char = '┓';

    /// Mixed: bottom-left heavy
    pub const B_LEFT_HEAVY: char = '┗';

    /// Mixed: bottom-right heavy
    pub const B_RIGHT_HEAVY: char = '┛';
}

// ─── Section 4: Selection/Status Characters ──────────────────────────────────

pub mod selection {
    //! Selection, expansion, and status indicator characters.

    /// Selected item indicator (filled triangle)
    pub const SELECTED: char = '▸';

    /// Unselected item (space)
    pub const UNSELECTED: char = ' ';

    /// Expanded/expanded state (down triangle)
    pub const EXPANDED: char = '▼';

    /// Collapsed state (right triangle)
    pub const COLLAPSED: char = '▶';

    /// Active/running status (filled circle)
    pub const STATUS_ACTIVE: char = '●';

    /// Idle/inactive status (empty circle)
    pub const STATUS_IDLE: char = '○';

    /// Radio button selected
    pub const RADIO_SELECTED: &str = "◉";

    /// Radio button unselected
    pub const RADIO_UNSELECTED: &str = "○";

    /// Close hint brackets
    pub const CLOSE_HINT: &str = " [✗] ";

    /// Progress bar filled character
    pub const PROGRESS_FILLED: char = '█';

    /// Progress bar empty character
    pub const PROGRESS_EMPTY: char = '░';

    /// Progress bar partial fill
    pub const PROGRESS_HALF: char = '▒';

    /// Checkbox checked
    pub const CHECKBOX_CHECKED: &str = "☑";

    /// Checkbox unchecked
    pub const CHECKBOX_UNCHECKED: &str = "☐";

    /// Checkbox partial
    pub const CHECKBOX_PARTIAL: &str = "☒";

    /// Branch indicator (tree)
    pub const BRANCH: char = '├';

    /// Last branch indicator (tree)
    pub const BRANCH_LAST: char = '└';

    /// Vertical continuation (tree)
    pub const VERTICAL: char = '│';

    /// Horizontal line (tree)
    pub const HORIZONTAL: char = '─';

    /// Git branch symbol (Powerline style)
    pub const GIT_BRANCH_SYMBOL: char = '\u{E0A0}';
}

// ─── Section 5: Style Builder Methods ────────────────────────────────────────

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

// ─── Section 6: Helper Functions ─────────────────────────────────────────────

pub mod helpers {
    //! Helper functions for layout calculations.

    use ratatui::layout::Rect;

    use super::layout;

    /// Apply standard padding to an area.
    ///
    /// Returns a new Rect with padding subtracted from each side.
    ///
    /// # Example
    ///
    /// ```
    /// use ratatui::layout::Rect;
    /// use crate::style::helpers::padded_area;
    ///
    /// let area = Rect::new(0, 0, 20, 10);
    /// let padded = padded_area(area);
    /// // padded.x = 2, padded.y = 1, padded.width = 16, padded.height = 8
    /// ```
    pub fn padded_area(area: Rect) -> Rect {
        Rect {
            x: area.x + layout::PADDING_X,
            y: area.y + layout::PADDING_Y,
            width: area.width.saturating_sub(layout::PADDING_WIDTH),
            height: area.height.saturating_sub(layout::PADDING_HEIGHT),
        }
    }

    /// Calculate content width within a padded area.
    ///
    /// Returns the width available for content after subtracting horizontal padding.
    pub fn content_width(area: Rect) -> u16 {
        area.width.saturating_sub(layout::PADDING_X)
    }

    /// Calculate message text width accounting for indent and markers.
    ///
    /// The indent parameter accounts for the left margin used by message markers
    /// (e.g., chevron for user, bullet for assistant).
    pub fn message_text_width(area: Rect, indent: u16) -> u16 {
        area.width.saturating_sub(indent + 4)
    }

    /// Calculate the inner width of a panel after subtracting border widths.
    pub fn inner_width(area: Rect) -> u16 {
        area.width.saturating_sub(2)
    }

    /// Calculate the inner height of a panel after subtracting border heights.
    pub fn inner_height(area: Rect) -> u16 {
        area.height.saturating_sub(2)
    }

    /// Center a rectangle within another rectangle horizontally.
    ///
    /// Returns the x-offset needed to center `inner` within `outer`.
    pub fn center_x(outer: Rect, inner_width: u16) -> u16 {
        outer.x + (outer.width.saturating_sub(inner_width)) / 2
    }

    /// Center a rectangle within another rectangle vertically.
    ///
    /// Returns the y-offset needed to center `inner` within `outer`.
    pub fn center_y(outer: Rect, inner_height: u16) -> u16 {
        outer.y + (outer.height.saturating_sub(inner_height)) / 2
    }

    /// Check if a point is within a rectangle.
    pub fn contains(area: Rect, x: u16, y: u16) -> bool {
        x >= area.x && x < area.x + area.width && y >= area.y && y < area.y + area.height
    }

    /// Split an area into left and right panels with a separator.
    pub fn split_with_separator(area: Rect, left_width: u16) -> (Rect, Rect) {
        let separator_width = 1u16;
        let left = Rect::new(area.x, area.y, left_width, area.height);
        let right = Rect::new(
            area.x + left_width + separator_width,
            area.y,
            area.width.saturating_sub(left_width + separator_width),
            area.height,
        );
        (left, right)
    }
}

#[cfg(test)]
mod tests {
    use ratatui::layout::Rect;

    use super::helpers;

    #[test]
    fn test_padded_area() {
        let area = Rect::new(0, 0, 20, 10);
        let padded = helpers::padded_area(area);
        assert_eq!(padded.x, 2);
        assert_eq!(padded.y, 1);
        assert_eq!(padded.width, 16);
        assert_eq!(padded.height, 8);
    }

    #[test]
    fn test_content_width() {
        let area = Rect::new(0, 0, 20, 10);
        assert_eq!(helpers::content_width(area), 18);
    }

    #[test]
    fn test_message_text_width() {
        let area = Rect::new(0, 0, 40, 10);
        assert_eq!(helpers::message_text_width(area, 3), 33);
    }

    #[test]
    fn test_center_x() {
        let outer = Rect::new(0, 0, 100, 20);
        assert_eq!(helpers::center_x(outer, 20), 40);
    }

    #[test]
    fn test_center_y() {
        let outer = Rect::new(0, 0, 100, 20);
        assert_eq!(helpers::center_y(outer, 10), 5);
    }

    #[test]
    fn test_contains() {
        let area = Rect::new(5, 5, 10, 10);
        assert!(helpers::contains(area, 5, 5));
        assert!(helpers::contains(area, 10, 10));
        assert!(!helpers::contains(area, 4, 5));
        assert!(!helpers::contains(area, 15, 5));
    }

    #[test]
    fn test_split_with_separator() {
        let area = Rect::new(0, 0, 50, 20);
        let (left, right) = helpers::split_with_separator(area, 20);
        assert_eq!(left.width, 20);
        assert_eq!(right.x, 21);
        assert_eq!(right.width, 29);
    }
}
