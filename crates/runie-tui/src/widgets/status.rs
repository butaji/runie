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
    &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"]
}

/// Legacy `| / - \` fallback for braille (glyphs.rs:230).
pub fn braille_spinner_fallback() -> &'static [&'static str] {
    &["|", "/", "-", "\\"]
}

/// Pulsing dot progress frames (glyphs.rs:238: `⋅ : ⸬ ⁙`).
pub fn dot_spinner_frames() -> &'static [&'static str] {
    &["⋅", ":", "⸬", "⁙"]
}

/// Quiet 1-column dot cycle fallback (glyphs.rs: `. : ·`).
pub fn dot_spinner_fallback() -> &'static [&'static str] {
    &[".", ":", "·"]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnStatusPhase {
    Starting,
    Waiting,
    Thinking,
    Responding,
}

impl TurnStatus {
    /// Runie advances its actor-owned animation clock at 20 Hz. Grok holds
    /// each braille frame for four 30 Hz ticks (~133 ms), so three Runie
    /// ticks is the closest stable equivalent (~150 ms).
    const SPINNER_DIVISOR: usize = 3;
    const FRAMES: [&'static str; 8] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"];

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
        if self.phase == TurnStatusPhase::Thinking {
            return "┃  ◆ Thinking…".to_owned();
        }
        let label = match self.phase {
            TurnStatusPhase::Starting => "Starting session… 0.0s",
            // The recorded full-mode waiting row includes the right-aligned
            // elapsed/usage/stop chrome on the same terminal row.
            TurnStatusPhase::Waiting => self.waiting_label.as_str(),
            TurnStatusPhase::Thinking => "Thinking…",
            TurnStatusPhase::Responding => "Responding…",
        };
        format!(
            "  {} {label}{}",
            Self::FRAMES[(self.frame / Self::SPINNER_DIVISOR) % Self::FRAMES.len()],
            self.chrome
        )
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
        format!(
            "Worked for {}.{}s",
            self.displayed_elapsed_ticks() / 20,
            (self.displayed_elapsed_ticks() / 2) % 10
        )
    }

    fn displayed_elapsed_ticks(&self) -> u64 {
        self.elapsed_ticks_override.unwrap_or(self.elapsed_ticks)
    }

    /// Event-derived context meter for the live header. Keeping this on the
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
            "{} / {}",
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
        Widget::render(Paragraph::new(self.footer_line()), area, buf);
    }

    fn footer_line(&self) -> Line<'static> {
        use ratatui::text::Span;
        let spans = match self.state {
            Status::Ready => ready_footer_spans(self.theme()),
            Status::Loading => loading_footer_spans(self.animation_frame, self.theme()),
            Status::Thinking | Status::Streaming | Status::Waiting(_) => {
                active_footer_spans(self.theme())
            }
            _ => vec![Span::styled(
                self.state.label(),
                self.state.style_for(self.theme()),
            )],
        };
        Line::from(spans).style(appearance::muted_style_for(self.theme))
    }
}

fn stop_reason_label(reason: StopReason) -> &'static str {
    match reason {
        StopReason::Stop => "stop",
        StopReason::ToolUse => "toolUse",
        StopReason::MaxTokens => "length",
        StopReason::Error => "error",
        StopReason::Aborted => "aborted",
        StopReason::Pending => "pending",
        StopReason::Deferred => "deferred",
    }
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

fn ready_footer_spans(theme: ThemeKind) -> Vec<ratatui::text::Span<'static>> {
    let bold = footer_key_style(theme).add_modifier(Modifier::BOLD);
    vec![
        ratatui::text::Span::styled("Enter", bold),
        ratatui::text::Span::raw(":send  │  "),
        ratatui::text::Span::styled("Shift+Tab", bold),
        ratatui::text::Span::raw(":mode  │  "),
        ratatui::text::Span::styled("Ctrl+x", bold),
        ratatui::text::Span::raw(":shortcuts"),
    ]
}

fn loading_footer_spans(frame: usize, theme: ThemeKind) -> Vec<ratatui::text::Span<'static>> {
    let frames = dot_spinner_frames();
    let spinner = frames[frame % frames.len()];
    vec![ratatui::text::Span::styled(
        format!("{spinner} Loading..."),
        footer_muted_style(theme).add_modifier(Modifier::DIM),
    )]
}

fn active_footer_spans(theme: ThemeKind) -> Vec<ratatui::text::Span<'static>> {
    let bold = footer_key_style(theme).add_modifier(Modifier::BOLD);
    vec![
        ratatui::text::Span::styled("Shift+Tab", bold),
        ratatui::text::Span::raw(":mode  │  "),
        ratatui::text::Span::styled("Esc", bold),
        ratatui::text::Span::raw(":cancel  │  "),
        ratatui::text::Span::styled("Ctrl+.", bold),
        ratatui::text::Span::raw(":shortcuts"),
    ]
}

fn footer_key_style(theme: ThemeKind) -> Style {
    appearance::footer_key_style_for(theme)
}

fn footer_muted_style(theme: ThemeKind) -> Style {
    if theme == ThemeKind::GrokNight {
        Style::default()
    } else {
        appearance::muted_style_for(theme)
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::too_many_lines,
        reason = "snapshot-style footer assertions compare idle and active frames together"
    )]
    use super::*;
    use ratatui::style::Color;

    #[test]
    fn default_is_ready() {
        assert_eq!(StatusBar::new().current(), &Status::Ready);
    }

    #[test]
    fn theme_is_an_actor_owned_status_projection() {
        let mut bar = StatusBar::new();
        assert_eq!(bar.theme(), ThemeKind::GrokNight);
        bar.set_theme(ThemeKind::GrokDay);
        assert_eq!(bar.theme(), ThemeKind::GrokDay);
    }

    #[test]
    fn renderer_adapter_preserves_status_projection_fields() {
        let mut source = StatusBar::new();
        source.set_theme(ThemeKind::GrokDay);
        source.set(Status::Thinking);
        source.advance_animation();
        let snapshot = source.model_snapshot();
        let adapted = StatusBar::from_model_snapshot(snapshot.clone());
        assert_eq!(adapted.model_snapshot(), snapshot);
    }

    #[test]
    fn status_preserves_every_declared_theme_variant() {
        let variants = [
            ThemeKind::GrokNight,
            ThemeKind::GrokDay,
            ThemeKind::TokyoNight,
            ThemeKind::RosePineMoon,
            ThemeKind::OscuraMidnight,
            ThemeKind::Auto,
        ];
        for theme in variants {
            let mut bar = StatusBar::new();
            bar.set_theme(theme);
            assert_eq!(bar.theme(), theme);
        }
    }

    #[test]
    fn status_footer_and_turn_status_use_selected_theme_tokens() {
        let mut bar = StatusBar::new();
        bar.set_theme(ThemeKind::GrokDay);
        let mut buffer = Buffer::empty(Rect::new(0, 0, 80, 1));
        bar.render(Rect::new(0, 0, 80, 1), &mut buffer);
        let enter = buffer.cell((0, 0)).expect("footer cell");
        assert_eq!(
            Some(enter.fg),
            appearance::base_style_for(ThemeKind::GrokDay).fg
        );

        let mut turn_buffer = Buffer::empty(Rect::new(0, 0, 40, 1));
        TurnStatus::new(0)
            .phase(TurnStatusPhase::Thinking)
            .with_theme(ThemeKind::GrokDay)
            .render(Rect::new(0, 0, 40, 1), &mut turn_buffer);
        assert_eq!(
            Some(turn_buffer.cell((2, 0)).expect("spinner cell").fg),
            appearance::accent_style_for(ThemeKind::GrokDay).fg
        );
    }

    #[test]
    fn label_distinct_per_variant() {
        let variants = [
            Status::Ready,
            Status::Loading,
            Status::Thinking,
            Status::Streaming,
            Status::Aborted,
            Status::Error("x".into()),
        ];
        let labels: Vec<_> = variants.iter().map(Status::label).collect();
        let unique: std::collections::HashSet<_> = labels.iter().collect();
        assert_eq!(unique.len(), labels.len());
    }

    #[test]
    fn full_mode_footer_matches_grok_idle_and_active_hints() {
        let mut bar = StatusBar::new();
        let mut buffer = Buffer::empty(Rect {
            x: 0,
            y: 0,
            width: 80,
            height: 1,
        });
        bar.render(
            Rect {
                x: 0,
                y: 0,
                width: 80,
                height: 1,
            },
            &mut buffer,
        );
        let idle: String = (0..80)
            .filter_map(|x| buffer.cell((x, 0)).map(|c| c.symbol().to_string()))
            .collect();
        assert!(idle.contains("Enter:send"));
        assert!(idle.contains("Shift+Tab:mode"));
        assert!(idle.contains("Ctrl+x:shortcuts"));

        bar.set(Status::Thinking);
        let mut buffer = Buffer::empty(Rect {
            x: 0,
            y: 0,
            width: 80,
            height: 1,
        });
        bar.render(
            Rect {
                x: 0,
                y: 0,
                width: 80,
                height: 1,
            },
            &mut buffer,
        );
        let active: String = (0..80)
            .filter_map(|x| buffer.cell((x, 0)).map(|c| c.symbol().to_string()))
            .collect();
        assert!(active.contains("Shift+Tab:mode"));
        assert!(active.contains("Esc:cancel"));
        assert!(active.contains("Ctrl+.:shortcuts"));
        assert!(!active.contains("Thinking…"));
    }

    #[test]
    fn active_footer_is_stable_across_animation_frames() {
        let mut bar = StatusBar::new();
        bar.set(Status::Thinking);
        let mut frames = Vec::new();
        for frame in 0..3 {
            bar.set_animation_frame(frame);
            let mut buffer = Buffer::empty(Rect {
                x: 0,
                y: 0,
                width: 80,
                height: 1,
            });
            bar.render(
                Rect {
                    x: 0,
                    y: 0,
                    width: 80,
                    height: 1,
                },
                &mut buffer,
            );
            frames.push(
                (0..80)
                    .filter_map(|x| buffer.cell((x, 0)).map(|c| c.symbol().to_string()))
                    .collect::<String>(),
            );
        }
        assert_eq!(frames[0], frames[1]);
        assert_eq!(frames[1], frames[2]);
        insta::assert_snapshot!(frames.join("\n"));
    }

    #[test]
    fn turn_status_projects_usage_and_stop_reason_from_event_state() {
        let mut bar = StatusBar::new();
        bar.begin_turn();
        bar.finish_turn(
            Usage {
                total_tokens: 42,
                ..Usage::default()
            },
            StopReason::ToolUse,
        );
        bar.set(Status::Streaming);
        let text = bar.turn_status().expect("active turn status").text();
        assert!(text.contains("⇣42"));
        bar.finish_turn(
            Usage {
                total_tokens: 3_180,
                ..Usage::default()
            },
            StopReason::Stop,
        );
        assert!(bar
            .turn_status()
            .expect("active turn status")
            .text()
            .contains("⇣3.18k"));
        assert!(text.contains("toolUse"));
    }

    #[test]
    fn worked_for_label_uses_owned_deterministic_elapsed_ticks() {
        let mut bar = StatusBar::new();
        bar.begin_turn();
        for _ in 0..22 {
            bar.set(Status::Thinking);
            bar.advance_animation();
        }
        assert_eq!(bar.worked_for_label(), "Worked for 1.1s");
    }

    #[test]
    fn header_meter_projects_event_owned_usage() {
        let mut bar = StatusBar::new();
        assert_eq!(bar.header_meter(), "0 / 500K");
        bar.finish_turn(
            Usage {
                total_tokens: 18_000,
                ..Usage::default()
            },
            StopReason::Stop,
        );
        assert_eq!(bar.header_meter(), "18K / 500K");
    }

    #[test]
    fn status_messages_are_pure_event_projection_inputs() {
        let mut bar = StatusBar::new();
        bar.apply(StatusMsg::BeginTurn);
        bar.apply(StatusMsg::Set(Status::Thinking));
        bar.apply(StatusMsg::AdvanceAnimation);
        assert_eq!(bar.current(), &Status::Thinking);
        assert_eq!(bar.worked_for_label(), "Worked for 0.0s");
        bar.apply(StatusMsg::FinishTurn(
            Usage {
                total_tokens: 1_200,
                ..Usage::default()
            },
            StopReason::Stop,
        ));
        assert_eq!(bar.header_meter(), "1.2K / 500K");
    }

    #[test]
    fn animation_frame_is_deterministic_and_owned_by_status_bar() {
        let mut bar = StatusBar::new();
        assert_eq!(bar.animation_frame(), 0);
        bar.set(Status::Thinking);
        bar.advance_animation();
        assert_eq!(bar.animation_frame(), 1);
        bar.set(Status::Ready);
        bar.advance_animation();
        assert_eq!(bar.animation_frame(), 1);
    }

    #[test]
    fn animation_demand_is_false_for_idle_and_terminal_states() {
        let mut bar = StatusBar::new();
        assert!(!bar.animation_demand());
        bar.set(Status::Thinking);
        assert!(bar.animation_demand());
        bar.set(Status::Ready);
        assert!(!bar.animation_demand());
        bar.set(Status::Error("done".into()));
        assert!(!bar.animation_demand());
    }

    #[test]
    fn active_footer_matches_grok_full_mode_vocabulary_and_spacing() {
        let mut bar = StatusBar::new();
        bar.set(Status::Thinking);
        let mut buffer = Buffer::empty(Rect {
            x: 0,
            y: 0,
            width: 80,
            height: 1,
        });
        bar.render(
            Rect {
                x: 2,
                y: 0,
                width: 76,
                height: 1,
            },
            &mut buffer,
        );
        let row: String = (0..80)
            .map(|x| buffer.cell((x, 0)).expect("footer cell").symbol())
            .collect();
        assert!(row.starts_with("  Shift+Tab:mode  │  Esc:cancel  │  Ctrl+.:shortcuts"));
        assert_eq!(row.chars().count(), 80);
    }

    #[test]
    fn spinner_frame_helpers_match_grok_glyphs() {
        assert_eq!(
            braille_spinner_frames(),
            &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"]
        );
        assert_eq!(braille_spinner_fallback(), &["|", "/", "-", "\\"]);
        assert_eq!(dot_spinner_frames(), &["⋅", ":", "⸬", "⁙"]);
        assert_eq!(dot_spinner_fallback(), &[".", ":", "·"]);
    }

    #[test]
    fn loading_status_renders_grok_loading_row() {
        let mut bar = StatusBar::new();
        bar.set(Status::Loading);
        bar.set_animation_frame(0);
        let mut buffer = Buffer::empty(Rect {
            x: 0,
            y: 0,
            width: 40,
            height: 1,
        });
        bar.render(
            Rect {
                x: 0,
                y: 0,
                width: 40,
                height: 1,
            },
            &mut buffer,
        );
        let row: String = (0..40)
            .filter_map(|x| buffer.cell((x, 0)).map(|c| c.symbol().to_string()))
            .collect();
        assert!(
            row.contains("Loading..."),
            "loading row should show the grok Loading label, got: {row:?}"
        );
        assert!(
            row.starts_with(dot_spinner_frames()[0]),
            "loading row should start with the dot spinner"
        );
    }

    #[test]
    fn turn_status_uses_groks_deterministic_braille_frames() {
        assert_eq!(TurnStatus::new(0).text(), "  ⠋ Starting session… 0.0s");
        assert_eq!(TurnStatus::new(8).text(), "  ⠹ Starting session… 0.0s");
        assert_eq!(TurnStatus::new(3).text(), "  ⠙ Starting session… 0.0s");
        assert!(TurnStatus::new(0)
            .phase(TurnStatusPhase::Waiting)
            .text()
            .contains("Waiting for response…"));
        assert!(TurnStatus::new(0)
            .phase(TurnStatusPhase::Responding)
            .text()
            .contains("Responding…"));
    }

    #[test]
    fn thinking_turn_status_matches_grok_working_marker() {
        assert_eq!(
            TurnStatus::new(0).phase(TurnStatusPhase::Thinking).text(),
            "┃  ◆ Thinking…"
        );
    }

    #[test]
    fn turn_status_holds_frames_at_grok_equivalent_cadence_and_colors_roles() {
        assert_eq!(TurnStatus::new(2).text(), TurnStatus::new(0).text());
        assert_ne!(TurnStatus::new(3).text(), TurnStatus::new(0).text());
        let mut buffer = Buffer::empty(Rect::new(0, 0, 40, 1));
        TurnStatus::new(0).render(Rect::new(0, 0, 40, 1), &mut buffer);
        assert_eq!(
            buffer.cell((2, 0)).expect("spinner").fg,
            Color::Rgb(187, 154, 247)
        );
        assert_eq!(
            buffer.cell((4, 0)).expect("label").fg,
            Color::Rgb(108, 108, 108)
        );
        assert!(!buffer
            .cell((2, 0))
            .expect("spinner")
            .modifier
            .contains(Modifier::DIM));
    }
}
