//! Feed element detail overlay state.
//!
//! Opened when the user presses Enter on any feed element in vim nav mode.
//! Displays the full content of the element in a bordered modal overlay,
//! with scrolling support.

use crate::view::elements::{Element, PostKind};

/// Detail view state for an open feed element overlay.
#[derive(Debug, Clone, PartialEq)]
pub struct FeedElementDetail {
    /// Index of the element in the feed's element list.
    pub element_index: usize,
    /// Scroll position within the element content.
    pub scroll: usize,
    /// The kind of element being shown.
    pub kind: FeedElementKind,
    /// Grok-style in-viewer search state. `search_editing` owns text input;
    /// accepted queries remain active for `n`/`N` navigation.
    pub search_query: String,
    pub search_editing: bool,
    pub search_filter: bool,
    pub search_matches: Vec<usize>,
    pub search_current: usize,
    pub visual_anchor: Option<usize>,
    pub wrap: bool,
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
            FeedElementKind::ContextGroup => "Context Group",
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
                PostKind::UserInput => FeedElementKind::UserInput { content: String::new() },
                PostKind::AgentResponse => {
                    FeedElementKind::AgentResponse { content: String::new(), provider: String::new() }
                }
                PostKind::Thought => FeedElementKind::Thought { content: String::new() },
                PostKind::ToolRunning => FeedElementKind::ToolRunning { name: String::new(), args: String::new() },
                PostKind::ToolDone => {
                    FeedElementKind::ToolDone { name: String::new(), args: String::new(), output: String::new() }
                }
                PostKind::ToolSummary => FeedElementKind::ToolSummary { name: String::new() },
                PostKind::ContextGroup => FeedElementKind::ContextGroup,
                PostKind::SubagentRow => FeedElementKind::SubagentRow { worker_id: String::new() },
                PostKind::TurnComplete => FeedElementKind::TurnComplete { duration_secs: 0.0 },
                PostKind::System => FeedElementKind::System { content: String::new() },
                PostKind::Thinking => return None,
            },
            search_query: String::new(),
            search_editing: false,
            search_filter: false,
            search_matches: Vec::new(),
            search_current: 0,
            visual_anchor: None,
            wrap: true,
        })
    }

    /// Build a detail from the actual selected feed element. The old
    /// `from_postkind` constructor intentionally supplied empty placeholders;
    /// viewers must retain the selected block's content when opened.
    pub fn from_element(kind: PostKind, element_index: usize, element: &Element) -> Option<Self> {
        let mut detail = Self::from_postkind(kind, element_index)?;
        detail.kind = match element {
            Element::UserMessage { content, .. } => FeedElementKind::UserInput { content: content.clone() },
            Element::AgentMessage { content, provider, .. } => {
                FeedElementKind::AgentResponse { content: content.clone(), provider: provider.clone() }
            }
            Element::ThoughtMarker { content, .. } | Element::AnthropicThinking { content, .. } => {
                FeedElementKind::Thought { content: content.clone() }
            }
            Element::ToolRunning { name, args, .. } => {
                FeedElementKind::ToolRunning { name: name.clone(), args: args.clone() }
            }
            Element::ToolDone { name, args, output, .. } => {
                FeedElementKind::ToolDone { name: name.clone(), args: args.clone(), output: output.clone() }
            }
            Element::ToolSummary { name, .. } => FeedElementKind::ToolSummary { name: name.clone() },
            Element::ContextGroup { .. } => FeedElementKind::ContextGroup,
            Element::SubagentRow { id, .. } => FeedElementKind::SubagentRow { worker_id: id.clone() },
            Element::TurnComplete { duration_secs, .. } => {
                FeedElementKind::TurnComplete { duration_secs: *duration_secs }
            }
            Element::SystemMessage { content, .. } => FeedElementKind::System { content: content.clone() },
            _ => return None,
        };
        Some(detail)
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

    /// Metadata payload used by Grok's uppercase `Y` viewer shortcut.
    pub fn metadata_text(&self) -> String {
        match &self.kind {
            FeedElementKind::ToolRunning { name, args } | FeedElementKind::ToolDone { name, args, .. } => {
                format!("Tool: {name}\nArguments:\n{args}")
            }
            FeedElementKind::ToolSummary { name } => format!("Tool: {name}"),
            FeedElementKind::SubagentRow { worker_id } => format!("Worker ID: {worker_id}"),
            FeedElementKind::AgentResponse { provider, .. } => format!("Provider: {provider}"),
            _ => self.body_text(),
        }
    }

    pub fn refresh_search(&mut self) {
        let query = self.search_query.to_lowercase();
        self.search_matches = if query.is_empty() {
            Vec::new()
        } else {
            self.body_text()
                .lines()
                .enumerate()
                .filter_map(|(index, line)| line.to_lowercase().contains(&query).then_some(index))
                .collect()
        };
        self.search_current = self
            .search_current
            .min(self.search_matches.len().saturating_sub(1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_element_keeps_selected_tool_content() {
        let element = Element::ToolDone {
            name: "list_dir".into(),
            args: "{\"path\":\".\"}".into(),
            duration_secs: 0.2,
            output: "Cargo.toml\nsrc/".into(),
            bytes_transferred: None,
            error: false,
            finished_at: None,
            timestamp: 0.0,
        };
        let detail = FeedElementDetail::from_element(PostKind::ToolDone, 3, &element).unwrap();
        assert_eq!(detail.element_index, 3);
        assert!(detail.body_text().contains("Cargo.toml"));
        assert!(detail.body_text().contains("list_dir"));
    }

    #[test]
    fn search_matches_lines_case_insensitively() {
        let mut detail = FeedElementDetail::from_postkind(PostKind::System, 0).unwrap();
        detail.kind = FeedElementKind::System { content: "First\nCargo.toml\nsecond cargo".into() };
        detail.search_query = "CARGO".into();
        detail.refresh_search();
        assert_eq!(detail.search_matches, vec![1, 2]);
        assert_eq!(detail.search_current, 0);
    }

    #[test]
    fn filter_mode_reuses_search_matches() {
        let mut detail = FeedElementDetail::from_postkind(PostKind::System, 0).unwrap();
        detail.kind = FeedElementKind::System { content: "keep\ndrop\nkeep this".into() };
        detail.search_query = "keep".into();
        detail.search_filter = true;
        detail.refresh_search();
        assert_eq!(detail.search_matches, vec![0, 2]);
        assert!(detail.search_filter);
    }

    #[test]
    fn metadata_copy_omits_tool_output() {
        let detail = FeedElementDetail {
            element_index: 0,
            scroll: 0,
            kind: FeedElementKind::ToolDone {
                name: "bash".into(),
                args: "{\"command\":\"pwd\"}".into(),
                output: "secret output".into(),
            },
            search_query: String::new(),
            search_editing: false,
            search_filter: false,
            search_matches: Vec::new(),
            search_current: 0,
            visual_anchor: None,
            wrap: true,
        };
        assert!(detail.metadata_text().contains("bash"));
        assert!(!detail.metadata_text().contains("secret output"));
    }
}
