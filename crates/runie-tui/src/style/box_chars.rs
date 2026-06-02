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
