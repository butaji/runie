//! Format — Legacy DisplayLine/DisplaySpan rendering
use crate::ui::elements::{Element, Feed};
use crate::labels::{PREFIX_USER, PREFIX_AGENT};

#[derive(Debug, Clone)]
pub struct DisplayLine { pub spans: Vec<DisplaySpan> }

#[derive(Debug, Clone)]
pub struct DisplaySpan { pub text: String, pub color: Option<Color> }

#[derive(Debug, Clone, Copy)]
pub enum Color { Cyan, Green, Yellow, DarkGray, White, Magenta }

pub fn format_messages(state: &crate::model::AppState) -> Vec<DisplayLine> {
    let feed = crate::ui::LazyCache::feed(state);
    render_feed(&feed, state)
}

pub fn render_feed(feed: &Feed, state: &crate::model::AppState) -> Vec<DisplayLine> {
    feed.elements.iter()
        .flat_map(|e| render_element(e, state))
        .collect()
}

fn render_element(elem: &Element, state: &crate::model::AppState) -> Vec<DisplayLine> {
    match elem {
        Element::Spacer => vec![DisplayLine::empty()],
        Element::UserMessage { content } => line(PREFIX_USER, content, Color::Cyan),
        Element::AgentMessage { content } => line(PREFIX_AGENT, content, Color::Green),
        Element::Thinking { elapsed } => gray_line(format!("{} Though... {:.1}s", state.spinner_frame(), elapsed)),
        Element::ThoughtMarker { content } => gray_line(content.clone()),
        Element::ToolRunning { name, elapsed } => gray_line(format!("{} Running {}... {:.1}s", state.spinner_frame(), name, elapsed)),
        Element::ToolDone { name, duration_secs } => gray_line(format!("◆ Ran {} {:.1}s", name, duration_secs)),
        Element::TurnComplete { duration_secs } => gray_line(format!("Turn completed in {:.1}s", duration_secs)),
        Element::Group { elements, .. } => elements.iter().flat_map(|e| render_element(e, state)).collect(),
    }
}

fn line(prefix: &str, content: &str, color: Color) -> Vec<DisplayLine> {
    vec![DisplayLine { spans: vec![
        DisplaySpan { text: prefix.to_string(), color: Some(color) },
        DisplaySpan { text: content.to_string(), color: None },
    ]}]
}

fn gray_line(text: String) -> Vec<DisplayLine> {
    vec![DisplayLine { spans: vec![
        DisplaySpan { text, color: Some(Color::DarkGray) }
    ]}]
}

impl DisplayLine {
    pub fn empty() -> Self { DisplayLine { spans: vec![] } }
    pub fn is_empty(&self) -> bool { self.spans.iter().all(|s| s.text.is_empty()) }
    pub fn to_text(&self) -> String { self.spans.iter().map(|s| s.text.clone()).collect() }
}
