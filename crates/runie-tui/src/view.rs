//! Declarative TUI view descriptions.
//!
//! This module deliberately knows nothing about Ratatui buffers or terminal
//! coordinates. Actors provide the model, pure view functions describe the
//! element tree, and the rendering adapter turns that tree into terminal
//! regions.

use std::fmt;

use runie_core::types::ThemeKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutViewport {
    pub width: u16,
    pub height: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutDirection {
    Vertical,
    Horizontal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutSize {
    Auto,
    Fixed(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutEntry {
    pub slot: Slot,
    pub basis: LayoutSize,
    pub grow: u16,
    pub shrink: u16,
    pub min_size: u16,
    pub max_size: Option<u16>,
}

impl LayoutEntry {
    pub const fn fixed(slot: Slot, size: u16) -> Self {
        Self {
            slot,
            basis: LayoutSize::Fixed(size),
            grow: 0,
            shrink: 1,
            min_size: 0,
            max_size: None,
        }
    }

    pub const fn grow(slot: Slot, min_size: u16) -> Self {
        Self {
            slot,
            basis: LayoutSize::Auto,
            grow: 1,
            shrink: 1,
            min_size,
            max_size: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StackLayout {
    pub direction: LayoutDirection,
    pub gap: u16,
    pub entries: &'static [LayoutEntry],
}

impl StackLayout {
    /// Resolve the main-axis allocation without terminal or renderer state.
    /// Intrinsic sizes are supplied by component projections; basis/grow/
    /// shrink/min/max are the same declarative inputs used by pi's Stack.
    pub fn allocate(&self, intrinsic_sizes: &[u16], available_size: Option<u16>) -> Vec<u16> {
        let mut sizes = self
            .entries
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                let basis = match entry.basis {
                    LayoutSize::Auto => intrinsic_sizes.get(index).copied().unwrap_or(0),
                    LayoutSize::Fixed(size) => size,
                };
                clamp_layout_size(basis, *entry)
            })
            .collect::<Vec<_>>();
        let Some(available_size) = available_size else {
            return sizes;
        };
        let gap_total = self
            .entries
            .len()
            .saturating_sub(1)
            .saturating_mul(usize::from(self.gap));
        let content_size = usize::from(available_size).saturating_sub(gap_total);
        let total = sizes.iter().map(|size| usize::from(*size)).sum::<usize>();
        if total < content_size {
            distribute_layout_space(&mut sizes, self.entries, content_size - total, true);
        } else if total > content_size {
            distribute_layout_space(&mut sizes, self.entries, total - content_size, false);
        }
        sizes
    }
}

fn clamp_layout_size(size: u16, entry: LayoutEntry) -> u16 {
    let min = entry.min_size;
    let max = entry.max_size.unwrap_or(u16::MAX).max(min);
    size.max(min).min(max)
}

fn distribute_layout_space(
    sizes: &mut [u16],
    entries: &[LayoutEntry],
    mut remaining: usize,
    growing: bool,
) {
    while remaining > 0 {
        let candidates = layout_candidates(sizes, entries, growing);
        if candidates.is_empty() {
            return;
        }
        let weight = candidates
            .iter()
            .map(|(index, entry)| layout_weight(sizes, *index, **entry, growing))
            .sum::<usize>();
        let mut distributed = 0;
        for (index, entry) in candidates {
            if remaining == 0 {
                break;
            }
            let item_weight = layout_weight(sizes, index, *entry, growing);
            let proposed = (remaining * item_weight / weight).max(1);
            let capacity = if growing {
                usize::from(entry.max_size.unwrap_or(u16::MAX) - sizes[index])
            } else {
                usize::from(sizes[index] - entry.min_size)
            };
            let delta = proposed.min(remaining).min(capacity);
            if delta == 0 {
                continue;
            }
            sizes[index] = if growing {
                sizes[index].saturating_add(delta as u16)
            } else {
                sizes[index].saturating_sub(delta as u16)
            };
            remaining -= delta;
            distributed += delta;
        }
        if distributed == 0 {
            return;
        }
    }
}

fn layout_candidates<'a>(
    sizes: &[u16],
    entries: &'a [LayoutEntry],
    growing: bool,
) -> Vec<(usize, &'a LayoutEntry)> {
    entries
        .iter()
        .enumerate()
        .filter(|(index, entry)| {
            if growing {
                entry.grow > 0 && sizes[*index] < entry.max_size.unwrap_or(u16::MAX)
            } else {
                entry.shrink > 0 && sizes[*index] > entry.min_size
            }
        })
        .collect()
}

fn layout_weight(sizes: &[u16], index: usize, entry: LayoutEntry, growing: bool) -> usize {
    if growing {
        usize::from(entry.grow)
    } else {
        usize::from(entry.shrink) * usize::from(sizes[index]).max(1)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overscroll {
    Chain,
    Contain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollLayout {
    pub slot: Slot,
    pub primary: bool,
    pub overscroll: Overscroll,
    pub follow_end: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutNode {
    Stack(StackLayout),
    Scroll(ScrollLayout),
    Slot(Slot),
}

/// Pure actor-owned scroll projection, equivalent to pi's ScrollView state.
/// Rendering only consumes this value; it never decides whether the feed
/// follows new content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollState {
    pub scroll_top: u16,
    pub content_height: u16,
    pub viewport_height: u16,
    pub following_end: bool,
}

impl ScrollState {
    pub const fn new(follow_end: bool) -> Self {
        Self {
            scroll_top: 0,
            content_height: 0,
            viewport_height: 0,
            following_end: follow_end,
        }
    }

    pub const fn max_scroll_top(self) -> u16 {
        self.content_height.saturating_sub(self.viewport_height)
    }

    pub const fn update_layout(mut self, content_height: u16, viewport_height: u16) -> Self {
        self.content_height = content_height;
        self.viewport_height = viewport_height;
        let max = self.max_scroll_top();
        self.scroll_top = if self.following_end || self.scroll_top > max {
            max
        } else {
            self.scroll_top
        };
        if self.following_end && self.content_height <= self.viewport_height {
            self.scroll_top = 0;
        }
        self
    }

    pub const fn scroll_to(mut self, requested: u16) -> Self {
        let max = self.max_scroll_top();
        self.scroll_top = if requested < max { requested } else { max };
        self.following_end = self.scroll_top == max;
        self
    }

    pub const fn append_content(mut self, content_height: u16) -> Self {
        self.content_height = content_height;
        if self.following_end {
            self.scroll_top = self.max_scroll_top();
        }
        self
    }
}

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
        Element, HeaderViewProps, LayoutDirection, LayoutEntry, LayoutSize, ScrollState, Slot,
        StackLayout, StateOwner,
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

    #[test]
    fn scroll_state_follows_new_content_until_user_scrolls() {
        let state = ScrollState::new(true).update_layout(20, 5);
        assert_eq!(state.scroll_top, 15);
        let state = state.scroll_to(4);
        assert_eq!(state.scroll_top, 4);
        assert!(!state.following_end);
        let state = state.append_content(30);
        assert_eq!(state.scroll_top, 4);
        let state = state.scroll_to(25).append_content(40);
        assert_eq!(state.scroll_top, 35);
        assert!(state.following_end);
    }

    #[test]
    fn scroll_state_clamps_when_viewport_grows_or_content_shrinks() {
        let state = ScrollState::new(false)
            .update_layout(40, 10)
            .scroll_to(30)
            .update_layout(40, 20);
        assert_eq!(state.scroll_top, 20);
        let state = state.update_layout(8, 20);
        assert_eq!(state.scroll_top, 0);
        assert_eq!(state.max_scroll_top(), 0);
    }

    #[test]
    fn stack_allocator_resolves_basis_growth_and_shrink_without_renderer_state() {
        const ENTRIES: [LayoutEntry; 3] = [
            LayoutEntry {
                slot: Slot::Header,
                basis: LayoutSize::Fixed(1),
                grow: 0,
                shrink: 0,
                min_size: 1,
                max_size: Some(1),
            },
            LayoutEntry::grow(Slot::Scrollback, 2),
            LayoutEntry {
                slot: Slot::Status,
                basis: LayoutSize::Fixed(4),
                grow: 0,
                shrink: 1,
                min_size: 1,
                max_size: Some(4),
            },
        ];
        let stack = StackLayout {
            direction: LayoutDirection::Vertical,
            gap: 1,
            entries: &ENTRIES,
        };
        assert_eq!(stack.allocate(&[8, 8, 8], Some(20)), vec![1, 13, 4]);
        assert_eq!(stack.allocate(&[8, 8, 8], Some(8)), vec![1, 3, 2]);
        assert_eq!(stack.allocate(&[8, 8, 8], None), vec![1, 8, 4]);
    }
}
