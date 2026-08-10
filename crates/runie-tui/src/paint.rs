//! Renderer-neutral paint instructions.
//!
//! Widgets may project immutable model snapshots into these values without
//! touching a terminal buffer. The Ratatui adapter can interpret them later,
//! while tests can assert the view data directly.

use crate::{
    appearance,
    view::{ComponentKind, PaintIntent, Slot},
};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};
use runie_core::types::ThemeKind;
use runie_tui_model::{
    InputMode, PromptSnapshot, StatusSnapshot, ToolCardPaintIntent, ToolCardRow,
};

impl From<ToolCardPaintIntent> for PaintIntent {
    fn from(intent: ToolCardPaintIntent) -> Self {
        match intent {
            ToolCardPaintIntent::Header | ToolCardPaintIntent::Content => Self::Base,
            ToolCardPaintIntent::Running => Self::Accent,
            ToolCardPaintIntent::Success => Self::Success,
            ToolCardPaintIntent::Error => Self::Error,
            ToolCardPaintIntent::Muted => Self::Muted,
        }
    }
}

#[macro_export]
macro_rules! paint {
    ($(($slot:expr, $component:expr, $text:expr, $intent:expr)),+ $(,)?) => {{
        let document = $crate::paint::PaintDocument::default();
        $(let document = document.text($slot, $component, $text, $intent);)*
        document
    }};
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PaintText {
    pub slot: Slot,
    pub component: ComponentKind,
    pub text: String,
    pub intent: PaintIntent,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PaintSpan {
    pub text: String,
    pub intent: PaintIntent,
    pub bold: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct PaintDocument {
    pub text: Vec<PaintText>,
    #[serde(default)]
    pub inline: Vec<PaintSpan>,
}

impl PaintDocument {
    pub fn text(
        mut self,
        slot: Slot,
        component: ComponentKind,
        text: impl Into<String>,
        intent: PaintIntent,
    ) -> Self {
        self.text.push(PaintText {
            slot,
            component,
            text: text.into(),
            intent,
        });
        self
    }

    pub fn span(mut self, text: impl Into<String>, intent: PaintIntent, bold: bool) -> Self {
        self.inline.push(PaintSpan {
            text: text.into(),
            intent,
            bold,
        });
        self
    }
}

/// Pure status projection used by future terminal adapters.
pub fn status_paint(snapshot: &StatusSnapshot) -> PaintDocument {
    let intent = if snapshot.state.label().starts_with("error:") {
        PaintIntent::Error
    } else if snapshot.animation_demand() {
        PaintIntent::Accent
    } else {
        PaintIntent::Muted
    };
    crate::paint![(
        Slot::Status,
        ComponentKind::Status,
        snapshot.state.label(),
        intent
    ),]
}

/// Project semantic tool-card rows into renderer-neutral paint data.
pub fn tool_card_paint(rows: &[ToolCardRow]) -> PaintDocument {
    let mut document = PaintDocument::default();
    for row in rows {
        document = document.text(
            Slot::Scrollback,
            ComponentKind::Scrollback,
            row.text.clone(),
            row.paint_intent().into(),
        );
    }
    document
}

/// Interpret renderer-neutral text instructions at the terminal boundary.
/// Intent-to-style mapping can evolve here without changing model projections.
pub fn render_paint_document(
    document: &PaintDocument,
    theme: ThemeKind,
    area: Rect,
    buffer: &mut Buffer,
) {
    let mut lines = document
        .text
        .iter()
        .map(|text| {
            Line::from(Span::styled(
                text.text.clone(),
                appearance::style_for_intent(theme, text.intent),
            ))
        })
        .collect::<Vec<_>>();
    if !document.inline.is_empty() {
        let spans = document
            .inline
            .iter()
            .map(|span| {
                let mut style = appearance::style_for_intent(theme, span.intent);
                if span.bold {
                    style = style.add_modifier(ratatui::style::Modifier::BOLD);
                }
                Span::styled(span.text.clone(), style)
            })
            .collect::<Vec<_>>();
        lines.push(Line::from(spans));
    }
    Paragraph::new(lines).render(area, buffer);
}

/// Minimal Ratatui adapter for the status paint document.
pub fn render_status_paint(snapshot: &StatusSnapshot, area: Rect, buffer: &mut Buffer) {
    render_paint_document(&status_paint(snapshot), snapshot.theme, area, buffer);
}

pub fn prompt_paint(snapshot: &PromptSnapshot) -> PaintDocument {
    let intent = if snapshot.mode == InputMode::Plan {
        PaintIntent::Warning
    } else {
        PaintIntent::Base
    };
    crate::paint![
        (
            Slot::Prompt,
            ComponentKind::Prompt,
            snapshot.text.clone(),
            PaintIntent::Base
        ),
        (
            Slot::Prompt,
            ComponentKind::Prompt,
            snapshot.caption(),
            intent
        ),
    ]
}

pub fn render_prompt_paint(snapshot: &PromptSnapshot, area: Rect, buffer: &mut Buffer) {
    render_paint_document(&prompt_paint(snapshot), snapshot.theme, area, buffer);
}

pub fn status_footer_paint(snapshot: &StatusSnapshot) -> PaintDocument {
    let mut document = PaintDocument::default();
    match &snapshot.state {
        runie_tui_model::Status::Ready => footer_spans(
            &mut document,
            [
                ("Enter", "send"),
                ("Shift+Tab", "mode"),
                ("Ctrl+x", "shortcuts"),
            ],
        ),
        runie_tui_model::Status::Loading => document.span(
            format!(
                "{} Loading...",
                runie_tui_model::DOT_SPINNER_FRAMES
                    [snapshot.animation_frame % runie_tui_model::DOT_SPINNER_FRAMES.len()]
            ),
            PaintIntent::Muted,
            false,
        ),
        runie_tui_model::Status::Thinking
        | runie_tui_model::Status::Streaming
        | runie_tui_model::Status::Waiting(_) => footer_spans(
            &mut document,
            [
                ("Shift+Tab", "mode"),
                ("Esc", "cancel"),
                ("Ctrl+.", "shortcuts"),
            ],
        ),
        state => document.span(state.label(), PaintIntent::Muted, false),
    }
}

fn footer_spans<const N: usize>(
    document: &mut PaintDocument,
    actions: [(&'static str, &'static str); N],
) -> PaintDocument {
    for (index, (key, action)) in actions.into_iter().enumerate() {
        if index > 0 {
            document.inline.push(PaintSpan {
                text: "  │  ".into(),
                intent: PaintIntent::Muted,
                bold: false,
            });
        }
        document.inline.push(PaintSpan {
            text: key.into(),
            intent: PaintIntent::FooterKey,
            bold: true,
        });
        document.inline.push(PaintSpan {
            text: format!(":{action}"),
            intent: PaintIntent::Muted,
            bold: false,
        });
    }
    std::mem::take(document)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paint_document_is_pure_data() {
        let document = PaintDocument::default().text(
            Slot::Status,
            ComponentKind::Status,
            "Thinking",
            PaintIntent::Accent,
        );
        assert_eq!(document.text[0].text, "Thinking");
        assert_eq!(document.text[0].intent, PaintIntent::Accent);
    }

    #[test]
    fn paint_document_round_trips_as_yaml_data() {
        let document = PaintDocument::default().text(
            Slot::Status,
            ComponentKind::Status,
            "Thinking",
            PaintIntent::Accent,
        );
        let restored: PaintDocument =
            serde_yaml::from_str(include_str!("fixtures/paint-document.yaml"))
                .expect("paint YAML restore");
        assert_eq!(restored, document);
    }

    #[test]
    fn status_paint_projects_state_without_a_terminal_buffer() {
        let snapshot = StatusSnapshot {
            state: runie_tui_model::Status::Thinking,
            ..StatusSnapshot::default()
        };
        let document = status_paint(&snapshot);
        assert_eq!(document.text[0].text, "thinking...");
        assert_eq!(document.text[0].intent, PaintIntent::Accent);
    }

    #[test]
    fn status_footer_paint_keeps_inline_hotkey_structure_as_data() {
        let document = status_footer_paint(&StatusSnapshot::default());
        let rendered = document
            .inline
            .iter()
            .map(|span| span.text.as_str())
            .collect::<String>();

        assert_eq!(
            rendered,
            "Enter:send  │  Shift+Tab:mode  │  Ctrl+x:shortcuts"
        );
        assert!(document.inline[0].bold);
        assert_eq!(document.inline[0].intent, PaintIntent::FooterKey);
    }

    #[test]
    fn paint_adapter_renders_the_projected_text() {
        let snapshot = StatusSnapshot::default();
        let mut buffer = Buffer::empty(ratatui::layout::Rect::new(0, 0, 20, 1));
        render_status_paint(
            &snapshot,
            ratatui::layout::Rect::new(0, 0, 20, 1),
            &mut buffer,
        );
        assert_eq!(buffer.cell((0, 0)).map(|cell| cell.symbol()), Some("r"));
    }

    #[test]
    fn paint_adapter_preserves_semantic_intent_as_theme_style() {
        let document = PaintDocument::default().text(
            Slot::Status,
            ComponentKind::Status,
            "busy",
            PaintIntent::Accent,
        );
        let mut buffer = Buffer::empty(Rect::new(0, 0, 10, 1));
        render_paint_document(
            &document,
            ThemeKind::GrokNight,
            Rect::new(0, 0, 10, 1),
            &mut buffer,
        );
        assert_eq!(
            buffer.cell((0, 0)).map(|cell| cell.fg),
            appearance::accent_style_for(ThemeKind::GrokNight).fg
        );
    }

    #[test]
    fn prompt_paint_projects_mode_caption_as_data() {
        let snapshot = PromptSnapshot {
            mode: InputMode::Plan,
            model_caption: "model".into(),
            text: "draft".into(),
            ..PromptSnapshot::default()
        };
        let document = prompt_paint(&snapshot);
        assert_eq!(document.text[0].text, "draft");
        assert_eq!(document.text[1].text, "plan · model");
        assert_eq!(document.text[1].intent, PaintIntent::Warning);
    }

    #[test]
    fn prompt_snapshot_fixture_replays_as_renderer_neutral_data() {
        let snapshot: PromptSnapshot =
            serde_yaml::from_str(include_str!("fixtures/prompt-snapshot.yaml"))
                .expect("prompt snapshot YAML restore");
        assert_eq!(snapshot.caption(), "plan · model");
        assert_eq!(prompt_paint(&snapshot).text[0].text, "draft");
    }

    #[test]
    fn tool_card_paint_projects_semantic_intents_without_terminal_state() {
        let row = ToolCardRow {
            tool_call_id: "tool".into(),
            tool_row_id: None,
            member_index: 0,
            card_kind: runie_tui_model::ToolCardKind::Execute,
            row_kind: runie_tui_model::ToolCardRowKind::Header,
            text: "bash".into(),
            mode: runie_core::types::ToolDisplayMode::Truncated,
            is_running: true,
            is_error: false,
        };
        let document = tool_card_paint(&[row]);
        assert_eq!(document.text[0].slot, Slot::Scrollback);
        assert_eq!(document.text[0].intent, PaintIntent::Accent);
    }

    #[test]
    fn prompt_adapter_renders_projected_lines() {
        let snapshot = PromptSnapshot {
            text: "draft".into(),
            model_caption: "model".into(),
            ..PromptSnapshot::default()
        };
        let mut buffer = Buffer::empty(Rect::new(0, 0, 20, 2));
        render_prompt_paint(&snapshot, Rect::new(0, 0, 20, 2), &mut buffer);
        assert_eq!(buffer.cell((0, 0)).map(|cell| cell.symbol()), Some("d"));
    }
}
