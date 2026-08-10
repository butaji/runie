use super::{
    chat_view, chat_view_with_props, component, ChatViewProps, ComponentKind, Direction, Element,
    HeaderViewProps, LayoutDirection, LayoutEntry, LayoutSize, ScrollState, Slot, StackLayout,
    StateOwner,
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
fn chat_document_keeps_composition_and_ownership_separate() {
    let document = super::chat_document(ChatViewProps {
        command_palette_visible: true,
        ..ChatViewProps::default()
    });
    assert_eq!(document.components.len(), super::CHAT_COMPONENTS.len());
    assert!(document.props.chat.command_palette_visible);
    assert_eq!(document.props.header.meter, "");
    assert!(document.props.feed.is_empty());
    assert!(document.props.prompt.is_empty());
    assert_eq!(document.props.status.state, runie_tui_model::Status::Ready);
    assert_eq!(document.props.ui, runie_tui_model::UiState::new());
    assert!(document
        .root
        .slots()
        .any(|slot| slot == Slot::CommandPaletteOverlay));
    assert_eq!(
        component(Slot::CommandPaletteOverlay).owner,
        StateOwner::UiActor
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

#[test]
fn layout_entries_macro_expands_mixed_declarative_entries() {
    let entries = crate::layout_entries! {
        fixed(Slot::Header, 1),
        grow(Slot::Scrollback, 2),
        fixed(Slot::Status, 1),
    };
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].basis, LayoutSize::Fixed(1));
    assert_eq!(entries[1].grow, 1);
    assert_eq!(entries[1].min_size, 2);
}

#[test]
fn stack_allocator_respects_caps_and_missing_intrinsic_sizes() {
    let entries = [
        LayoutEntry {
            slot: Slot::Header,
            basis: LayoutSize::Auto,
            grow: 1,
            shrink: 1,
            min_size: 1,
            max_size: Some(2),
        },
        LayoutEntry::grow(Slot::Scrollback, 0),
    ];
    let stack = StackLayout {
        direction: LayoutDirection::Vertical,
        gap: 1,
        entries: &entries,
    };

    // The omitted second intrinsic size is zero, but the growable entry
    // still receives the remainder after the first entry reaches its cap.
    assert_eq!(stack.allocate(&[9], Some(6)), vec![2, 3]);
    assert_eq!(stack.allocate(&[], None), vec![1, 0]);
}
