//! Format - Rendering UI elements to display lines
//! 
//! This module converts the DSL elements into display lines for rendering.

use crate::ui::elements::{Element, Feed};
use crate::ui::dsl::Dsl;
use crate::labels::{PREFIX_USER, PREFIX_AGENT};

/// A formatted line ready for rendering
#[derive(Debug, Clone)]
pub struct DisplayLine {
    pub spans: Vec<DisplaySpan>,
}

/// A single styled span of text
#[derive(Debug, Clone)]
pub struct DisplaySpan {
    pub text: String,
    pub color: Option<Color>,
}

/// Terminal colors
#[derive(Debug, Clone, Copy)]
pub enum Color {
    Cyan,
    Green,
    Yellow,
    DarkGray,
    White,
}

/// Build feed from state and render to display lines
pub fn format_messages(state: &crate::model::AppState) -> Vec<DisplayLine> {
    let feed = Dsl::feed(state);
    render_feed(&feed)
}

/// Render feed to display lines
pub fn render_feed(feed: &Feed) -> Vec<DisplayLine> {
    let mut lines = vec![];
    
    for element in &feed.elements {
        lines.extend(render_element(element));
    }
    
    lines
}

/// Render single element to display lines
fn render_element(element: &Element) -> Vec<DisplayLine> {
    match element {
        // Spacer provides 1 empty line between elements
        Element::Spacer => vec![DisplayLine::empty()],
        
        Element::UserMessage { content } => vec![
            DisplayLine {
                spans: vec![
                    DisplaySpan { text: PREFIX_USER.to_string(), color: Some(Color::Cyan) },
                    DisplaySpan { text: content.clone(), color: None },
                ],
            },
        ],
        
        Element::AgentMessage { content } => vec![
            DisplayLine {
                spans: vec![
                    DisplaySpan { text: PREFIX_AGENT.to_string(), color: Some(Color::Green) },
                    DisplaySpan { text: content.clone(), color: None },
                ],
            },
        ],
        
        Element::Thinking { elapsed } => {
            let spinner = Dsl::spinner(*elapsed);
            let text = format!("{} Thinking... {:.1}s", spinner, elapsed);
            vec![
                DisplayLine {
                    spans: vec![DisplaySpan { text, color: Some(Color::DarkGray) }],
                },
            ]
        }
        
        Element::ThoughtMarker { content } => vec![
            DisplayLine {
                spans: vec![DisplaySpan { text: content.clone(), color: Some(Color::DarkGray) }],
            },
        ],
        
        Element::Group { elements, .. } => {
            let mut lines = vec![];
            for element in elements {
                lines.extend(render_element(element));
            }
            lines
        }
    }
}

impl DisplayLine {
    pub fn empty() -> Self {
        DisplayLine { spans: vec![] }
    }
    
    pub fn is_empty(&self) -> bool {
        self.spans.iter().all(|s| s.text.is_empty())
    }
}
