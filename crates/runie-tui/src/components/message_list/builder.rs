use super::types::{MessageItem, PlanStatus};
use super::{MessageListViewModel, Feed};
use crate::tui::state::AnimationState;
use crate::components::message_list::render::WrapCache;

/// Builder for constructing feed content with the Runie visual system.
///
/// Visual conventions:
/// - `❯` user messages
/// - `·` reasoning blocks
/// - `◆` tags (thought duration, edits)
/// - `●` tool calls
/// - `!` errors
/// - `[turn: ...]` right-aligned separators
/// - plain text for assistant responses
/// - no blank lines between items
///
/// # Example
/// ```
/// let feed = FeedBuilder::new()
///     .user("hey!")
///     .think("The user is saying hey!")
///     .thought(1.2)
///     .assistant("Hey! How can I help?")
///     .tool("bash", "pwd", Some("/home/user"))
///     .turn(3, 1, 80)
///     .build();
/// ```
#[derive(Default)]
pub(crate) struct FeedBuilder {
    messages: Vec<MessageItem>,
    scroll_offset: usize,
    agent_running: bool,
    animation: AnimationState,
    wrap_cache: WrapCache,
}

impl FeedBuilder {
    pub(crate) fn new() -> Self {
        Self {
            messages: Vec::new(),
            scroll_offset: 0,
            agent_running: false,
            animation: AnimationState::default(),
            wrap_cache: WrapCache::new(),
        }
    }

    /// Add a user message. Renders as: `❯ text`
    pub(crate) fn user(mut self, text: impl Into<String>) -> Self {
        self.messages.push(MessageItem::User {
            text: text.into(),
            model: None,
            timestamp: None,
        });
        self
    }

    /// Add an assistant message. Renders as: plain text
    pub(crate) fn assistant(mut self, text: impl Into<String>) -> Self {
        self.messages.push(MessageItem::Assistant {
            text: text.into(),
            model: None,
            timestamp: None,
        });
        self
    }

    /// Add a think tag. Renders as: `<think> text</think>`
    /// Note: Creates a new Assistant message with think tags
    pub(crate) fn think(mut self, text: impl Into<String>) -> Self {
        // Think blocks are embedded in assistant text with <think> tags
        self.messages.push(MessageItem::Assistant {
            text: format!("<think>{}</think>", text.into()),
            model: None,
            timestamp: None,
        });
        self
    }

    /// Add a standalone think block (not attached to a message). Renders as: `<think> text</think>`
    pub(crate) fn think_block(mut self, text: impl Into<String>) -> Self {
        // For standalone think lines, we use Assistant with think markup
        self.messages.push(MessageItem::Assistant {
            text: format!("<think>{}\n</think>\n", text.into()),
            model: None,
            timestamp: None,
        });
        self
    }

    /// Add a thought duration tag. Renders as: `◆ Thought for Xs`
    pub(crate) fn thought(mut self, duration_secs: f32) -> Self {
        self.messages.push(MessageItem::Thought { duration_secs, text: String::new() });
        self
    }

    /// Add a tool call. Renders as: `● name · args → result`
    pub(crate) fn tool(
        mut self,
        name: impl Into<String>,
        args: impl Into<String>,
        result: Option<impl Into<String>>,
    ) -> Self {
        self.messages.push(MessageItem::ToolCall {
            name: name.into(),
            args: args.into(),
            result: result.map(Into::into),
            is_error: false,
        });
        self
    }

    /// Add a tool call that resulted in an error.
    pub(crate) fn tool_error(
        mut self,
        name: impl Into<String>,
        args: impl Into<String>,
        error: impl Into<String>,
    ) -> Self {
        self.messages.push(MessageItem::ToolCall {
            name: name.into(),
            args: args.into(),
            result: Some(error.into()),
            is_error: true,
        });
        self
    }

    /// Add a file edit. Renders as: `◆ Edit filename`
    pub(crate) fn edit(mut self, filename: impl Into<String>) -> Self {
        self.messages.push(MessageItem::Edit {
            filename: filename.into(),
            diff: None,
        });
        self
    }

    /// Add a system message. Renders as: `· text`
    pub(crate) fn system(mut self, text: impl Into<String>) -> Self {
        self.messages.push(MessageItem::System { text: text.into() });
        self
    }

    /// Add an error. Renders as: `! text`
    pub(crate) fn error(mut self, message: impl Into<String>) -> Self {
        self.messages.push(MessageItem::Error {
            message: message.into(),
            recoverable: true,
        });
        self
    }

    /// Add a turn separator. Renders right-aligned as: `[turn: Xs, Ytc, ⇣Z]`
    pub(crate) fn turn(mut self, elapsed_secs: u64, tool_calls: usize, tokens: usize) -> Self {
        self.messages.push(MessageItem::Separator {
            elapsed_secs,
            tool_calls,
            tokens_used: Some(tokens),
        });
        self
    }

    /// Add a plan step.
    pub(crate) fn plan_step(mut self, step: usize, text: impl Into<String>, status: PlanStatus) -> Self {
        self.messages.push(MessageItem::PlanStep {
            step,
            text: text.into(),
            status,
        });
        self
    }

    /// Add an interrupt marker.
    pub(crate) fn interrupt(mut self) -> Self {
        self.messages.push(MessageItem::Interrupt);
        self
    }

    /// Set scroll offset.
    pub(crate) fn scroll_offset(mut self, offset: usize) -> Self {
        self.scroll_offset = offset;
        self
    }

    /// Set agent running state.
    pub(crate) fn agent_running(mut self, running: bool) -> Self {
        self.agent_running = running;
        self
    }

    /// Set animation state.
    pub(crate) fn animation(mut self, animation: AnimationState) -> Self {
        self.animation = animation;
        self
    }

    /// Set messages from a slice (for AppState integration).
    pub(crate) fn messages(mut self, messages: &[MessageItem]) -> Self {
        self.messages.extend_from_slice(messages);
        self
    }

    /// Set wrap cache (for AppState integration).
    pub(crate) fn wrap_cache(mut self, cache: WrapCache) -> Self {
        self.wrap_cache = cache;
        self
    }

    /// Consume the builder and return the MessageListViewModel.
    pub(crate) fn build(self) -> MessageListViewModel {
        MessageListViewModel {
            feed: Feed::from(self.messages),
            scroll_offset: self.scroll_offset,
            agent_running: self.agent_running,
            animation: self.animation,
            wrap_cache: self.wrap_cache,
        }
    }

    /// Extend an existing message list.
    pub(crate) fn extend_into(self, list: &mut Vec<MessageItem>) {
        list.extend(self.messages);
    }
}

#[cfg(test)]
mod tests {
    use super::FeedBuilder;
    use super::super::FeedItem;

    #[test]
    fn test_feed_builder_basic() {
        let vm = FeedBuilder::new()
            .user("hey!")
            .think("The user is saying hey!")
            .thought(1.2)
            .assistant("Hey! How can I help?")
            .tool("bash", "pwd", Some("/home/user"))
            .turn(3, 1, 80)
            .build();

        let items = vm.feed.items();
        // In new architecture: thoughts/tool_calls/turns are inline in AssistantMessage
        // So we get: UserMessage, AssistantMessage (from think), AssistantMessage (from assistant)
        assert_eq!(items.len(), 3);
        assert!(matches!(&items[0], FeedItem::UserMessage { .. }));
        assert!(matches!(&items[1], FeedItem::AssistantMessage { .. })); // think block
        assert!(matches!(&items[2], FeedItem::AssistantMessage { .. })); // assistant response
    }

    #[test]
    fn test_feed_builder_error() {
        let vm = FeedBuilder::new()
            .user("do something risky")
            .tool_error("rm", "-rf /", "permission denied")
            .error("Something went wrong")
            .build();

        let items = vm.feed.items();
        // Error renders as SystemNotice; tool_error still filtered
        assert_eq!(items.len(), 2);
        assert!(matches!(&items[0], FeedItem::UserMessage { .. }));
        assert!(matches!(&items[1], FeedItem::SystemNotice { .. }));
    }
}