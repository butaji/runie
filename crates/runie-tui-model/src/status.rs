//! Renderer-independent status state and reducer messages.

use runie_core::types::{StopReason, ThemeKind, Usage, WaitingReason};

const HEADER_TOKEN_BUDGET: u64 = 500_000;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Status {
    #[default]
    Ready,
    Loading,
    Thinking,
    Streaming,
    Waiting(WaitingReason),
    Aborted,
    Error(String),
}

impl Status {
    pub fn label(&self) -> String {
        match self {
            Self::Ready => "ready".into(),
            Self::Loading => "loading".into(),
            Self::Thinking => "thinking...".into(),
            Self::Streaming => "streaming".into(),
            Self::Waiting(reason) => format!("waiting: {}", reason.label()),
            Self::Aborted => "aborted".into(),
            Self::Error(error) => format!("error: {error}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusMsg {
    Set(Status),
    BeginTurn,
    FinishTurn(Usage, StopReason),
    SetTheme(ThemeKind),
    AdvanceAnimation,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StatusSnapshot {
    pub state: Status,
    pub theme: ThemeKind,
    pub animation_frame: usize,
    pub elapsed_ticks: u64,
    pub turn_usage: Option<Usage>,
    pub turn_stop_reason: Option<StopReason>,
}

impl StatusSnapshot {
    /// Pure event-derived context meter for the declarative header props.
    pub fn header_meter(&self) -> String {
        let used = self
            .turn_usage
            .as_ref()
            .map(|usage| usage.total_tokens)
            .unwrap_or_default();
        format!(
            "{} / {}K",
            format_token_count(used),
            HEADER_TOKEN_BUDGET / 1_000
        )
    }
}

fn format_token_count(tokens: u64) -> String {
    if tokens >= 100_000 {
        format!("{}K", tokens / 1_000)
    } else if tokens >= 1_000 {
        let rendered = format!("{:.1}", tokens as f64 / 1_000.0);
        format!("{}K", rendered.trim_end_matches(".0"))
    } else {
        tokens.to_string()
    }
}
