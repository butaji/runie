//! Feed - single source of truth for conversation feed state.
//!
//! Key invariants:
//! - UserMessage always precedes AssistantMessage (never follows)
//! - AssistantMessage contains its thoughts and tool_calls inline
//! - Thoughts/ToolCalls are NOT separate feed items (they attach to assistant)
//! - Streaming content appended in-place via append_to_last

use std::collections::HashSet;
use uuid::Uuid;

pub mod builder;
pub mod convert;

#[derive(Debug, Clone, PartialEq)]
pub struct Thought {
    pub duration: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToolCall {
    pub name: String,
    pub args: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FeedItem {
    UserMessage { id: String, text: String, timestamp: Option<String> },
    AssistantMessage {
        id: String,
        text: String,
        thoughts: Vec<Thought>,
        tool_calls: Vec<ToolCall>,
        timestamp: Option<String>,
        turn_duration: Option<f32>,
        thoughts_collapsed: bool,
        expanded: bool,
        /// Streaming stats for bottom spinner during agent generation
        streaming_thinking_elapsed_ms: Option<u64>,
        streaming_total_elapsed_ms: Option<u64>,
        streaming_download_bytes: Option<u64>,
    },
    SystemNotice { text: String },
    /// Separator between conversation turns showing elapsed time and metrics
    Separator { elapsed_secs: u64, tool_calls: usize, tokens_used: Option<usize> },
    /// Tool execution in progress (shown during tool execution)
    ToolRunning {
        name: String,
        args: String,
        duration_ms: u64,
        total_elapsed_ms: u64,
        download_bytes: u64,
    },
    /// Tool execution completed
    ToolComplete {
        name: String,
        result: String,
        lines: Option<usize>,
    },
}

impl FeedItem {
    pub fn id(&self) -> &str {
        match self {
            FeedItem::UserMessage { id, .. } => id,
            FeedItem::AssistantMessage { id, .. } => id,
            FeedItem::SystemNotice { .. } => "",
            FeedItem::Separator { .. } => "",
            FeedItem::ToolRunning { .. } => "",
            FeedItem::ToolComplete { .. } => "",
        }
    }

    pub fn is_message(&self) -> bool {
        matches!(self, FeedItem::UserMessage { .. } | FeedItem::AssistantMessage { .. })
    }
}

/// Feed - single source of truth for feed state.
///
/// Maintains valid conversation structure:
/// - Messages in order: User → Assistant → User → Assistant...
/// - Thoughts and ToolCalls attached to AssistantMessage, not separate items
#[derive(Debug, Clone, PartialEq)]
pub struct Feed {
    items: Vec<FeedItem>,
    /// IDs for deduplication
    seen_ids: HashSet<String>,
}

impl Default for Feed {
    fn default() -> Self {
        Self::new()
    }
}

impl Feed {

    #[must_use]
    
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            seen_ids: HashSet::new(),
        }
    }

    /// Create a new FeedBuilder for declarative feed construction.
    pub fn builder() -> crate::components::message_list::feed::builder::FeedBuilder {
        crate::components::message_list::feed::builder::FeedBuilder::new()
    }

    /// Add a user message. Returns mutable reference to the inserted item.
    pub fn add_user_message(&mut self, text: String) -> &mut FeedItem {
        let id = Uuid::new_v4().to_string();
        self.items.push(FeedItem::UserMessage {
            id: id.clone(),
            text,
            timestamp: None,
        });
        self.seen_ids.insert(id);
        let idx = self.items.len().saturating_sub(1);
        &mut self.items[idx]
    }

    /// Begin an assistant message. Returns mutable reference to the inserted item.
    pub fn add_assistant_message(&mut self) -> &mut FeedItem {
        let id = Uuid::new_v4().to_string();
        self.items.push(FeedItem::AssistantMessage {
            id: id.clone(),
            text: String::new(),
            thoughts: Vec::new(),
            tool_calls: Vec::new(),
            timestamp: None,
            turn_duration: None,
            thoughts_collapsed: false,
            expanded: true,
            streaming_thinking_elapsed_ms: None,
            streaming_total_elapsed_ms: None,
            streaming_download_bytes: None,
        });
        self.seen_ids.insert(id);
        let idx = self.items.len().saturating_sub(1);
        &mut self.items[idx]
    }

    /// Append text to the last assistant message.
    /// For streaming content updates.
    /// Note: Only appends to AssistantMessage, not UserMessage (user messages are immutable).
    pub fn append_to_last(&mut self, text: &str) {
        if let Some(last) = self.items.last_mut() {
            if let FeedItem::AssistantMessage { text: ref mut t, .. } = last {
                t.push_str(text);
            }
        }
    }

    /// Update text of last assistant message (full replacement, for streaming).
    pub fn update_last_assistant_text(&mut self, text: &str) {
        if let Some(FeedItem::AssistantMessage { text: ref mut t, .. }) = self.items.last_mut() {
            *t = text.to_string();
        }
    }

    /// Add a thought to the last assistant message.
    /// Silently no-op if no assistant message exists.
    pub fn add_thought(&mut self, duration: f32) {
        if let Some(FeedItem::AssistantMessage { thoughts, .. }) = self.items.last_mut() {
            thoughts.push(Thought { duration });
        }
    }

    /// Add a tool call to the last assistant message.
    /// Silently no-op if no assistant message exists.
    pub fn add_tool_call(&mut self, name: String, args: String) {
        if let Some(FeedItem::AssistantMessage { tool_calls, .. }) = self.items.last_mut() {
            tool_calls.push(ToolCall { name, args });
        }
    }

    /// Complete the current turn with duration.
    /// Sets turn_duration on the last assistant message.
    pub fn complete_turn(&mut self, duration: f32) {
        if let Some(FeedItem::AssistantMessage { turn_duration, .. }) = self.items.last_mut() {
            *turn_duration = Some(duration);
        }
    }

    /// Set streaming stats for the bottom spinner on last assistant message.
    /// Silently no-op if no assistant message exists.
    pub fn set_streaming_stats(&mut self, thinking_elapsed_ms: u64, total_elapsed_ms: u64, download_bytes: u64) {
        if let Some(FeedItem::AssistantMessage {
            streaming_thinking_elapsed_ms,
            streaming_total_elapsed_ms,
            streaming_download_bytes,
            ..
        }) = self.items.last_mut() {
            *streaming_thinking_elapsed_ms = Some(thinking_elapsed_ms);
            *streaming_total_elapsed_ms = Some(total_elapsed_ms);
            *streaming_download_bytes = Some(download_bytes);
        }
    }

    /// Get all feed items.
    pub fn items(&self) -> &[FeedItem] {
        &self.items
    }

    /// Get mutable reference to items.
    pub fn items_mut(&mut self) -> &mut Vec<FeedItem> {
        &mut self.items
    }

    /// Clear all items.
    pub fn clear(&mut self) {
        self.items.clear();
        self.seen_ids.clear();
    }

    /// Check if an ID has been seen (for deduplication).
    pub fn has_id(&self, id: &str) -> bool {
        self.seen_ids.contains(id)
    }

    /// Add an item only if ID hasn't been seen (dedup by ID).
    /// Returns true if item was added.
    pub fn add_if_new(&mut self, item: FeedItem) -> bool {
        let id = item.id();
        if id.is_empty() || !self.seen_ids.contains(id) {
            if !id.is_empty() {
                self.seen_ids.insert(id.to_string());
            }
            self.items.push(item);
            true
        } else {
            false
        }
    }

    /// Check if last item is an assistant message (for streaming check).
    pub fn has_assistant_in_progress(&self) -> bool {
        matches!(self.items.last(), Some(FeedItem::AssistantMessage { .. }))
    }

    /// Get last assistant message if exists.
    pub fn last_assistant_mut(&mut self) -> Option<&mut FeedItem> {
        if let Some(idx) = self.items.iter().rposition(|i| matches!(i, FeedItem::AssistantMessage { .. })) {
            // Return only if it's the last item
            if idx == self.items.len() - 1 {
                return self.items.get_mut(idx);
            }
        }
        None
    }

    /// Get the count of items.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Add a system notice.
    pub fn add_system_notice(&mut self, text: String) {
        self.items.push(FeedItem::SystemNotice { text });
    }

    /// Add a tool running status item.
    pub fn add_tool_running(&mut self, name: String, args: String, duration_ms: u64, total_elapsed_ms: u64, download_bytes: u64) {
        self.items.push(FeedItem::ToolRunning {
            name,
            args,
            duration_ms,
            total_elapsed_ms,
            download_bytes,
        });
    }

    /// Add a tool complete status item.
    pub fn add_tool_complete(&mut self, name: String, result: String, lines: Option<usize>) {
        self.items.push(FeedItem::ToolComplete {
            name,
            result,
            lines,
        });
    }

    /// Remove last item if it's an assistant with empty text.
    pub fn remove_last_empty_assistant(&mut self) {
        if let Some(FeedItem::AssistantMessage { text, thoughts, tool_calls, .. }) = self.items.last() {
            if text.is_empty() && thoughts.is_empty() && tool_calls.is_empty() {
                self.items.pop();
            }
        }
    }
}



#[cfg(test)]
mod feed_tests;
