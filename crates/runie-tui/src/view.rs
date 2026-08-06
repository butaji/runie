//! Declarative TUI view descriptions.
//!
//! This module deliberately knows nothing about Ratatui buffers or terminal
//! coordinates. Actors provide the model, pure view functions describe the
//! element tree, and the rendering adapter turns that tree into terminal
//! regions.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Slot {
    Header,
    Scrollback,
    Prompt,
    Status,
    FooterBadge,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Element {
    Empty,
    Slot(Slot),
    Stack {
        direction: Direction,
        children: Vec<Element>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Vertical,
    Horizontal,
}

impl Element {
    pub const fn slot(slot: Slot) -> Self {
        Self::Slot(slot)
    }

    pub fn vertical(children: impl IntoIterator<Item = Self>) -> Self {
        Self::Stack {
            direction: Direction::Vertical,
            children: children.into_iter().collect(),
        }
    }

    pub fn horizontal(children: impl IntoIterator<Item = Self>) -> Self {
        Self::Stack {
            direction: Direction::Horizontal,
            children: children.into_iter().collect(),
        }
    }

    pub fn slots(&self) -> impl Iterator<Item = Slot> + '_ {
        self.walk().into_iter()
    }

    fn walk(&self) -> Vec<Slot> {
        match self {
            Self::Empty => Vec::new(),
            Self::Slot(slot) => vec![*slot],
            Self::Stack { children, .. } => children.iter().flat_map(Self::walk).collect(),
        }
    }
}

impl fmt::Display for Slot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Header => "header",
            Self::Scrollback => "scrollback",
            Self::Prompt => "prompt",
            Self::Status => "status",
            Self::FooterBadge => "footer-badge",
        })
    }
}

/// The stable element tree for the chat surface. Geometry belongs to the
/// layout adapter; changing terminal dimensions must not change this tree.
pub fn chat_view() -> Element {
    Element::vertical([
        Element::slot(Slot::Header),
        Element::slot(Slot::Scrollback),
        Element::slot(Slot::Prompt),
        Element::slot(Slot::Status),
        Element::slot(Slot::FooterBadge),
    ])
}

#[cfg(test)]
mod tests {
    use super::{chat_view, Direction, Element, Slot};

    #[test]
    fn chat_view_is_a_stable_declarative_region_tree() {
        assert_eq!(
            chat_view(),
            Element::Stack {
                direction: Direction::Vertical,
                children: vec![
                    Element::Slot(Slot::Header),
                    Element::Slot(Slot::Scrollback),
                    Element::Slot(Slot::Prompt),
                    Element::Slot(Slot::Status),
                    Element::Slot(Slot::FooterBadge),
                ],
            }
        );
        assert_eq!(
            chat_view().slots().collect::<Vec<_>>(),
            vec![
                Slot::Header,
                Slot::Scrollback,
                Slot::Prompt,
                Slot::Status,
                Slot::FooterBadge,
            ]
        );
    }
}
