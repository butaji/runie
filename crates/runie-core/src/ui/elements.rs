//! UI Elements - Building blocks for the DSL
//! 
//! These are pure data structures representing UI components.

/// A UI element that can be rendered
#[derive(Debug, Clone)]
pub enum Element {
    /// Empty spacer line
    Spacer,
    /// User message: "You: <content>"
    UserMessage { content: String },
    /// Agent response: "Agent: <content>"
    AgentMessage { content: String },
    /// Thinking indicator: "⠋ Thinking... Xs" (animated)
    Thinking { elapsed: f64 },
    /// Thought marker: "◆ Thought Xs" (stored duration)
    ThoughtMarker { content: String },
    /// Group of elements
    Group { id: String, elements: Vec<Element> },
}

impl Element {
    /// Get the ID of this element (for correlation)
    pub fn id(&self) -> Option<&str> {
        match self {
            Element::Group { id, .. } => Some(id),
            _ => None,
        }
    }
    
    /// Check if this element is a thought marker
    pub fn is_thought(&self) -> bool {
        matches!(self, Element::ThoughtMarker { .. })
    }
}

/// A list of elements ready for rendering
#[derive(Debug, Clone, Default)]
pub struct Feed {
    pub elements: Vec<Element>,
}

impl Feed {
    /// Create empty feed
    pub fn new() -> Self {
        Self { elements: vec![] }
    }
    
    /// Add element to feed
    pub fn push(mut self, element: Element) -> Self {
        self.elements.push(element);
        self
    }
    
    /// Add multiple elements
    pub fn extend(mut self, elements: Vec<Element>) -> Self {
        self.elements.extend(elements);
        self
    }
    
    /// Get number of elements
    pub fn len(&self) -> usize {
        self.elements.len()
    }
    
    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }
}
