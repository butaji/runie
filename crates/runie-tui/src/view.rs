//! Declarative TUI view descriptions.
//!
//! This module deliberately knows nothing about Ratatui buffers or terminal
//! coordinates. Actors provide the model, pure view functions describe the
//! element tree, and the rendering adapter turns that tree into terminal
//! regions.

use std::fmt;

use runie_core::types::ThemeKind;

/// Small, explicit view DSL. It only expands to `Element` constructors; it
/// owns no state and performs no rendering.
#[macro_export]
macro_rules! view {
    (vertical [$($child:expr),* $(,)?]) => {
        $crate::view::Element::vertical([$($child),*])
    };
    (vertical_iter $children:expr) => {
        $crate::view::Element::vertical($children)
    };
    (horizontal [$($child:expr),* $(,)?]) => {
        $crate::view::Element::horizontal([$($child),*])
    };
    (slot $slot:expr) => {
        $crate::view::Element::slot($slot)
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Slot {
    Header,
    Scrollback,
    Prompt,
    Status,
    FooterBadge,
    WelcomeOverlay,
    ShortcutsOverlay,
    CommandPaletteOverlay,
    DoctorHint,
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
    WelcomeOverlay,
    ShortcutsOverlay,
    CommandPaletteOverlay,
    DoctorHint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StateOwner {
    UiActor,
    ScrollbackActor,
    PromptActor,
    StatusActor,
}

/// Semantic paint intent. It is deliberately not a Ratatui `Style`: terminal
/// colors, modifiers, and capability quantization belong to the renderer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PaintIntent {
    Base,
    Panel,
    Muted,
    Accent,
    SecondaryAccent,
    Success,
    Error,
    Warning,
    Selection,
    SelectionBorder,
    DiffInsert,
    DiffDelete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ComponentSpec {
    pub kind: ComponentKind,
    pub slot: Slot,
    pub owner: StateOwner,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ChatViewProps {
    pub welcome_visible: bool,
    pub shortcuts_visible: bool,
    pub command_palette_visible: bool,
    pub doctor_hint_visible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderViewProps {
    pub meter: String,
    pub theme: ThemeKind,
}

pub const CHAT_COMPONENTS: [ComponentSpec; 9] = [
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
    ComponentSpec {
        kind: ComponentKind::WelcomeOverlay,
        slot: Slot::WelcomeOverlay,
        owner: StateOwner::UiActor,
    },
    ComponentSpec {
        kind: ComponentKind::ShortcutsOverlay,
        slot: Slot::ShortcutsOverlay,
        owner: StateOwner::UiActor,
    },
    ComponentSpec {
        kind: ComponentKind::CommandPaletteOverlay,
        slot: Slot::CommandPaletteOverlay,
        owner: StateOwner::UiActor,
    },
    ComponentSpec {
        kind: ComponentKind::DoctorHint,
        slot: Slot::DoctorHint,
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
            Self::WelcomeOverlay => "welcome-overlay",
            Self::ShortcutsOverlay => "shortcuts-overlay",
            Self::CommandPaletteOverlay => "command-palette-overlay",
            Self::DoctorHint => "doctor-hint",
        })
    }
}

/// The stable element tree for the chat surface. Geometry belongs to the
/// layout adapter; changing terminal dimensions must not change this tree.
pub fn chat_view() -> Element {
    chat_view_with_props(ChatViewProps::default())
}

pub fn chat_view_with_props(props: ChatViewProps) -> Element {
    let mut children = vec![
        view!(slot Slot::Header),
        view!(slot Slot::Scrollback),
        view!(slot Slot::Prompt),
        view!(slot Slot::Status),
        view!(slot Slot::FooterBadge),
    ];
    if props.welcome_visible {
        children.push(view!(slot Slot::WelcomeOverlay));
    }
    if props.shortcuts_visible {
        children.push(view!(slot Slot::ShortcutsOverlay));
    }
    if props.command_palette_visible {
        children.push(view!(slot Slot::CommandPaletteOverlay));
    }
    if props.doctor_hint_visible {
        children.push(view!(slot Slot::DoctorHint));
    }
    view!(vertical_iter children)
}

pub fn component(slot: Slot) -> ComponentSpec {
    CHAT_COMPONENTS
        .into_iter()
        .find(|spec| spec.slot == slot)
        .expect("chat slots have component specs")
}

#[cfg(test)]
mod tests {
    use super::{
        chat_view, chat_view_with_props, component, ChatViewProps, ComponentKind, Direction,
        Element, HeaderViewProps, Slot, StateOwner,
    };
    use runie_core::types::ThemeKind;

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
        let overlays = chat_view_with_props(ChatViewProps {
            command_palette_visible: true,
            ..ChatViewProps::default()
        })
        .slots()
        .collect::<Vec<_>>();
        assert!(overlays.contains(&Slot::CommandPaletteOverlay));
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

    #[test]
    fn view_macro_expands_to_plain_elements() {
        let tree = crate::view!(vertical [
            crate::view!(slot Slot::Header),
            crate::view!(horizontal [crate::view!(slot Slot::Prompt)]),
        ]);
        assert_eq!(
            tree.slots().collect::<Vec<_>>(),
            vec![Slot::Header, Slot::Prompt]
        );
    }

    #[test]
    fn header_props_are_renderer_neutral() {
        let props = HeaderViewProps {
            meter: "15K / 500K".into(),
            theme: ThemeKind::GrokNight,
        };
        assert_eq!(props.meter, "15K / 500K");
        assert_eq!(props.theme, ThemeKind::GrokNight);
    }
}
