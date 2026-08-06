//! Renderer-independent transcript line vocabulary.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    User,
    Assistant,
    Reasoning,
    ThinkingStatus,
    Tool,
    ToolRunning,
    ToolError,
    ToolResult,
    ToolOutput,
    SessionStart,
    System,
    Separator,
    TurnSummary,
    CompletedAssistant,
    Activity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub kind: LineKind,
    pub text: String,
    pub tool_call_id: Option<String>,
    has_vpad: bool,
}

impl Line {
    pub fn new(kind: LineKind, text: impl Into<String>) -> Self {
        Self {
            kind,
            text: text.into(),
            tool_call_id: None,
            has_vpad: false,
        }
    }

    pub fn with_vpad(mut self, has_vpad: bool) -> Self {
        self.has_vpad = has_vpad;
        self
    }

    pub fn has_vpad(&self) -> bool {
        self.has_vpad
    }

    pub fn for_tool(mut self, tool_call_id: impl Into<String>) -> Self {
        self.tool_call_id = Some(tool_call_id.into());
        self
    }
}
