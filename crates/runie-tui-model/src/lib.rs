//! Renderer-independent TUI projections.
//!
//! This crate deliberately has no terminal, widget, or core-runtime
//! dependency. Actors reduce events into these immutable values; renderers
//! only consume them.

mod feed;
mod status;

pub use feed::{Line, LineKind, ScrollbackMsg};
pub use status::{Status, StatusMsg};

/// Pure viewport projection for a feed that may follow its newest content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollState {
    pub scroll_top: u16,
    pub content_height: u16,
    pub viewport_height: u16,
    pub following_end: bool,
}

impl ScrollState {
    pub const fn new(follow_end: bool) -> Self {
        Self {
            scroll_top: 0,
            content_height: 0,
            viewport_height: 0,
            following_end: follow_end,
        }
    }

    pub const fn max_scroll_top(self) -> u16 {
        self.content_height.saturating_sub(self.viewport_height)
    }

    pub const fn update_layout(mut self, content_height: u16, viewport_height: u16) -> Self {
        self.content_height = content_height;
        self.viewport_height = viewport_height;
        let max = self.max_scroll_top();
        self.scroll_top = if self.following_end || self.scroll_top > max {
            max
        } else {
            self.scroll_top
        };
        if self.following_end && self.content_height <= self.viewport_height {
            self.scroll_top = 0;
        }
        self
    }

    pub const fn scroll_to(mut self, requested: u16) -> Self {
        let max = self.max_scroll_top();
        self.scroll_top = if requested < max { requested } else { max };
        self.following_end = self.scroll_top == max;
        self
    }

    pub const fn append_content(mut self, content_height: u16) -> Self {
        self.content_height = content_height;
        if self.following_end {
            self.scroll_top = self.max_scroll_top();
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::ScrollState;

    #[test]
    fn following_feed_tracks_appended_content() {
        let state = ScrollState::new(true)
            .update_layout(20, 5)
            .append_content(27);
        assert_eq!(state.scroll_top, 22);
        assert!(state.following_end);
    }

    #[test]
    fn explicit_scroll_detaches_from_tail_until_tail_is_reached() {
        let state = ScrollState::new(true)
            .update_layout(20, 5)
            .scroll_to(3)
            .append_content(27);
        assert_eq!(state.scroll_top, 3);
        assert!(!state.following_end);
    }
}
