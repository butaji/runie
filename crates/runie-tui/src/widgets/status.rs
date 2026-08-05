//! 1-row status bar.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Paragraph, Widget};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    Ready,
    Loading,
    Thinking,
    Streaming,
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
            Self::Aborted => "aborted".into(),
            Self::Error(e) => format!("error: {e}"),
        }
    }

    pub fn style(&self) -> Style {
        match self {
            Self::Ready => Style::default().fg(Color::Green),
            Self::Loading => Style::default().fg(Color::DarkGray),
            Self::Thinking => Style::default().fg(Color::Yellow),
            Self::Streaming => Style::default().fg(Color::Blue),
            Self::Aborted => Style::default().fg(Color::DarkGray),
            Self::Error(_) => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
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

impl Default for Status {
    fn default() -> Self {
        Self::Ready
    }
}

#[derive(Debug, Clone, Default)]
pub struct StatusBar {
    state: Status,
    animation_frame: usize,
}

/// Grok's one-row foreground activity indicator above the prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnStatus {
    frame: usize,
    phase: TurnStatusPhase,
    chrome: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnStatusPhase {
    Starting,
    Waiting,
    Thinking,
    Responding,
}

impl TurnStatus {
    const FRAMES: [&'static str; 8] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"];

    pub fn new(frame: usize) -> Self {
        Self {
            frame,
            phase: TurnStatusPhase::Starting,
            chrome: String::new(),
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

    pub fn text(&self) -> String {
        let label = match self.phase {
            TurnStatusPhase::Starting => "Starting session… 0.0s",
            // The recorded full-mode waiting row includes the right-aligned
            // elapsed/usage/stop chrome on the same terminal row.
            TurnStatusPhase::Waiting => "Waiting for response…",
            TurnStatusPhase::Thinking => "Thinking…",
            TurnStatusPhase::Responding => "Responding…",
        };
        format!(
            "  {} {label}{}",
            Self::FRAMES[self.frame % Self::FRAMES.len()],
            self.chrome
        )
    }

    pub fn render(self, area: Rect, buf: &mut Buffer) {
        Paragraph::new(self.text())
            .style(Style::default().add_modifier(Modifier::DIM))
            .render(area, buf);
    }
}

impl StatusBar {
    pub fn new() -> Self {
        Self {
            state: Status::default(),
            animation_frame: 0,
        }
    }

    pub fn set(&mut self, s: Status) {
        self.state = s;
    }

    pub fn current(&self) -> &Status {
        &self.state
    }

    /// Advance the deterministic spinner used by active full-mode states.
    /// The caller owns the cadence; tests can select exact frames without
    /// depending on wall-clock timing.
    pub fn advance_animation(&mut self) {
        if matches!(
            self.state,
            Status::Loading | Status::Thinking | Status::Streaming
        ) {
            self.animation_frame = self.animation_frame.wrapping_add(1);
        }
    }

    pub fn set_animation_frame(&mut self, frame: usize) {
        self.animation_frame = frame;
    }

    pub fn animation_frame(&self) -> usize {
        self.animation_frame
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        // Grok's full-mode footer is context-sensitive: idle/editing exposes
        // send/mode/shortcuts, while an active turn exposes mode/cancel.
        use ratatui::text::Span;
        let left = match self.state {
            Status::Ready => "Enter:send │ Shift+Tab:mode │ Ctrl+x:shortcuts".to_string(),
            Status::Loading => String::new(),
            Status::Thinking | Status::Streaming => String::new(),
            _ => self.state.label(),
        };
        let mut spans = Vec::new();
        if matches!(self.state, Status::Ready) {
            spans.push(Span::styled(
                "Enter",
                Style::default().add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::raw(":send  │  "));
            spans.push(Span::styled(
                "Shift+Tab",
                Style::default().add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::raw(":mode  │  "));
            spans.push(Span::styled(
                "Ctrl+x",
                Style::default().add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::raw(":shortcuts"));
        } else if matches!(self.state, Status::Loading) {
            // Grok's "{spinner} Loading..." foreground row (agent_view/render.rs).
            let spinner = dot_spinner_frames()[self.animation_frame % dot_spinner_frames().len()];
            spans.push(Span::styled(
                format!("{spinner} Loading..."),
                Style::default().add_modifier(Modifier::DIM),
            ));
        } else if matches!(self.state, Status::Thinking | Status::Streaming) {
            spans.push(Span::styled(
                "Shift+Tab",
                Style::default().add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::raw(":mode  │  "));
            spans.push(Span::styled(
                "Esc",
                Style::default().add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::raw(":cancel  │  "));
            spans.push(Span::styled(
                "Ctrl+.",
                Style::default().add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::raw(":shortcuts"));
        } else {
            spans.push(Span::styled(left, self.state.style()));
        }
        let line = Line::from(spans).style(Style::default());
        let p = Paragraph::new(line);
        Widget::render(p, area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_ready() {
        assert_eq!(StatusBar::new().current(), &Status::Ready);
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
        assert_eq!(TurnStatus::new(8).text(), "  ⠋ Starting session… 0.0s");
        assert_eq!(TurnStatus::new(3).text(), "  ⠸ Starting session… 0.0s");
        assert!(TurnStatus::new(0)
            .phase(TurnStatusPhase::Waiting)
            .text()
            .contains("Waiting for response…"));
        assert!(TurnStatus::new(0)
            .phase(TurnStatusPhase::Responding)
            .text()
            .contains("Responding…"));
    }
}
