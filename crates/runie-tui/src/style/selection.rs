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
