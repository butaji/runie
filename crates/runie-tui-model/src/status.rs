//! Renderer-independent status state and reducer messages.

use runie_core::types::{StopReason, ThemeKind, Usage, WaitingReason};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusSnapshot {
    pub state: Status,
    pub theme: ThemeKind,
    pub animation_frame: usize,
    pub elapsed_ticks: u64,
    pub turn_usage: Option<Usage>,
    pub turn_stop_reason: Option<StopReason>,
}
