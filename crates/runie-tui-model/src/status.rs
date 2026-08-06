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
    /// Reduce one status intent into the actor-owned immutable projection.
    /// `elapsed_seed` is supplied by the runtime only for deterministic parity
    /// captures; the model remains independent of clocks and terminal I/O.
    pub fn apply(&mut self, message: StatusMsg, elapsed_seed: Option<u64>) {
        match message {
            StatusMsg::Set(state) => self.state = state,
            StatusMsg::BeginTurn => {
                self.elapsed_ticks = elapsed_seed.unwrap_or_default();
                self.turn_usage = None;
                self.turn_stop_reason = None;
            }
            StatusMsg::FinishTurn(usage, stop_reason) => {
                self.turn_usage = Some(usage);
                self.turn_stop_reason = Some(stop_reason);
            }
            StatusMsg::SetTheme(theme) => self.theme = theme,
            StatusMsg::AdvanceAnimation => {
                if matches!(
                    self.state,
                    Status::Loading | Status::Thinking | Status::Streaming | Status::Waiting(_)
                ) {
                    self.animation_frame = self.animation_frame.wrapping_add(1);
                    if elapsed_seed.is_none() {
                        self.elapsed_ticks = self.elapsed_ticks.saturating_add(1);
                    }
                }
            }
        }
    }

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

#[cfg(test)]
mod tests {
    use super::{Status, StatusMsg, StatusSnapshot};
    use runie_core::types::{StopReason, Usage};

    #[test]
    fn reducer_keeps_parity_seed_outside_the_model() {
        let mut state = StatusSnapshot::default();
        state.apply(StatusMsg::BeginTurn, Some(17));
        state.apply(StatusMsg::Set(Status::Thinking), Some(17));
        state.apply(StatusMsg::AdvanceAnimation, Some(17));
        assert_eq!(state.elapsed_ticks, 17);
        assert_eq!(state.animation_frame, 1);

        state.apply(
            StatusMsg::FinishTurn(Usage::default(), StopReason::Stop),
            Some(17),
        );
        assert!(state.turn_usage.is_some());
    }
}
