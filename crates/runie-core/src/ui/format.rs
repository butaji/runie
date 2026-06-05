//! Format - Rendering UI elements to display lines

use crate::ui::elements::{Element, Feed};
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
    let feed = crate::ui::Dsl::feed(state);
    render_feed(&feed, state)
}

pub fn render_feed(feed: &Feed, state: &crate::model::AppState) -> Vec<DisplayLine> {
    let mut lines = vec![];
    
    for element in &feed.elements {
        lines.extend(render_element(element, state));
    }
    
    lines
}

fn render_element(element: &Element, state: &crate::model::AppState) -> Vec<DisplayLine> {
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
            let spinner = state.spinner_frame();
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
        
        Element::ToolRunning { name, elapsed } => {
            let spinner = state.spinner_frame();
            let text = format!("{} Running {}... {:.1}s", spinner, name, elapsed);
            vec![
                DisplayLine {
                    spans: vec![DisplaySpan { text, color: Some(Color::DarkGray) }],
                },
            ]
        }
        
        Element::ToolDone { name, duration_secs } => {
            let text = format!("◆ Ran {} {:.1}s", name, duration_secs);
            vec![
                DisplayLine {
                    spans: vec![DisplaySpan { text, color: Some(Color::DarkGray) }],
                },
            ]
        },
        
        Element::TurnComplete { duration_secs } => vec![
            DisplayLine {
                spans: vec![
                    DisplaySpan { text: "Turn completed in ".to_string(), color: Some(Color::DarkGray) },
                    DisplaySpan { text: format!("{:.1}s", duration_secs), color: Some(Color::DarkGray) },
                ],
            },
        ],
        
        Element::Group { elements, .. } => {
            let mut lines = vec![];
            for element in elements {
                lines.extend(render_element(element, state));
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
