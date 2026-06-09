//! UI Elements - Building blocks for the DSL

#[derive(Debug, Clone)]
pub enum Element {
    Spacer { timestamp: f64 },
    UserMessage { content: String, timestamp: f64 },
    AgentMessage { content: String, timestamp: f64, provider: String },
    Thinking { started: std::time::Instant, timestamp: f64 },
    ThoughtMarker { content: String, timestamp: f64 },
    ThoughtSummary { content: String, duration_secs: f64, timestamp: f64 },
    ToolRunning { name: String, started: std::time::Instant, timestamp: f64 },
    ToolDone { name: String, duration_secs: f64, output: String, timestamp: f64 },
    ToolSummary { name: String, duration_secs: f64, timestamp: f64 },
    TurnComplete { duration_secs: f64, timestamp: f64 },
}

/// Builder for attaching a timestamp to an Element.
pub struct ElementBuilder(pub(crate) Element);

impl ElementBuilder {
    pub fn at(self, timestamp: f64) -> Element {
        let mut e = self.0;
        match &mut e {
            Element::Spacer { timestamp: ts } => *ts = timestamp,
            Element::UserMessage { timestamp: ts, .. } => *ts = timestamp,
            Element::AgentMessage { timestamp: ts, .. } => *ts = timestamp,
            Element::Thinking { timestamp: ts, .. } => *ts = timestamp,
            Element::ThoughtMarker { timestamp: ts, .. } => *ts = timestamp,
            Element::ThoughtSummary { timestamp: ts, .. } => *ts = timestamp,
            Element::ToolRunning { timestamp: ts, .. } => *ts = timestamp,
            Element::ToolDone { timestamp: ts, .. } => *ts = timestamp,
            Element::ToolSummary { timestamp: ts, .. } => *ts = timestamp,
            Element::TurnComplete { timestamp: ts, .. } => *ts = timestamp,
        }
        e
    }
}

impl Element {
    pub fn user(content: impl Into<String>) -> ElementBuilder {
        ElementBuilder(Element::UserMessage { content: content.into(), timestamp: 0.0 })
    }
    pub fn agent(content: impl Into<String>) -> ElementBuilder {
        ElementBuilder(Element::AgentMessage { content: content.into(), timestamp: 0.0, provider: String::new() })
    }
    pub fn thinking(started: std::time::Instant) -> ElementBuilder {
        ElementBuilder(Element::Thinking { started, timestamp: 0.0 })
    }
    pub fn thought(content: impl Into<String>) -> ElementBuilder {
        ElementBuilder(Element::ThoughtMarker { content: content.into(), timestamp: 0.0 })
    }
    pub fn thought_summary(content: impl Into<String>, duration_secs: f64) -> ElementBuilder {
        ElementBuilder(Element::ThoughtSummary { content: content.into(), duration_secs, timestamp: 0.0 })
    }
    pub fn tool_running(name: impl Into<String>, started: std::time::Instant) -> ElementBuilder {
        ElementBuilder(Element::ToolRunning { name: name.into(), started, timestamp: 0.0 })
    }
    pub fn tool_done(name: impl Into<String>, duration_secs: f64, output: impl Into<String>) -> ElementBuilder {
        ElementBuilder(Element::ToolDone { name: name.into(), duration_secs, output: output.into(), timestamp: 0.0 })
    }
    pub fn tool_summary(name: impl Into<String>, duration_secs: f64) -> ElementBuilder {
        ElementBuilder(Element::ToolSummary { name: name.into(), duration_secs, timestamp: 0.0 })
    }
    pub fn turn_complete(duration_secs: f64) -> ElementBuilder {
        ElementBuilder(Element::TurnComplete { duration_secs, timestamp: 0.0 })
    }
    pub fn spacer() -> ElementBuilder {
        ElementBuilder(Element::Spacer { timestamp: 0.0 })
    }


    pub fn is_thought(&self) -> bool {
        matches!(self, Element::ThoughtMarker { .. })
    }

    /// Sort key for ordering elements in the feed.
    pub fn timestamp(&self) -> f64 {
        match self {
            Element::Spacer { timestamp } => *timestamp,
            Element::UserMessage { timestamp, .. } => *timestamp,
            Element::AgentMessage { timestamp, .. } => *timestamp,
            Element::Thinking { timestamp, .. } => *timestamp,
            Element::ThoughtMarker { timestamp, .. } => *timestamp,
            Element::ThoughtSummary { timestamp, .. } => *timestamp,
            Element::ToolRunning { timestamp, .. } => *timestamp,
            Element::ToolDone { timestamp, .. } => *timestamp,
            Element::ToolSummary { timestamp, .. } => *timestamp,
            Element::TurnComplete { timestamp, .. } => *timestamp,
        }
    }

    /// Number of terminal lines this element renders to.
    /// Must stay in sync with `to_lines()` in `runie-tui/src/ui.rs`.
    pub fn line_count(&self) -> usize {
        match self {
            Element::Spacer { .. } => 1,
            Element::UserMessage { content, .. } => content.lines().count().max(1),
            Element::AgentMessage { content, .. } => content.lines().count().max(1),
            Element::Thinking { .. } => 1,
            Element::ThoughtMarker { content, .. } => content.lines().count().max(1),
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
