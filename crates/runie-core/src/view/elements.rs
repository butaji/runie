//! UI Elements - Building blocks for the DSL

#[derive(Debug, Clone)]
pub enum Element {
    Spacer {
        timestamp: f64,
    },
    UserMessage {
        content: String,
        timestamp: f64,
    },
    AgentMessage {
        content: String,
        timestamp: f64,
        provider: String,
    },
    Thinking {
        started: std::time::Instant,
        timestamp: f64,
    },
    ThoughtMarker {
        content: String,
        timestamp: f64,
    },
    ThoughtSummary {
        content: String,
        duration_secs: f64,
        /// Whether the summary hides an expandable body. Duration-only
        /// thoughts have no body, so they render without the `[+]`
        /// affordance (expanding would reveal nothing).
        expandable: bool,
        timestamp: f64,
    },
    ToolRunning {
        name: String,
        args: String,
        started: std::time::Instant,
        timestamp: f64,
    },
    ToolDone {
        name: String,
        args: String,
        duration_secs: f64,
        output: String,
        bytes_transferred: Option<u64>,
        error: bool,
        timestamp: f64,
    },
    ToolSummary {
        name: String,
        duration_secs: f64,
        timestamp: f64,
    },
    ContextGroup {
        tools: Vec<Element>,
        collapsed: bool,
        timestamp: f64,
    },
    /// Swarm pattern worker lifecycle row (GROK.md §26). One row per worker
    /// per turn; `started` is set while Running (drives the braille spinner),
    /// `expanded` renders the worker `output` as the post body.
    SubagentRow {
        id: String,
        description: String,
        model: String,
        status: crate::model::PatternWorkerStatus,
        started: Option<std::time::Instant>,
        duration_ms: Option<u64>,
        output: String,
        expanded: bool,
        timestamp: f64,
    },
    TurnComplete {
        duration_secs: f64,
        timestamp: f64,
    },
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
            Element::ContextGroup { timestamp: ts, .. } => *ts = timestamp,
            Element::SubagentRow { timestamp: ts, .. } => *ts = timestamp,
            Element::TurnComplete { timestamp: ts, .. } => *ts = timestamp,
        }
        e
    }
}

impl Element {
    pub fn user(content: impl Into<String>) -> ElementBuilder {
        ElementBuilder(Element::UserMessage {
            content: content.into(),
            timestamp: 0.0,
        })
    }
    pub fn agent(content: impl Into<String>) -> ElementBuilder {
        ElementBuilder(Element::AgentMessage {
            content: content.into(),
            timestamp: 0.0,
            provider: String::new(),
        })
    }
    pub fn thinking(started: std::time::Instant) -> ElementBuilder {
        ElementBuilder(Element::Thinking {
            started,
            timestamp: 0.0,
        })
    }
    pub fn thought(content: impl Into<String>) -> ElementBuilder {
        ElementBuilder(Element::ThoughtMarker {
            content: content.into(),
            timestamp: 0.0,
        })
    }
    pub fn thought_summary(content: impl Into<String>, duration_secs: f64) -> ElementBuilder {
        ElementBuilder(Element::ThoughtSummary {
            content: content.into(),
            duration_secs,
            expandable: true,
            timestamp: 0.0,
        })
    }
    /// A thought summary with no hidden body — renders without the `[+]`
    /// expand affordance. Used for duration-only thoughts ("◆ Thought for
    /// 2.3s") where there is nothing to expand.
    pub fn thought_summary_static(
        content: impl Into<String>,
        duration_secs: f64,
    ) -> ElementBuilder {
        ElementBuilder(Element::ThoughtSummary {
            content: content.into(),
            duration_secs,
            expandable: false,
            timestamp: 0.0,
        })
    }
    pub fn tool_running(
        name: impl Into<String>,
        args: impl Into<String>,
        started: std::time::Instant,
    ) -> ElementBuilder {
        ElementBuilder(Element::ToolRunning {
            name: name.into(),
            args: args.into(),
            started,
            timestamp: 0.0,
        })
    }
    pub fn tool_done(
        name: impl Into<String>,
        args: impl Into<String>,
        duration_secs: f64,
        output: impl Into<String>,
        bytes_transferred: Option<u64>,
        error: bool,
    ) -> ElementBuilder {
        ElementBuilder(Element::ToolDone {
            name: name.into(),
            args: args.into(),
            duration_secs,
            output: output.into(),
            bytes_transferred,
            error,
            timestamp: 0.0,
        })
    }
    pub fn tool_summary(name: impl Into<String>, duration_secs: f64) -> ElementBuilder {
        ElementBuilder(Element::ToolSummary {
            name: name.into(),
            duration_secs,
            timestamp: 0.0,
        })
    }
    pub fn context_group(tools: Vec<Element>, collapsed: bool) -> ElementBuilder {
        ElementBuilder(Element::ContextGroup {
            tools,
            collapsed,
            timestamp: 0.0,
        })
    }
    /// A swarm worker lifecycle row. `started` is `Some` only while the
    /// worker is Running (spinner animation); `expanded` is set later by the
    /// transform when the post is individually expanded.
    #[allow(clippy::too_many_arguments)]
    pub fn subagent_row(
        id: impl Into<String>,
        description: impl Into<String>,
        model: impl Into<String>,
        status: crate::model::PatternWorkerStatus,
        started: Option<std::time::Instant>,
        duration_ms: Option<u64>,
        output: impl Into<String>,
    ) -> ElementBuilder {
        ElementBuilder(Element::SubagentRow {
            id: id.into(),
            description: description.into(),
            model: model.into(),
            status,
            started,
            duration_ms,
            output: output.into(),
            expanded: false,
            timestamp: 0.0,
        })
    }
    pub fn turn_complete(duration_secs: f64) -> ElementBuilder {
        ElementBuilder(Element::TurnComplete {
            duration_secs,
            timestamp: 0.0,
        })
    }
    pub fn spacer() -> ElementBuilder {
        ElementBuilder(Element::Spacer { timestamp: 0.0 })
    }

    pub fn is_thought(&self) -> bool {
        matches!(self, Element::ThoughtMarker { .. })
    }

    /// Update the timestamp used to order this element in the feed.
    pub fn set_timestamp(&mut self, timestamp: f64) {
        match self {
            Element::Spacer { timestamp: ts } => *ts = timestamp,
            Element::UserMessage { timestamp: ts, .. } => *ts = timestamp,
            Element::AgentMessage { timestamp: ts, .. } => *ts = timestamp,
            Element::Thinking { timestamp: ts, .. } => *ts = timestamp,
            Element::ThoughtMarker { timestamp: ts, .. } => *ts = timestamp,
            Element::ThoughtSummary { timestamp: ts, .. } => *ts = timestamp,
            Element::ToolRunning { timestamp: ts, .. } => *ts = timestamp,
            Element::ToolDone { timestamp: ts, .. } => *ts = timestamp,
            Element::ToolSummary { timestamp: ts, .. } => *ts = timestamp,
            Element::ContextGroup { timestamp: ts, .. } => *ts = timestamp,
            Element::SubagentRow { timestamp: ts, .. } => *ts = timestamp,
            Element::TurnComplete { timestamp: ts, .. } => *ts = timestamp,
        }
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
            Element::ContextGroup { timestamp, .. } => *timestamp,
            Element::SubagentRow { timestamp, .. } => *timestamp,
            Element::TurnComplete { timestamp, .. } => *timestamp,
        }
    }
}

/// The logical kind of a feed post. Used by the app to reason about
/// the feed in user-facing terms instead of raw elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostKind {
    /// System-generated message (e.g. "state cleared").
    System,
    /// User input message.
    UserInput,
    /// Agent/assistant response.
    AgentResponse,
    /// Transient "thinking" indicator while the model is working.
    Thinking,
    /// A thought block produced by the model.
    Thought,
    /// A tool currently being executed.
    ToolRunning,
    /// A completed tool call with output.
    ToolDone,
    /// A collapsed tool result.
    ToolSummary,
    /// A group of context-gathering tools.
    ContextGroup,
    /// A swarm pattern worker lifecycle row.
    SubagentRow,
    /// Turn completion marker.
    TurnComplete,
}

/// A navigable "post" in the feed — a logical unit that the user
/// selects with j/k/arrow keys. A post spans a contiguous range of
/// elements (e.g. a user message plus its following spacer).
#[derive(Debug, Clone)]
pub struct Post {
    pub index: usize,
    /// Inclusive element index where this post starts.
    pub start: usize,
    /// Exclusive element index where this post ends.
    pub end: usize,
    /// Logical kind of this post.
    pub kind: PostKind,
    /// Whether the post body is expanded (true) or collapsed (false).
    /// Collapsed posts render a one-line summary instead of full content.
    pub expanded: bool,
}

impl Post {
    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    pub fn is_empty(&self) -> bool {
        self.start >= self.end
    }
}

#[derive(Debug, Clone, Default)]
pub struct Feed {
    pub elements: Vec<Element>,
    /// Navigable posts in the feed. Each post is a logical unit that
    /// groups one or more consecutive elements.
    pub posts: Vec<Post>,
}

impl Feed {
    pub fn new() -> Self {
        Self {
            elements: vec![],
            posts: vec![],
        }
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

    /// Number of navigable posts in the feed.
    pub fn post_count(&self) -> usize {
        self.posts.len()
    }

    /// Append a built post to the feed. The builder is consumed and the
    /// post's element range is recorded automatically.
    pub fn push_post(&mut self, builder: crate::view::posts::PostBuilder) {
        builder.build(self);
    }

    /// Append a built post and return its index.
    pub fn push_post_and_index(&mut self, builder: crate::view::posts::PostBuilder) -> usize {
        builder.build(self)
    }
}
