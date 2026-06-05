//! Format - Rendering UI elements to display lines

use crate::ui::elements::{Element, Feed};
use crate::ui::dsl::Dsl;
use crate::labels::{PREFIX_USER, PREFIX_AGENT};

#[derive(Debug, Clone)]
pub struct DisplayLine {
    pub spans: Vec<DisplaySpan>,
}

#[derive(Debug, Clone)]
pub struct DisplaySpan {
    pub text: String,
    pub color: Option<Color>,
}

#[derive(Debug, Clone, Copy)]
pub enum Color {
    Cyan,
    Green,
    Yellow,
    DarkGray,
    White,
    Magenta,
}

pub fn format_messages(state: &crate::model::AppState) -> Vec<DisplayLine> {
    let feed = Dsl::feed(state);
    render_feed(&feed)
}

pub fn render_feed(feed: &Feed) -> Vec<DisplayLine> {
    let mut lines = vec![];
    
    for element in &feed.elements {
        lines.extend(render_element(element));
    }
    
    lines
}

fn render_element(element: &Element) -> Vec<DisplayLine> {
    match element {
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
            let text = format!("{} Though... {:.1}s", spinner, elapsed);
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
        
        Element::ToolRun { content } => vec![
            DisplayLine {
                spans: vec![DisplaySpan { text: content.clone(), color: Some(Color::Yellow) }],
            },
        ],
        
        Element::TurnComplete { duration_secs } => vec![
            DisplayLine {
                spans: vec![
                    DisplaySpan { text: "✓ Turn completed in ".to_string(), color: Some(Color::DarkGray) },
                    DisplaySpan { text: format!("{:.1}s", duration_secs), color: Some(Color::DarkGray) },
                ],
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
