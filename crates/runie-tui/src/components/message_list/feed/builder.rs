//! Builder API for Feed construction.

use std::time::Duration;
use serde_json::Value;
use uuid::Uuid;

use super::{Feed, FeedItem, Thought, ToolCall};

/// Fluent builder for constructing Feed with declarative syntax.
///
/// # Example
///
/// ```
/// use runie_tui::components::message_list::feed::FeedBuilder;
///
/// let feed = FeedBuilder::new()
///     .user_message("Hello!")
///     .assistant()
///         .thinking_for(std::time::Duration::from_secs(1))
///         .say("Hi there!")
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct FeedBuilder {
    items: Vec<FeedItem>,
    state: BuilderState,
}

#[derive(Debug, Clone)]
enum BuilderState {
    /// Ready to add user message or build
    Idle,
    /// After user_message, expecting .assistant() or another user_message
    AfterUser,
    #[allow(dead_code)]
    /// Inside an assistant builder
    InAssistant(AssistantBuilder),
    /// Assistant builder ready for next method
    AssistantReady(AssistantBuilder),
    /// Turn duration pending to be applied to last assistant
    TurnDurationPending(f32),
}

#[derive(Debug, Clone)]
struct AssistantBuilder {
    id: String,
    text: String,
    thoughts: Vec<Thought>,
    tool_calls: Vec<ToolCall>,
}

impl FeedBuilder {
    /// Start building a new Feed.

    #[must_use]
    #[must_use]
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            state: BuilderState::Idle,
        }
    }

    /// Add a user message and continue the chain.
    pub fn user_message(mut self, text: impl Into<String>) -> Self {
        self.end_any_assistant();

        let id = Uuid::new_v4().to_string();
        self.items.push(FeedItem::UserMessage {
            id: id.clone(),
            text: text.into(),
            timestamp: None,
        });
        self.state = BuilderState::AfterUser;
        self
    }

    /// Set turn completion duration (applied to last assistant message on build).
    pub fn turn_completed_in(mut self, duration: f32) -> Self {
        if let BuilderState::AssistantReady(_) = &self.state {
            self.state = BuilderState::TurnDurationPending(duration);
        }
        self
    }

    /// Begin an assistant message. Returns builder for assistant content.
    pub fn assistant(mut self) -> AssistantFeedBuilder {
        self.end_any_assistant();

        let id = Uuid::new_v4().to_string();
        let assistant = AssistantBuilder {
            id,
            text: String::new(),
            thoughts: Vec::new(),
            tool_calls: Vec::new(),
        };

        // If we have items and last is user, transition to in_assistant
        let has_user = matches!(&self.state, BuilderState::AfterUser);

        AssistantFeedBuilder {
            feed_builder: self,
            assistant,
            requires_user_before: has_user,
            turn_duration: None,
        }
    }

    /// Internal: finalize any in-progress assistant before new state.
    fn end_any_assistant(&mut self) {
        if let BuilderState::InAssistant(builder) | BuilderState::AssistantReady(builder) =
            std::mem::replace(&mut self.state, BuilderState::Idle)
        {
            self.items.push(FeedItem::AssistantMessage {
                id: builder.id,
                text: builder.text,
                thoughts: builder.thoughts,
                tool_calls: builder.tool_calls,
                timestamp: None,
                turn_duration: None,
            });
        }
    }

    /// Finalize the builder and return the Feed.
    pub fn build(mut self) -> Feed {
        self.end_any_assistant();

        // Apply pending turn duration if any
        if let BuilderState::TurnDurationPending(duration) = &self.state {
            if let Some(last) = self.items.last_mut() {
                if let FeedItem::AssistantMessage { turn_duration, .. } = last {
                    *turn_duration = Some(*duration);
                }
            }
        }

        Feed {
            items: self.items,
            seen_ids: Default::default(),
        }
    }
}

impl Default for FeedBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for assistant message content.
#[derive(Debug, Clone)]
pub struct AssistantFeedBuilder {
    feed_builder: FeedBuilder,
    assistant: AssistantBuilder,
    #[allow(dead_code)]
    requires_user_before: bool,
    turn_duration: Option<f32>,
}

impl AssistantFeedBuilder {
    /// Add a thinking period with duration.
    pub fn thinking_for(mut self, duration: Duration) -> Self {
        self.assistant.thoughts.push(Thought {
            duration: duration.as_secs_f32(),
        });
        self
    }

    /// Add text content (final response).
    pub fn say(mut self, text: impl Into<String>) -> Self {
        // If we already have text, append with newline
        if !self.assistant.text.is_empty() {
            self.assistant.text.push('\n');
        }
        self.assistant.text.push_str(&text.into());
        self
    }

    /// Add a code block (appends to text).
    pub fn code_block(self, code: impl Into<String>) -> Self {
        self.say(format!("```\n{}```", code.into()))
    }

    /// Add a code block with language tag.
    pub fn code_block_with_lang(self, lang: &str, code: impl Into<String>) -> Self {
        self.say(format!("```{}\n{}```", lang, code.into()))
    }

    /// Add a tool call.
    pub fn tool_call(mut self, name: impl Into<String>, args: Value) -> Self {
        self.assistant.tool_calls.push(ToolCall {
            name: name.into(),
            args: serde_json::to_string(&args).unwrap_or_default(),
        });
        self
    }

    /// Set the turn completion duration.
    pub fn turn_completed_in(mut self, duration: Duration) -> Self {
        self.turn_duration = Some(duration.as_secs_f32());
        self
    }

    /// Build the Feed directly from the assistant builder.
    pub fn build(self) -> Feed {
        self.done().build()
    }

    /// Add another user message (returns to FeedBuilder).
    pub fn user_message(self, text: impl Into<String>) -> FeedBuilder {
        self.done().user_message(text)
    }

    /// Continue building the feed (return to parent builder).
    pub fn done(self) -> FeedBuilder {
        let mut fb = self.feed_builder;
        fb.state = BuilderState::AssistantReady(self.assistant);
        if let Some(duration) = self.turn_duration {
            fb.state = BuilderState::TurnDurationPending(duration);
        }
        fb
    }
}

// Allow chaining from assistant back to feed builder
impl From<AssistantFeedBuilder> for FeedBuilder {
    fn from(a: AssistantFeedBuilder) -> Self {
        a.done()
    }
}

// ============================================================================
// Convenience Traits for Even Shorter Syntax
// ============================================================================

/// Extension trait for chaining user_message on FeedBuilder or &mut Feed.
pub trait FeedChainable {
    fn user_message(self, text: &str) -> Self;
}

impl FeedChainable for FeedBuilder {
    fn user_message(self, text: &str) -> Self {
        self.user_message(text.to_string())
    }
}
