//! UI Elements - Building blocks for the DSL

#[derive(Debug, Clone)]
pub enum Element {
    Spacer,
    UserMessage { content: String },
    AgentMessage { content: String },
    Thinking { elapsed: f64 },
    ThoughtMarker { content: String },
    ToolRunning { name: String, elapsed: f64 },
    ToolDone { name: String, duration_secs: f64 },
    TurnComplete { duration_secs: f64 },
    Group { id: String, elements: Vec<Element> },
}

impl Element {
    pub fn id(&self) -> Option<&str> {
        match self {
            Element::Group { id, .. } => Some(id),
            _ => None,
        }
    }
    
    pub fn is_thought(&self) -> bool {
        matches!(self, Element::ThoughtMarker { .. })
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
