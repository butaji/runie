//! Feed element detail overlay state.
//!
//! Opened when the user presses Enter on any feed element in vim nav mode.
//! Displays the full content of the element in a bordered modal overlay,
//! with scrolling support.

use crate::view::elements::PostKind;

/// Detail view state for an open feed element overlay.
#[derive(Debug, Clone, PartialEq)]
pub struct FeedElementDetail {
    /// Index of the element in the feed's element list.
    pub element_index: usize,
    /// Scroll position within the element content.
    pub scroll: usize,
    /// The kind of element being shown.
    pub kind: FeedElementKind,
}

/// Discriminated union of all feed element kinds that can be shown in the detail overlay.
#[derive(Debug, Clone, PartialEq)]
pub enum FeedElementKind {
    UserInput { content: String },
    AgentResponse { content: String, provider: String },
    Thought { content: String },
    ToolRunning { name: String, args: String },
    ToolDone { name: String, args: String, output: String },
    ToolSummary { name: String },
    ContextGroup,
    SubagentRow { worker_id: String },
    TurnComplete { duration_secs: f64 },
    System { content: String },
}

impl FeedElementKind {
    /// Human-readable label for the element kind.
    pub fn label(&self) -> &'static str {
        match self {
            FeedElementKind::UserInput { .. } => "User Input",
            FeedElementKind::AgentResponse { .. } => "Agent Response",
            FeedElementKind::Thought { .. } => "Thought",
            FeedElementKind::ToolRunning { .. } => "Tool Running",
            FeedElementKind::ToolDone { .. } => "Tool Done",
            FeedElementKind::ToolSummary { .. } => "Tool Summary",
            FeedElementKind::ContextGroup { .. } => "Context Group",
            FeedElementKind::SubagentRow { .. } => "Subagent",
            FeedElementKind::TurnComplete { .. } => "Turn Complete",
            FeedElementKind::System { .. } => "System",
        }
    }
}

impl FeedElementDetail {
    /// Build a detail from a PostKind and element index.
    pub fn from_postkind(kind: PostKind, element_index: usize) -> Option<Self> {
        Some(Self {
            element_index,
            scroll: 0,
            kind: match kind {
                PostKind::UserInput => FeedElementKind::UserInput {
                    content: String::new(),
                },
                PostKind::AgentResponse => FeedElementKind::AgentResponse {
                    content: String::new(),
                    provider: String::new(),
                },
                PostKind::Thought => FeedElementKind::Thought {
                    content: String::new(),
                },
                PostKind::ToolRunning => FeedElementKind::ToolRunning {
                    name: String::new(),
                    args: String::new(),
                },
                PostKind::ToolDone => FeedElementKind::ToolDone {
                    name: String::new(),
                    args: String::new(),
                    output: String::new(),
                },
                PostKind::ToolSummary => FeedElementKind::ToolSummary {
                    name: String::new(),
                },
                PostKind::ContextGroup => FeedElementKind::ContextGroup,
                PostKind::SubagentRow => FeedElementKind::SubagentRow {
                    worker_id: String::new(),
                },
                PostKind::TurnComplete => FeedElementKind::TurnComplete { duration_secs: 0.0 },
                PostKind::System => FeedElementKind::System {
                    content: String::new(),
                },
                PostKind::Thinking => return None,
            },
        })
    }

    /// Return the body text to display in the overlay.
    pub fn body_text(&self) -> String {
        match &self.kind {
            FeedElementKind::UserInput { content } => content.clone(),
            FeedElementKind::AgentResponse { content, .. } => content.clone(),
            FeedElementKind::Thought { content } => content.clone(),
            FeedElementKind::ToolRunning { name, args } => {
                format!("Tool: {name}\n\nArguments:\n{args}")
            }
            FeedElementKind::ToolDone { name, args, output } => {
                format!("Tool: {name}\n\nArguments:\n{args}\n\nOutput:\n{output}")
            }
            FeedElementKind::ToolSummary { name } => format!("Tool: {name}"),
            FeedElementKind::ContextGroup => "[Context group contents]".to_string(),
            FeedElementKind::SubagentRow { worker_id } => {
                format!("Worker ID: {worker_id}")
            }
            FeedElementKind::TurnComplete { duration_secs } => {
                format!("Turn completed in {:.1}s", duration_secs)
            }
            FeedElementKind::System { content } => content.clone(),
        }
    }
}
