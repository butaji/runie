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

/// Home menu content height (3 items + 2 dividers = 5 rows)
pub const MENU_HEIGHT: u16 = 5;

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
