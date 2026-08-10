//! 1-row status bar.

use crate::appearance;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Paragraph, Widget};
use runie_core::types::{StopReason, ThemeKind, Usage};
pub use runie_tui_model::{Status, StatusMsg, StatusSnapshot};

pub trait StatusStyleExt {
    fn style(&self) -> Style;
    fn style_for(&self, theme: ThemeKind) -> Style;
}

impl StatusStyleExt for Status {
    fn style(&self) -> Style {
        self.style_for(ThemeKind::GrokNight)
    }

    fn style_for(&self, theme: ThemeKind) -> Style {
        match self {
            Self::Ready => appearance::success_style_for(theme),
            Self::Loading => appearance::muted_style_for(theme),
            Self::Thinking => appearance::accent_style_for(theme),
            Self::Streaming => appearance::secondary_style_for(theme),
            Self::Waiting(_) => appearance::warning_style_for(theme),
            Self::Aborted => appearance::muted_style_for(theme),
            Self::Error(_) => appearance::error_style_for(theme),
        }
    }
}

/// Braille spinner frames matching grok's `braille_spinner_frames`
/// (xai-grok-pager-render/src/glyphs.rs:225) — FANCY set.
pub fn braille_spinner_frames() -> &'static [&'static str] {
    &runie_tui_model::BRAILLE_SPINNER_FRAMES
}

/// Legacy `| / - \` fallback for braille (glyphs.rs:230).
pub fn braille_spinner_fallback() -> &'static [&'static str] {
    &runie_tui_model::BRAILLE_SPINNER_FALLBACK
}

/// Pulsing dot progress frames (glyphs.rs:238: `⋅ : ⸬ ⁙`).
pub fn dot_spinner_frames() -> &'static [&'static str] {
    &runie_tui_model::DOT_SPINNER_FRAMES
}

/// Quiet 1-column dot cycle fallback (glyphs.rs: `. : ·`).
pub fn dot_spinner_fallback() -> &'static [&'static str] {
    &runie_tui_model::DOT_SPINNER_FALLBACK
}

#[derive(Debug, Clone, Default)]
pub struct StatusBar {
    state: Status,
    theme: ThemeKind,
    animation_frame: usize,
    elapsed_ticks: u64,
    elapsed_ticks_override: Option<u64>,
    turn_usage: Option<Usage>,
    turn_stop_reason: Option<StopReason>,
    context_window: Option<u64>,
    thinking_elapsed_ms: Option<u64>,
}

/// Grok's one-row foreground activity indicator above the prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnStatus {
    frame: usize,
    phase: TurnStatusPhase,
    chrome: String,
    theme: ThemeKind,
    waiting_label: String,
}

pub use runie_tui_model::TurnStatusPhase;

impl TurnStatus {
    /// Runie advances its actor-owned animation clock at 20 Hz. Grok holds
    /// each braille frame for four 30 Hz ticks (~133 ms), so three Runie
    /// ticks is the closest stable equivalent (~150 ms).
    const SPINNER_DIVISOR: usize = 3;
    const FRAMES: [&'static str; 8] = runie_tui_model::BRAILLE_SPINNER_FRAMES;

    pub fn new(frame: usize) -> Self {
        Self {
            frame,
            phase: TurnStatusPhase::Starting,
            chrome: String::new(),
            theme: ThemeKind::GrokNight,
            waiting_label: "Waiting for response…".to_owned(),
        }
    }

    pub fn phase(mut self, phase: TurnStatusPhase) -> Self {
        self.phase = phase;
        self
    }

    pub fn with_chrome(mut self, chrome: impl Into<String>) -> Self {
        self.chrome = chrome.into();
        self
    }

    pub fn with_theme(mut self, theme: ThemeKind) -> Self {
        self.theme = theme;
        self
    }

    pub fn with_waiting_label(mut self, label: impl Into<String>) -> Self {
        self.waiting_label = label.into();
        self
    }

    pub fn text(&self) -> String {
        runie_tui_model::turn_status_text(self.phase, self.frame, &self.waiting_label, &self.chrome)
    }

    pub fn render(self, area: Rect, buf: &mut Buffer) {
        if self.phase == TurnStatusPhase::Thinking {
            Paragraph::new(Line::from(vec![ratatui::text::Span::styled(
                "┃  ◆ Thinking…",
                appearance::accent_style_for(self.theme).add_modifier(Modifier::BOLD),
            )]))
            .render(area, buf);
            return;
        }
        let label = match self.phase {
            TurnStatusPhase::Starting => "Starting session… 0.0s",
            TurnStatusPhase::Waiting => self.waiting_label.as_str(),
            TurnStatusPhase::Thinking => "Thinking…",
            TurnStatusPhase::Responding => "Responding…",
        };
        let spinner = Self::FRAMES[(self.frame / Self::SPINNER_DIVISOR) % Self::FRAMES.len()];
        let waiting = self.phase == TurnStatusPhase::Waiting;
        let spinner_style = if waiting {
            appearance::base_style_for(self.theme)
        } else {
            appearance::accent_style_for(self.theme)
        };
        let text_style = if waiting {
            appearance::base_style_for(self.theme)
        } else {
            appearance::muted_style_for(self.theme)
        };
        let mut line = Line::from(vec![
            ratatui::text::Span::raw("  "),
            ratatui::text::Span::styled(spinner, spinner_style),
            ratatui::text::Span::raw(" "),
            ratatui::text::Span::styled(label, text_style),
        ]);
        if !self.chrome.is_empty() {
            line.spans
                .push(ratatui::text::Span::styled(self.chrome, text_style));
        }
        Paragraph::new(line).render(area, buf);
    }
}

impl StatusBar {
    pub fn new() -> Self {
        Self {
            state: Status::default(),
            theme: ThemeKind::GrokNight,
            animation_frame: 0,
            elapsed_ticks: 0,
            elapsed_ticks_override: crate::clock::parity_elapsed_ticks(),
            turn_usage: None,
            turn_stop_reason: None,
            context_window: None,
            thinking_elapsed_ms: None,
        }
    }

    /// Renderer-local adapter from the actor-owned immutable projection.
    /// Rendering may use the widget implementation, but it must not read a
    /// second mutable/actor source while painting the same frame.
    pub fn from_model_snapshot(snapshot: StatusSnapshot) -> Self {
        Self {
            state: snapshot.state,
            theme: snapshot.theme,
            animation_frame: snapshot.animation_frame,
            elapsed_ticks: snapshot.elapsed_ticks,
            elapsed_ticks_override: None,
            turn_usage: snapshot.turn_usage,
            turn_stop_reason: snapshot.turn_stop_reason,
            context_window: snapshot.context_window,
            thinking_elapsed_ms: snapshot.thinking_elapsed_ms,
        }
    }

    pub fn set(&mut self, s: Status) {
        self.apply(StatusMsg::Set(s));
    }

    pub fn begin_turn(&mut self) {
        self.apply(StatusMsg::BeginTurn);
    }

    pub fn finish_turn(&mut self, usage: Usage, stop_reason: StopReason) {
        self.apply(StatusMsg::FinishTurn(usage, stop_reason));
    }

    pub fn set_theme(&mut self, theme: ThemeKind) {
        self.apply(StatusMsg::SetTheme(theme));
    }

    pub fn advance_animation(&mut self) {
        self.apply(StatusMsg::AdvanceAnimation);
    }

    /// Apply one status message. This is deliberately the only transition
    /// entry point used by the imperative compatibility methods above.
    pub fn apply(&mut self, message: StatusMsg) {
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
                self.elapsed_ticks = self.elapsed_ticks_override.unwrap_or_default();
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
                    if self.elapsed_ticks_override.is_none() {
                        self.elapsed_ticks = self.elapsed_ticks.saturating_add(1);
                    }
                }
            }
        }
    }

    pub fn worked_for_label(&self) -> String {
        runie_tui_model::format_worked_for_seconds(self.displayed_elapsed_ticks())
    }

    fn displayed_elapsed_ticks(&self) -> u64 {
        self.elapsed_ticks_override.unwrap_or(self.elapsed_ticks)
    }

    /// Event-derived turn-token meter for the live header. Keeping this on the
    /// status projection avoids a second mutable source of truth in the
    /// binary's render loop.
    pub fn header_meter(&self) -> String {
        let used = self
            .turn_usage
            .as_ref()
            .map(|usage| usage.total_tokens)
            .unwrap_or_default();
        let budget = self.context_window.unwrap_or(500_000);
        format!(
            "{} turn / {}",
            format_token_count(used).replace('k', "K"),
            format_token_count(budget).replace('k', "K")
        )
    }

    /// Build the pure foreground status projection consumed by the TUI view.
    pub fn turn_status(&self) -> Option<TurnStatus> {
        let phase = match self.state {
            Status::Thinking => TurnStatusPhase::Thinking,
            Status::Streaming => TurnStatusPhase::Responding,
            Status::Waiting(_) => TurnStatusPhase::Waiting,
            _ => return None,
        };
        let chrome = match (&self.turn_usage, &self.turn_stop_reason) {
            (Some(usage), Some(reason)) => format!(
                "  {}.{}s ⇣{} [{}]",
                self.displayed_elapsed_ticks() / 20,
                (self.displayed_elapsed_ticks() / 2) % 10,
                format_token_count(usage.total_tokens),
                stop_reason_label(*reason)
            ),
            _ => format!(
                "  {}.{}s ⇣0 [stop]",
                self.displayed_elapsed_ticks() / 20,
                (self.displayed_elapsed_ticks() / 2) % 10
            ),
        };
        let waiting_label = match &self.state {
            Status::Waiting(reason) => reason.label(),
            _ => "Waiting for response…".to_owned(),
        };
        Some(
            TurnStatus::new(self.animation_frame)
                .phase(phase)
                .with_chrome(chrome)
                .with_theme(self.theme())
                .with_waiting_label(waiting_label),
        )
    }

    pub fn current(&self) -> &Status {
        &self.state
    }

    pub fn model_snapshot(&self) -> StatusSnapshot {
        StatusSnapshot {
            state: self.state.clone(),
            theme: self.theme,
            animation_frame: self.animation_frame,
            elapsed_ticks: self.displayed_elapsed_ticks(),
            turn_usage: self.turn_usage.clone(),
            turn_stop_reason: self.turn_stop_reason,
            context_window: self.context_window,
            thinking_elapsed_ms: self.thinking_elapsed_ms,
        }
    }

    pub fn theme(&self) -> ThemeKind {
        self.theme
    }

    /// Advance the deterministic spinner used by active full-mode states.
    /// The caller owns the cadence; tests can select exact frames without
    /// depending on wall-clock timing.
    /// Whether the renderer should schedule another animation wake-up.
    /// Idle and terminal states do not create timer work.
    pub fn animation_demand(&self) -> bool {
        matches!(
            self.state,
            Status::Loading | Status::Thinking | Status::Streaming | Status::Waiting(_)
        )
    }

    pub fn set_animation_frame(&mut self, frame: usize) {
        self.animation_frame = frame;
    }

    pub fn animation_frame(&self) -> usize {
        self.animation_frame
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        crate::paint::render_paint_document(
            &crate::paint::status_footer_paint(&self.model_snapshot()),
            self.theme,
            area,
            buf,
        );
    }
}

fn stop_reason_label(reason: StopReason) -> &'static str {
    reason.display_name()
}

fn format_token_count(tokens: u64) -> String {
    if tokens < 1_000 {
        return tokens.to_string();
    }
    if tokens < 1_000_000 {
        return format_trimmed_decimal(tokens as f64 / 1_000.0, 'k');
    }
    format_trimmed_decimal(tokens as f64 / 1_000_000.0, 'M')
}

fn format_trimmed_decimal(value: f64, suffix: char) -> String {
    let rendered = format!("{value:.2}")
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_owned();
    format!("{rendered}{suffix}")
}

#[cfg(test)]
#[path = "status_tests.rs"]
mod tests;
