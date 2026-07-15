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
    /// Explicit Anthropic-style thinking block with type and signature
    AnthropicThinking {
        /// The raw thinking content
        content: String,
        /// Signature for verification (base64 encoded)
        signature: Option<String>,
        /// Whether this thinking is encrypted/redacted
        redacted: bool,
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
    /// Tool call requiring user confirmation before execution
    ToolConfirmation {
        request_id: String,
        name: String,
        args: String,
        /// Human-readable description of the tool action
        description: String,
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
        activity: String,
        output: String,
        expanded: bool,
        timestamp: f64,
    },
    TurnComplete {
        duration_secs: f64,
        timestamp: f64,
    },
    /// Inline image rendered via terminal protocols (iTerm2/Kitty)
    Image {
        /// Base64 encoded image data
        data: String,
        /// MIME type (e.g., "image/png", "image/jpeg")
        mime_type: String,
        /// Width in cells (for aspect ratio calculation)
        width_cells: Option<u16>,
        /// Height in cells (optional, calculated from aspect if not provided)
        height_cells: Option<u16>,
        /// Terminal protocol to use
        protocol: ImageProtocol,
        timestamp: f64,
    },
    /// Structured data part (JSON, DataPart from A2A)
    DataPart {
        /// The structured data as JSON string
        data: String,
        /// Optional human-readable format string
        format_string: Option<String>,
        timestamp: f64,
    },
    /// Markdown table rendered with alignment
    MarkdownTable {
        /// Table header row
        headers: Vec<String>,
        /// Table data rows
        rows: Vec<Vec<String>>,
        /// Column alignments (None = left, Some(true) = right, Some(false) = center)
        alignments: Vec<Option<bool>>,
        timestamp: f64,
    },
    /// Diff/changelist output from tools
    DiffOutput {
        /// The diff content (unified format)
        content: String,
        /// Type of diff
        diff_type: DiffType,
        timestamp: f64,
    },
    /// Web search tool invocation
    WebSearchCall {
        query: String,
        results: Vec<WebSearchResult>,
        timestamp: f64,
    },
    /// ANSI escape sequence styled content
    AnsiStyled {
        /// Raw content with ANSI escape sequences
        raw_content: String,
        /// Stripped plain text for fallback/calculations
        plain_text: String,
        timestamp: f64,
    },
}

/// Terminal image rendering protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageProtocol {
    /// iTerm2 inline image protocol: \033]1337;File=inline=1:...
    ITerm2,
    /// Kitty graphics protocol: \033_G...;...\033\\
    Kitty,
    /// Sixel graphics protocol (legacy terminals)
    Sixel,
}

impl Default for ImageProtocol {
    fn default() -> Self {
        ImageProtocol::ITerm2
    }
}

/// Type of diff output
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffType {
    /// Unified diff format
    Unified,
    /// Side-by-side diff
    SideBySide,
    /// Context diff
    Context,
}

/// A single web search result
#[derive(Debug, Clone, PartialEq)]
pub struct WebSearchResult {
    /// Title of the result
    pub title: String,
    /// URL of the result
    pub url: String,
    /// Snippet/description
    pub snippet: String,
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
            Element::AnthropicThinking { timestamp: ts, .. } => *ts = timestamp,
            Element::ToolRunning { timestamp: ts, .. } => *ts = timestamp,
            Element::ToolDone { timestamp: ts, .. } => *ts = timestamp,
            Element::ToolSummary { timestamp: ts, .. } => *ts = timestamp,
            Element::ToolConfirmation { timestamp: ts, .. } => *ts = timestamp,
            Element::ContextGroup { timestamp: ts, .. } => *ts = timestamp,
            Element::SubagentRow { timestamp: ts, .. } => *ts = timestamp,
            Element::TurnComplete { timestamp: ts, .. } => *ts = timestamp,
            Element::Image { timestamp: ts, .. } => *ts = timestamp,
            Element::DataPart { timestamp: ts, .. } => *ts = timestamp,
            Element::MarkdownTable { timestamp: ts, .. } => *ts = timestamp,
            Element::DiffOutput { timestamp: ts, .. } => *ts = timestamp,
            Element::WebSearchCall { timestamp: ts, .. } => *ts = timestamp,
            Element::AnsiStyled { timestamp: ts, .. } => *ts = timestamp,
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
    /// Anthropic-style thinking block with optional signature
    pub fn anthropic_thinking(content: impl Into<String>, signature: Option<String>) -> ElementBuilder {
        ElementBuilder(Element::AnthropicThinking {
            content: content.into(),
            signature,
            redacted: false,
            timestamp: 0.0,
        })
    }
    /// Redacted/encrypted thinking content
    pub fn redacted_thinking(encrypted_data: impl Into<String>) -> ElementBuilder {
        ElementBuilder(Element::AnthropicThinking {
            content: encrypted_data.into(),
            signature: None,
            redacted: true,
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
    /// Tool call requiring user confirmation
    pub fn tool_confirmation(
        request_id: impl Into<String>,
        name: impl Into<String>,
        args: impl Into<String>,
        description: impl Into<String>,
    ) -> ElementBuilder {
        ElementBuilder(Element::ToolConfirmation {
            request_id: request_id.into(),
            name: name.into(),
            args: args.into(),
            description: description.into(),
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
        activity: impl Into<String>,
        output: impl Into<String>,
    ) -> ElementBuilder {
        ElementBuilder(Element::SubagentRow {
            id: id.into(),
            description: description.into(),
            model: model.into(),
            status,
            started,
            duration_ms,
            activity: activity.into(),
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
    /// Inline image element
    pub fn image(
        data: impl Into<String>,
        mime_type: impl Into<String>,
    ) -> ElementBuilder {
        ElementBuilder(Element::Image {
            data: data.into(),
            mime_type: mime_type.into(),
            width_cells: None,
            height_cells: None,
            protocol: ImageProtocol::default(),
            timestamp: 0.0,
        })
    }
    /// Structured data part
    pub fn data_part(data: impl Into<String>, format_string: Option<String>) -> ElementBuilder {
        ElementBuilder(Element::DataPart {
            data: data.into(),
            format_string,
            timestamp: 0.0,
        })
    }
    /// Markdown table
    pub fn markdown_table(
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
        alignments: Vec<Option<bool>>,
    ) -> ElementBuilder {
        ElementBuilder(Element::MarkdownTable {
            headers,
            rows,
            alignments,
            timestamp: 0.0,
        })
    }
    /// Diff output
    pub fn diff_output(content: impl Into<String>, diff_type: DiffType) -> ElementBuilder {
        ElementBuilder(Element::DiffOutput {
            content: content.into(),
            diff_type,
            timestamp: 0.0,
        })
    }
    /// Web search call with results
    pub fn web_search_call(query: impl Into<String>, results: Vec<WebSearchResult>) -> ElementBuilder {
        ElementBuilder(Element::WebSearchCall {
            query: query.into(),
            results,
            timestamp: 0.0,
        })
    }
    /// ANSI styled content
    pub fn ansi_styled(raw: impl Into<String>, plain: impl Into<String>) -> ElementBuilder {
        ElementBuilder(Element::AnsiStyled {
            raw_content: raw.into(),
            plain_text: plain.into(),
            timestamp: 0.0,
        })
    }

    pub fn is_thought(&self) -> bool {
        matches!(self, Element::ThoughtMarker { .. } | Element::AnthropicThinking { .. })
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
            Element::AnthropicThinking { timestamp: ts, .. } => *ts = timestamp,
            Element::ToolRunning { timestamp: ts, .. } => *ts = timestamp,
            Element::ToolDone { timestamp: ts, .. } => *ts = timestamp,
            Element::ToolSummary { timestamp: ts, .. } => *ts = timestamp,
            Element::ToolConfirmation { timestamp: ts, .. } => *ts = timestamp,
            Element::ContextGroup { timestamp: ts, .. } => *ts = timestamp,
            Element::SubagentRow { timestamp: ts, .. } => *ts = timestamp,
            Element::TurnComplete { timestamp: ts, .. } => *ts = timestamp,
            Element::Image { timestamp: ts, .. } => *ts = timestamp,
            Element::DataPart { timestamp: ts, .. } => *ts = timestamp,
            Element::MarkdownTable { timestamp: ts, .. } => *ts = timestamp,
            Element::DiffOutput { timestamp: ts, .. } => *ts = timestamp,
            Element::WebSearchCall { timestamp: ts, .. } => *ts = timestamp,
            Element::AnsiStyled { timestamp: ts, .. } => *ts = timestamp,
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
            Element::AnthropicThinking { timestamp, .. } => *timestamp,
            Element::ToolRunning { timestamp, .. } => *timestamp,
            Element::ToolDone { timestamp, .. } => *timestamp,
            Element::ToolSummary { timestamp, .. } => *timestamp,
            Element::ToolConfirmation { timestamp, .. } => *timestamp,
            Element::ContextGroup { timestamp, .. } => *timestamp,
            Element::SubagentRow { timestamp, .. } => *timestamp,
            Element::TurnComplete { timestamp, .. } => *timestamp,
            Element::Image { timestamp, .. } => *timestamp,
            Element::DataPart { timestamp, .. } => *timestamp,
            Element::MarkdownTable { timestamp, .. } => *timestamp,
            Element::DiffOutput { timestamp, .. } => *timestamp,
            Element::WebSearchCall { timestamp, .. } => *timestamp,
            Element::AnsiStyled { timestamp, .. } => *timestamp,
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
