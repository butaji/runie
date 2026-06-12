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

pub mod box_chars;
pub mod format;
pub mod helpers;
pub mod layout;
pub mod selection;
pub mod style_set;

pub use style_set::StyleSet;

// ─── Re-exports from glyphs ──────────────────────────────────────────────────

/// User message prompt chevron (matches input box)
pub use crate::glyphs::CHEVRON;

/// Chevron with trailing space (for prompts)
pub use crate::glyphs::CHEVRON_WITH_SPACE;

/// Assistant idle/dot indicator
pub use crate::glyphs::DOT;

/// Assistant response bullet (ring operator)
pub use crate::glyphs::ASSISTANT_BULLET;

/// Thought duration diamond
pub use crate::glyphs::THOUGHT_MARKER;

/// Tool call bullet
pub use crate::glyphs::TOOL_BULLET;

/// Separator line character
pub use crate::glyphs::SEPARATOR;

/// Error indicator
pub use crate::glyphs::ERROR_MARKER;

/// Plan step pending arrow
pub use crate::glyphs::PLAN_PENDING;

/// Plan step active connector
pub use crate::glyphs::PLAN_ACTIVE;

/// Rewind/reset indicator
pub use crate::glyphs::REWIND;

/// Interrupt/stop indicator
pub use crate::glyphs::INTERRUPT;

/// Braille spinner frames
pub use crate::glyphs::SPINNER_FRAMES;

/// Reverse braille spinner
pub use crate::glyphs::SPINNER_FRAMES_REVERSE;

/// Get current spinner frame from animation tick
pub use crate::glyphs::spinner_frame;

/// Get reverse spinner frame from animation tick
pub use crate::glyphs::spinner_frame_reverse;

/// Diamond shape
pub use crate::glyphs::DIAMOND;

/// Bullet shape
pub use crate::glyphs::BULLET;

/// Gauge empty
pub use crate::glyphs::GAUGE_EMPTY;

/// Gauge full
pub use crate::glyphs::GAUGE_FULL;

/// Checkmark (complete)
pub use crate::glyphs::CHECK_MARKER;

/// Streaming cursor block
pub use crate::glyphs::CURSOR_BLOCK;

/// Pulse fill character
pub use crate::glyphs::PULSE_FILL;
