//! Scroll event variants.

use std::fmt;
use strum::IntoStaticStr;

/// Events that scroll the message feed.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, IntoStaticStr)]
#[strum(serialize_all = "PascalCase")]
pub enum ScrollEvent {
    Up,
    Down,
    PageUp,
    PageDown,
    GoToTop,
    GoToBottom,
}

impl fmt::Display for ScrollEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScrollEvent::Up => write!(f, "ScrollUp"),
            ScrollEvent::Down => write!(f, "ScrollDown"),
            ScrollEvent::PageUp => write!(f, "ScrollPageUp"),
            ScrollEvent::PageDown => write!(f, "ScrollPageDown"),
            ScrollEvent::GoToTop => write!(f, "ScrollGoToTop"),
            ScrollEvent::GoToBottom => write!(f, "ScrollGoToBottom"),
        }
    }
}

impl ScrollEvent {
    /// Serialize to a stable string for keybinding lookup.
    pub fn name(&self) -> &'static str {
        match self {
            ScrollEvent::Up => "ScrollUp",
            ScrollEvent::Down => "ScrollDown",
            ScrollEvent::PageUp => "ScrollPageUp",
            ScrollEvent::PageDown => "ScrollPageDown",
            ScrollEvent::GoToTop => "ScrollGoToTop",
            ScrollEvent::GoToBottom => "ScrollGoToBottom",
        }
    }
}
