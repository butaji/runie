use super::types::{MessageItem, PlanStatus};

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
#[derive(Debug, Default)]
pub struct FeedBuilder {
    messages: Vec<MessageItem>,
}

impl FeedBuilder {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

    /// Add a user message. Renders as: `❯ text`
    pub fn user(mut self, text: impl Into<String>) -> Self {
        self.messages.push(MessageItem::User {
            text: text.into(),
            model: Some("You".to_string()),
            timestamp: None,
        });
        self
    }

    /// Add an assistant message. Renders as plain text.
    pub fn assistant(mut self, text: impl Into<String>) -> Self {
        self.messages.push(MessageItem::Assistant {
            text: text.into(),
            model: None,
            timestamp: None,
        });
        self
    }

    /// Add a thinking/reasoning block. Renders as: `· text`
    pub fn think(mut self, text: impl Into<String>) -> Self {
        // Think blocks are embedded in assistant text with <think> tags
        // For standalone think lines, we use Assistant with think markup
        self.messages.push(MessageItem::Assistant {
            text: format!("<think>{}</think>", text.into()),
            model: None,
            timestamp: None,
        });
        self
    }

    /// Add a thought duration tag. Renders as: `◆ Thought for Xs`
    pub fn thought(mut self, duration_secs: f32) -> Self {
        self.messages.push(MessageItem::Thought { duration_secs });
        self
    }

    /// Add a tool call. Renders as: `● name · args → result`
    pub fn tool(
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
    pub fn tool_error(
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
    pub fn edit(mut self, filename: impl Into<String>) -> Self {
        self.messages.push(MessageItem::Edit {
            filename: filename.into(),
            diff: None,
        });
        self
    }

    /// Add a system message. Renders as: `· text`
    pub fn system(mut self, text: impl Into<String>) -> Self {
        self.messages.push(MessageItem::System { text: text.into() });
        self
    }

    /// Add an error. Renders as: `! text`
    pub fn error(mut self, message: impl Into<String>) -> Self {
        self.messages.push(MessageItem::Error {
            message: message.into(),
            recoverable: true,
        });
        self
    }

    /// Add a turn separator. Renders right-aligned as: `[turn: Xs, Ytc, ⇣Z]`
    pub fn turn(mut self, elapsed_secs: u64, tool_calls: usize, tokens: usize) -> Self {
        self.messages.push(MessageItem::Separator {
            elapsed_secs,
            tool_calls,
            tokens_used: Some(tokens),
        });
        self
    }

    /// Add a plan step.
    pub fn plan_step(mut self, step: usize, text: impl Into<String>, status: PlanStatus) -> Self {
        self.messages.push(MessageItem::PlanStep {
            step,
            text: text.into(),
            status,
        });
        self
    }

    /// Add an interrupt marker.
    pub fn interrupt(mut self) -> Self {
        self.messages.push(MessageItem::Interrupt);
        self
    }

    /// Consume the builder and return the messages.
    pub fn build(self) -> Vec<MessageItem> {
        self.messages
    }

    /// Extend an existing message list.
    pub fn extend_into(self, list: &mut Vec<MessageItem>) {
        list.extend(self.messages);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feed_builder_basic() {
        let feed = FeedBuilder::new()
            .user("hey!")
            .think("The user is saying hey!")
            .thought(1.2)
            .assistant("Hey! How can I help?")
            .tool("bash", "pwd", Some("/home/user"))
            .turn(3, 1, 80)
            .build();

        assert_eq!(feed.len(), 6);
        assert!(matches!(feed[0], MessageItem::User { .. }));
        assert!(matches!(feed[1], MessageItem::Assistant { .. }));
        assert!(matches!(feed[2], MessageItem::Thought { .. }));
        assert!(matches!(feed[3], MessageItem::Assistant { .. }));
        assert!(matches!(feed[4], MessageItem::ToolCall { .. }));
        assert!(matches!(feed[5], MessageItem::Separator { .. }));
    }

    #[test]
    fn test_feed_builder_error() {
        let feed = FeedBuilder::new()
            .user("do something risky")
            .tool_error("rm", "-rf /", "permission denied")
            .error("Something went wrong")
            .build();

        assert_eq!(feed.len(), 3);
        assert!(matches!(feed[1], MessageItem::ToolCall { is_error: true, .. }));
        assert!(matches!(feed[2], MessageItem::Error { .. }));
    }
}
