//! Renderer-independent status state and reducer messages.

use runie_core::types::{StopReason, ThemeKind, Usage, WaitingReason};

const HEADER_TOKEN_BUDGET: u64 = 500_000;

/// Grok's eight-frame braille foreground spinner. Centralized here so the
/// actor-owned animation clock and the renderer share one vocabulary.
pub const BRAILLE_SPINNER_FRAMES: [&str; 8] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"];

/// Legacy `| / - \` fallback for the braille spinner (glyphs.rs:230).
/// Centralized here so the renderer agrees with the model on the
/// quiet terminal fallback shape.
pub const BRAILLE_SPINNER_FALLBACK: [&str; 4] = ["|", "/", "-", "\\"];

/// Pulsing dot progress frames (glyphs.rs:238: `⋅ : ⸬ ⁙`).
/// Centralized here so the activity spinner and the renderer share one
/// glyph vocabulary.
pub const DOT_SPINNER_FRAMES: [&str; 4] = ["⋅", ":", "⸬", "⁙"];

/// Quiet 1-column dot cycle fallback (glyphs.rs: `. : ·`). Centralized
/// here so the renderer agrees with the model on the quiet fallback.
pub const DOT_SPINNER_FALLBACK: [&str; 3] = [".", ":", "·"];

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
    Reset,
    BeginTurn,
    FinishTurn(Usage, StopReason),
    SetTheme(ThemeKind),
    SetContextWindow(Option<u64>),
    SetThinkingElapsed(Option<u64>),
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
    pub context_window: Option<u64>,
    pub thinking_elapsed_ms: Option<u64>,
}

impl StatusSnapshot {
    /// Whether the runtime should schedule another animation tick.
    pub fn animation_demand(&self) -> bool {
        matches!(
            self.state,
            Status::Loading | Status::Thinking | Status::Streaming | Status::Waiting(_)
        )
    }

    pub fn worked_for_label(&self) -> String {
        format_worked_for_seconds(self.elapsed_ticks)
    }
}

/// Format an actor's elapsed-ticks count as Grok's `Worked for N.Ns`
/// label. Centralized here so the actor-owned projection and the
/// renderer share one worked-for shape, including the renderer's
/// override-aware variant.
pub fn format_worked_for_seconds(elapsed_ticks: u64) -> String {
    format!(
        "Worked for {}.{}s",
        elapsed_ticks / 20,
        (elapsed_ticks / 2) % 10
    )
}

impl StatusSnapshot {
    /// Reduce one status intent into the actor-owned immutable projection.
    /// `elapsed_seed` is supplied by the runtime only for deterministic parity
    /// captures; the model remains independent of clocks and terminal I/O.
    pub fn apply(&mut self, message: StatusMsg, elapsed_seed: Option<u64>) {
        match message {
            StatusMsg::Set(state) => self.state = state,
            StatusMsg::Reset => {
                self.state = Status::Ready;
                self.animation_frame = 0;
                self.elapsed_ticks = 0;
                self.turn_usage = None;
                self.turn_stop_reason = None;
                self.thinking_elapsed_ms = None;
            }
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
            StatusMsg::SetContextWindow(window) => self.context_window = window,
            StatusMsg::SetThinkingElapsed(elapsed_ms) => self.thinking_elapsed_ms = elapsed_ms,
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
        let budget = self.context_window.unwrap_or(HEADER_TOKEN_BUDGET);
        format!(
            "{} / {}",
            format_token_count(used),
            format_token_count(budget)
        )
    }
}

fn format_token_count(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        let rendered = format!("{:.1}", tokens as f64 / 1_000_000.0);
        format!("{}M", rendered.trim_end_matches(".0"))
    } else if tokens >= 100_000 {
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

    #[test]
    fn animation_demand_is_a_snapshot_predicate() {
        let mut state = StatusSnapshot::default();
        assert!(!state.animation_demand());
        state.state = Status::Thinking;
        assert!(state.animation_demand());
        state.state = Status::Ready;
        assert!(!state.animation_demand());
    }

    #[test]
    fn reset_clears_terminal_turn_facts_but_preserves_theme_and_context() {
        let mut state = StatusSnapshot {
            theme: runie_core::types::ThemeKind::TerminalNative,
            context_window: Some(42),
            state: Status::Thinking,
            animation_frame: 3,
            elapsed_ticks: 17,
            turn_usage: Some(Usage::default()),
            turn_stop_reason: Some(StopReason::Stop),
            thinking_elapsed_ms: Some(900),
        };
        state.apply(StatusMsg::Reset, None);
        assert_eq!(state.state, Status::Ready);
        assert_eq!(state.theme, runie_core::types::ThemeKind::TerminalNative);
        assert_eq!(state.context_window, Some(42));
        assert_eq!(state.animation_frame, 0);
        assert_eq!(state.elapsed_ticks, 0);
        assert!(state.turn_usage.is_none());
        assert!(state.turn_stop_reason.is_none());
        assert!(state.thinking_elapsed_ms.is_none());
    }

    #[test]
    fn worked_for_label_uses_actor_elapsed_ticks() {
        let state = StatusSnapshot {
            elapsed_ticks: 57,
            ..StatusSnapshot::default()
        };
        assert_eq!(state.worked_for_label(), "Worked for 2.8s");
    }

    #[test]
    fn spinner_frames_pin_grok_source_vocabularies() {
        // Pin the braille foreground spinner: eight frames in the
        // Grok source-backed order.
        assert_eq!(
            super::BRAILLE_SPINNER_FRAMES,
            ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"]
        );
        // Pin the braille fallback: a four-frame ASCII quiet cycle.
        assert_eq!(super::BRAILLE_SPINNER_FALLBACK, ["|", "/", "-", "\\"]);
        // Pin the dot progress frames: the four-glyph pulse cycle.
        assert_eq!(super::DOT_SPINNER_FRAMES, ["⋅", ":", "⸬", "⁙"]);
        // Pin the dot fallback: a three-frame 1-column quiet cycle.
        assert_eq!(super::DOT_SPINNER_FALLBACK, [".", ":", "·"]);
    }

    #[test]
    fn format_worked_for_seconds_pins_grok_label_form() {
        // Pin the smoke path: 57 elapsed ticks at 20 Hz renders as
        // "Worked for 2.8s" so the renderer agrees with the model.
        assert_eq!(super::format_worked_for_seconds(57), "Worked for 2.8s");
        // Pin the zero-tick case: a fresh turn shows "Worked for 0.0s".
        assert_eq!(super::format_worked_for_seconds(0), "Worked for 0.0s");
        // Pin the larger-tick case: 20 ticks is one full second.
        assert_eq!(super::format_worked_for_seconds(20), "Worked for 1.0s");
    }

    #[test]
    fn context_window_is_actor_owned_and_changes_the_meter() {
        let mut state = StatusSnapshot::default();
        state.apply(StatusMsg::SetContextWindow(Some(1_000_000)), None);
        assert_eq!(state.header_meter(), "0 / 1M");
    }
}
