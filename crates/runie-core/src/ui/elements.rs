//! UI Elements - Building blocks for the DSL

#[derive(Debug, Clone)]
pub enum Element {
    Spacer,
    UserMessage { content: String, timestamp: String },
    AgentMessage { content: String, timestamp: String },
    Thinking { started: std::time::Instant },
    ThoughtMarker { content: String },
    ThoughtSummary { content: String, duration_secs: f64 },
    ToolRunning { name: String, started: std::time::Instant },
    ToolDone { name: String, duration_secs: f64, output: String },
    ToolSummary { name: String, duration_secs: f64 },
    TurnComplete { duration_secs: f64 },
}

impl Element {
    pub fn is_thought(&self) -> bool {
        matches!(self, Element::ThoughtMarker { .. })
    }

    /// Number of terminal lines this element renders to.
    /// Must stay in sync with `to_lines()` in `runie-tui/src/ui.rs`.
    pub fn line_count(&self) -> usize {
        match self {
            Element::Spacer => 1,
            Element::UserMessage { content, .. } => content.lines().count().max(1),
            Element::AgentMessage { content, .. } => content.lines().count().max(1),
            Element::Thinking { .. } => 1,
            Element::ThoughtMarker { content } => content.lines().count().max(1),
            Element::ThoughtSummary { .. } => 1,
            Element::ToolRunning { .. } => 1,
            Element::ToolDone { output, .. } => {
                if output.is_empty() { 1 } else { 1 + output.lines().count() }
            }
            Element::ToolSummary { .. } => 1,
            Element::TurnComplete { .. } => 1,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Feed {
    pub elements: Vec<Element>,
}

impl Feed {
    pub fn new() -> Self {
        Self { elements: vec![] }
    }
    
    pub fn push(mut self, element: Element) -> Self {
        self.elements.push(element);
        self
    }
    
    pub fn extend(mut self, elements: Vec<Element>) -> Self {
        self.elements.extend(elements);
        self
    }
    
    pub fn len(&self) -> usize {
        self.elements.len()
    }
    
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }
}
