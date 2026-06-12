use crate::tui::state::TuiMode;
use crate::tui::view_models::{McpStatus, StatusBarViewModel};
use runie_ai::TokenUsage;

pub(crate) struct StatusBarBuilder {
    mode: TuiMode,
    current_model: Option<String>,
    session_token_usage: TokenUsage,
    status_header: Option<String>,
    status_details: Option<String>,
    status_start_time: Option<std::time::Instant>,
    mcp_status: McpStatus,
    agent_running: bool,
    input_has_text: bool,
}

impl StatusBarBuilder {
    pub(crate) fn new() -> Self {
        Self {
            mode: TuiMode::Chat,
            current_model: None,
            session_token_usage: TokenUsage::default(),
            status_header: None,
            status_details: None,
            status_start_time: None,
            mcp_status: McpStatus::None,
            agent_running: false,
            input_has_text: false,
        }
    }

    pub(crate) fn mode(mut self, mode: TuiMode) -> Self {
        self.mode = mode;
        self
    }

    pub(crate) fn current_model(mut self, model: impl Into<String>) -> Self {
        self.current_model = Some(model.into());
        self
    }

    pub(crate) fn session_token_usage(mut self, usage: TokenUsage) -> Self {
        self.session_token_usage = usage;
        self
    }

    pub(crate) fn status_header(mut self, header: impl Into<String>) -> Self {
        self.status_header = Some(header.into());
        self
    }

    pub(crate) fn status_details(mut self, details: impl Into<String>) -> Self {
        self.status_details = Some(details.into());
        self
    }

    pub(crate) fn status_start_time(mut self, start_time: std::time::Instant) -> Self {
        self.status_start_time = Some(start_time);
        self
    }

    pub(crate) fn mcp_status(mut self, status: McpStatus) -> Self {
        self.mcp_status = status;
        self
    }

    pub(crate) fn agent_running(mut self, running: bool) -> Self {
        self.agent_running = running;
        self
    }

    pub(crate) fn input_has_text(mut self, has_text: bool) -> Self {
        self.input_has_text = has_text;
        self
    }

    pub(crate) fn build(self) -> StatusBarViewModel {
        StatusBarViewModel {
            mode: self.mode,
            current_model: self.current_model,
            session_token_usage: self.session_token_usage,
            status_header: self.status_header,
            status_details: self.status_details,
            status_start_time: self.status_start_time,
            mcp_status: self.mcp_status,
            agent_running: self.agent_running,
            input_has_text: self.input_has_text,
        }
    }
}

impl Default for StatusBarBuilder {
    fn default() -> Self {
        Self::new()
    }
}
