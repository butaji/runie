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

/// Semantic component identity. These names are stable across terminal
/// backends and are the vocabulary used by YAML/view assertions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComponentKind {
    Header,
    Scrollback,
    Prompt,
    Status,
    FooterBadge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StateOwner {
    UiActor,
    ScrollbackActor,
    PromptActor,
    StatusActor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ComponentSpec {
    pub kind: ComponentKind,
    pub slot: Slot,
    pub owner: StateOwner,
}

pub const CHAT_COMPONENTS: [ComponentSpec; 5] = [
    ComponentSpec {
        kind: ComponentKind::Header,
        slot: Slot::Header,
        owner: StateOwner::UiActor,
    },
    ComponentSpec {
        kind: ComponentKind::Scrollback,
        slot: Slot::Scrollback,
        owner: StateOwner::ScrollbackActor,
    },
    ComponentSpec {
        kind: ComponentKind::Prompt,
        slot: Slot::Prompt,
        owner: StateOwner::PromptActor,
    },
    ComponentSpec {
        kind: ComponentKind::Status,
        slot: Slot::Status,
        owner: StateOwner::StatusActor,
    },
    ComponentSpec {
        kind: ComponentKind::FooterBadge,
        slot: Slot::FooterBadge,
        owner: StateOwner::StatusActor,
    },
];

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

pub fn component(slot: Slot) -> ComponentSpec {
    CHAT_COMPONENTS
        .into_iter()
        .find(|spec| spec.slot == slot)
        .expect("chat slots have component specs")
}

#[cfg(test)]
mod tests {
    use super::{chat_view, component, ComponentKind, Direction, Element, Slot, StateOwner};

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
        assert_eq!(component(Slot::Scrollback).kind, ComponentKind::Scrollback);
        assert_eq!(
            component(Slot::Scrollback).owner,
            StateOwner::ScrollbackActor
        );
        assert_eq!(component(Slot::Prompt).owner, StateOwner::PromptActor);
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
