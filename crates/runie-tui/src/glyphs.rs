//! Unified style glyphs and visual constants.
//!
//! All UI glyphs (chevrons, bullets, etc.) defined here.
//! Spinner frames live in `runie_core::model::state`.
//! No hardcoded glyphs elsewhere in the codebase.

/// User message prompt chevron (matches input box)
pub const CHEVRON: char = '\u{276F}'; // ❯

/// Chevron with trailing space (for prompts)
pub const CHEVRON_WITH_SPACE: &str = "\u{276F} ";

/// Assistant idle/dot indicator
pub const DOT: char = '\u{25E6}';

/// Assistant response bullet (ring operator)
pub const ASSISTANT_BULLET: char = '\u{2218}';

/// Thought duration diamond
pub const THOUGHT_MARKER: char = '◆';

/// Tool call bullet
pub const TOOL_BULLET: char = '●';

/// Diamond shape
pub const DIAMOND: char = '\u{25C6}'; // ◆

/// Bullet shape
pub const BULLET: char = '\u{2022}'; // •

/// Separator line character
pub const SEPARATOR: char = '─';

/// Error indicator
pub const ERROR_MARKER: char = '!';

/// Streaming cursor block
pub const CURSOR_BLOCK: char = '▊';

/// Gauge empty
pub const GAUGE_EMPTY: char = '○';

/// Gauge full
pub const GAUGE_FULL: char = '■';

/// Checkmark (complete)
pub const CHECK_MARKER: char = '✓';

/// Plan step pending arrow
pub const PLAN_PENDING: char = '▸';

/// Plan step active connector
pub const PLAN_ACTIVE: char = '│';

/// Rewind/reset indicator
pub const REWIND: char = '↺';

/// Interrupt/stop indicator
pub const INTERRUPT: char = '✗';

/// Pulse fill character
pub const PULSE_FILL: char = '▐';

/// Scrollbar indicator block (right edge)
pub const SCROLLBAR_INDICATOR: char = '█';
