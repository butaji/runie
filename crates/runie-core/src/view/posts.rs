//! Post DSL — build navigable feed posts in a fluent, declarative style.
//!
//! A `Post` is the user's mental model for a feed item: a user message,
//! an agent response, a thought, a tool execution, etc. The DSL lets the
//! transform layer describe *what* kind of post it is adding without
//! worrying about element indices or spacer bookkeeping.
//!
//! Example:
//!
//! ```rust,ignore
//! use runie_core::ui::posts::{PostBuilder, PostKind};
//! use runie_core::ui::elements::Feed;
//!
//! let mut feed = Feed::new();
//! feed.push_post(
//!     PostBuilder::new(PostKind::UserInput)
//!         .with_element(Element::user("hello").at(1.0))
//!         .expanded(true)
//!         .at(1.0),
//! );
//! ```

use crate::view::elements::{Element, Feed, Post, PostKind};

/// Fluent builder for a single feed post.
#[derive(Debug, Clone)]
pub struct PostBuilder {
    kind: PostKind,
    elements: Vec<Element>,
    timestamp: f64,
    expanded: bool,
}

impl PostBuilder {
    /// Start building a post of the given logical kind.
    pub fn new(kind: PostKind) -> Self {
        Self {
            kind,
            elements: Vec::new(),
            timestamp: 0.0,
            expanded: true,
        }
    }

    /// Add an element to the post body. Elements are rendered in order.
    pub fn with_element(mut self, element: Element) -> Self {
        self.elements.push(element);
        self
    }

    /// Set whether the post body is expanded (default: true).
    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }

    /// Set the timestamp used for ordering the post and its trailing spacer.
    pub fn at(mut self, timestamp: f64) -> Self {
        self.timestamp = timestamp;
        self
    }

    /// Consume the builder and append the post to the feed. Returns the
    /// index of the newly created post.
    pub fn build(self, feed: &mut Feed) -> usize {
        // Add a leading spacer for the very first post when it is not a
        // user message. User messages already have internal top/bottom
        // margins, so their bracket naturally spans content + 2 rows.
        // Other post types need a spacer above them so the selection
        // bracket can form a full `[` shape even at the top of the feed.
        if feed.elements.is_empty() && self.kind != PostKind::UserInput {
            feed.elements.push(Element::spacer().at(self.timestamp));
        }

        let start = feed.elements.len();
        let mut elements = self.elements;
        for element in &mut elements {
            element.set_timestamp(self.timestamp);
        }
        feed.elements.extend(elements);
        feed.elements.push(Element::spacer().at(self.timestamp));

        let index = feed.posts.len();
        feed.posts.push(Post {
            index,
            start,
            end: feed.elements.len(),
            kind: self.kind,
            expanded: self.expanded,
        });
        index
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn post_builder_records_range_and_kind() {
        let mut feed = Feed::new();
        let idx = feed.push_post_and_index(
            PostBuilder::new(PostKind::UserInput)
                .with_element(Element::user("hi").at(0.0))
                .at(1.0),
        );
        assert_eq!(idx, 0);
        assert_eq!(feed.posts.len(), 1);
        assert_eq!(feed.posts[0].kind, PostKind::UserInput);
        assert!(feed.posts[0].expanded);
        // user element + spacer
        assert_eq!(feed.posts[0].len(), 2);
    }

    #[test]
    fn collapsed_post_carries_expanded_flag() {
        let mut feed = Feed::new();
        feed.push_post(
            PostBuilder::new(PostKind::Thought)
                .with_element(Element::thought("deep thought").at(0.0))
                .expanded(false)
                .at(2.0),
        );
        assert!(!feed.posts[0].expanded);
    }

    #[test]
    fn multiple_posts_get_increasing_indices() {
        let mut feed = Feed::new();
        feed.push_post(
            PostBuilder::new(PostKind::UserInput)
                .with_element(Element::user("a").at(0.0))
                .at(0.0),
        );
        feed.push_post(
            PostBuilder::new(PostKind::AgentResponse)
                .with_element(Element::agent("b").at(0.0))
                .at(1.0),
        );
        assert_eq!(feed.posts[0].index, 0);
        assert_eq!(feed.posts[1].index, 1);
        assert_eq!(feed.posts[1].start, feed.posts[0].end);
    }
}
