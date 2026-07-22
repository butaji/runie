//! Panel (command palette) list rendering tests (Layer 3)
//!
//! Verifies selected items get a full-width active background, inverted
//! high-contrast foreground for the command name, and a lower-contrast
//! style for the description.
#![allow(clippy::too_many_lines)]

use super::*;
use ratatui::style::{Color, Modifier};
use ratatui::{backend::TestBackend, Terminal};
use runie_core::{
    commands::DialogState,
    dialog::{Panel, PanelStack},
    event::Event,
    AppState,
};

use crate::theme::{color_accent, color_bg, color_bg_panel};
use crate::ui::view;

fn content_rect() -> ratatui::layout::Rect {
    // Popup is 60x18 centered in an 80x24 terminal → outer at (10,3).
    // Block borders shave 1 col/row; setup_popup adds another 1-cell margin.
    ratatui::layout::Rect { x: 12, y: 5, width: 56, height: 14 }
}

fn open_panel(state: &mut AppState, panel: Panel) {
    state.open_dialog = Some(DialogState::Active { kind: DialogKind::Generic, panels: PanelStack::new(panel) });
}

fn render(state: &mut AppState) -> ratatui::buffer::Buffer {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, state)).unwrap();
    terminal.backend().buffer().clone()
}

fn item_y(buf: &ratatui::buffer::Buffer, text: &str) -> Option<u16> {
    let r = content_rect();
    for y in r.y..r.y + r.height {
        let line: String = (r.x..r.x + r.width).map(|x| buf[(x, y)].symbol()).collect();
        if line.contains(text) {
            return Some(y);
        }
    }
    None
}

fn render_single_action(label: &str) -> (ratatui::buffer::Buffer, u16) {
    let mut state = AppState::default();
    let panel = Panel::new("cmds", "Commands").item(label, runie_core::dialog::ItemAction::Close);
    open_panel(&mut state, panel);
    let buf = render(&mut state);
    let y = item_y(&buf, label.split(' ').next().unwrap()).expect("should find action item");
    (buf, y)
}

fn assert_full_width_bg(buf: &ratatui::buffer::Buffer, y: u16, expected: Color, msg: &str) {
    let r = content_rect();
    for x in r.x..r.x + r.width {
        assert_eq!(buf[(x, y)].style().bg, Some(expected), "{} at x={}", msg, x);
    }
}

fn has_bg_on_line(buf: &ratatui::buffer::Buffer, y: u16, expected: Color) -> bool {
    let r = content_rect();
    (r.x..r.x + r.width).any(|x| buf[(x, y)].style().bg == Some(expected))
}

#[test]
fn selected_action_fills_full_width_with_active_bg() {
    let _lock = crate::theme::test_lock();
    let (buf, y) = render_single_action("new New conversation");
    assert_full_width_bg(
        &buf,
        y,
        color_accent(),
        "selected action background should fill whole panel width",
    );

    let below_y = y + 1;
    if below_y < content_rect().y + content_rect().height {
        assert!(
            has_bg_on_line(&buf, below_y, color_bg_panel()),
            "area below selected item should retain panel background"
        );
    }
}

fn find_symbol_x(buf: &ratatui::buffer::Buffer, y: u16, ch: char) -> Option<u16> {
    let r = content_rect();
    (r.x..r.x + r.width).find(|&x| buf[(x, y)].symbol() == ch.to_string())
}

fn assert_cell_style(
    buf: &ratatui::buffer::Buffer,
    x: u16,
    y: u16,
    fg: Option<Color>,
    bg: Option<Color>,
    bold: bool,
    msg: &str,
) {
    let cell = &buf[(x, y)];
    assert_eq!(cell.style().fg, fg, "{}", msg);
    assert_eq!(cell.style().bg, bg, "{}", msg);
    assert_eq!(
        cell.style().add_modifier.contains(Modifier::BOLD),
        bold,
        "{}",
        msg
    );
}

#[test]
fn selected_action_uses_inverted_bold_name_and_dim_description() {
    let _lock = crate::theme::test_lock();
    let (buf, y) = render_single_action("new New conversation");
    let _r = content_rect();

    let name_x = find_symbol_x(&buf, y, 'n').expect("should locate name 'new'");
    // sanity: next chars are "e", "w"
    assert_eq!(buf[(name_x + 1, y)].symbol(), "e");
    assert_eq!(buf[(name_x + 2, y)].symbol(), "w");

    assert_cell_style(
        &buf,
        name_x,
        y,
        Some(color_bg()),
        Some(color_accent()),
        true,
        "selected name should be inverted, bold, and on accent bg",
    );

    let desc_x = name_x + 3;
    assert_cell_style(
        &buf,
        desc_x,
        y,
        Some(color_bg_panel()),
        Some(color_accent()),
        false,
        "selected description should be inverted but lower-contrast",
    );
}

#[test]
fn selected_toggle_fills_full_width() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    let panel = Panel::new("settings", "Settings").toggle(
        "Read-only",
        false,
        runie_core::dialog::ItemAction::Toggle("read_only".into()),
    );
    open_panel(&mut state, panel);

    let buf = render(&mut state);
    let y = item_y(&buf, "Read-only").expect("should find toggle item");
    let accent = color_accent();
    let r = content_rect();

    for x in r.x..r.x + r.width {
        assert_eq!(
            buf[(x, y)].style().bg,
            Some(accent),
            "selected toggle background should fill whole panel width at x={}",
            x
        );
    }
}

#[test]
fn selected_select_fills_full_width_and_highlights_label() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    let panel = Panel::new("settings", "Settings").select(
        "Provider",
        "openai",
        vec!["openai".into(), "anthropic".into()],
        "provider",
    );
    open_panel(&mut state, panel);

    let buf = render(&mut state);
    let y = item_y(&buf, "Provider").expect("should find select item");
    let accent = color_accent();
    let bg = color_bg();
    let r = content_rect();

    for x in r.x..r.x + r.width {
        assert_eq!(
            buf[(x, y)].style().bg,
            Some(accent),
            "selected select background should fill whole panel width at x={}",
            x
        );
    }

    // The label should be bold and inverted.
    let label_x = (r.x + 2..r.x + r.width)
        .find(|x| buf[(*x, y)].symbol() == "P")
        .expect("find 'P'");
    let label_cell = &buf[(label_x, y)];
    assert_eq!(
        label_cell.style().fg,
        Some(bg),
        "selected select label should be inverted"
    );
    assert!(label_cell.style().add_modifier.contains(Modifier::BOLD));
}

#[test]
fn unselected_action_does_not_fill_full_width_with_active_bg() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    let panel = Panel::new("cmds", "Commands")
        .item("first First item", runie_core::dialog::ItemAction::Close)
        .item("second Second item", runie_core::dialog::ItemAction::Close);
    open_panel(&mut state, panel);
    // Move selection down to the second item so the first is unselected.
    if let Some(DialogState::Active { kind: DialogKind::Generic, panels: ref mut stack }) = state.open_dialog {
        stack.select_down();
    }

    let buf = render(&mut state);
    let y = item_y(&buf, "first").expect("should find first item");
    let accent = color_accent();
    let r = content_rect();

    let active_cells = (r.x..r.x + r.width)
        .filter(|x| buf[(*x, y)].style().bg == Some(accent))
        .count();
    assert!(
        active_cells < r.width as usize,
        "unselected item should not have active background across full width, got {} cells",
        active_cells
    );
}

fn bottom_hint_text(buf: &ratatui::buffer::Buffer) -> String {
    let r = content_rect();
    (r.y + r.height - 2..r.y + r.height)
        .flat_map(|y| (r.x..r.x + r.width).map(move |x| buf[(x, y)].symbol()))
        .collect()
}

#[test]
fn list_hint_shows_close_when_root_closable() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    let panel = Panel::new("cmds", "Commands")
        .closable(true)
        .item("Do thing", runie_core::dialog::ItemAction::Close);
    open_panel(&mut state, panel);

    let buf = render(&mut state);
    let hint = bottom_hint_text(&buf);
    assert!(
        hint.contains("esc") && hint.contains("close"),
        "closable root should show esc close hint, got: {}",
        hint
    );
}

#[test]
fn list_hint_omits_close_when_root_not_closable() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    let panel = Panel::new("cmds", "Commands")
        .non_closable()
        .item("Do thing", runie_core::dialog::ItemAction::Close);
    open_panel(&mut state, panel);

    let buf = render(&mut state);
    let hint = bottom_hint_text(&buf);
    assert!(
        !hint.contains("close"),
        "non-closable root should omit close hint, got: {}",
        hint
    );
}

#[test]
fn space_toggles_checkbox_render_state() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    let panel = Panel::new("settings", "Settings").toggle(
        "Read-only",
        false,
        runie_core::dialog::ItemAction::Toggle("read_only".into()),
    );
    open_panel(&mut state, panel);

    let before = render(&mut state);
    let y = item_y(&before, "Read-only").expect("toggle item should be rendered");
    let line_before: String = (content_rect().x..content_rect().x + content_rect().width)
        .map(|x| before[(x, y)].symbol())
        .collect();
    assert!(
        line_before.contains("[ ]"),
        "unchecked toggle should render [ ], got: {}",
        line_before
    );

    state.update(Event::Input(' '));

    let after = render(&mut state);
    let line_after: String = (content_rect().x..content_rect().x + content_rect().width)
        .map(|x| after[(x, y)].symbol())
        .collect();
    assert!(
        line_after.contains("[x]"),
        "checked toggle should render [x], got: {}",
        line_after
    );
}

#[test]
fn filter_chevron_is_accent_without_user_card_background() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    let panel = Panel::new("cmds", "Commands").item(
        "quit Quit application",
        runie_core::dialog::ItemAction::Close,
    );
    open_panel(&mut state, panel);

    let buf = render(&mut state);
    let r = content_rect();
    let y = (r.y..r.y + r.height)
        .find(|&y| (r.x..r.x + r.width).any(|x| buf[(x, y)].symbol() == "❯"))
        .expect("filter prompt row");
    let chevron_x = find_symbol_x(&buf, y, '❯').expect("filter chevron");
    let card_bg = crate::theme::color_bg_user();

    // No cell on the filter row may wear the user-card background.
    for cx in r.x..r.x + r.width {
        assert_ne!(
            buf[(cx, y)].style().bg,
            Some(card_bg),
            "filter row must not wear the user-card background (col {cx})"
        );
    }
    // The chevron itself uses the accent color, same as the input box.
    assert_eq!(
        buf[(chevron_x, y)].style().fg,
        Some(color_accent()),
        "filter chevron must use the accent color (same as the input box)"
    );
}

#[test]
fn popup_list_renders_selection() {
    // Verifies that the popup list renders the selected item with highlight style.
    // This produces the same visual output as a `List` widget with highlight style:
    // - Full-width background with accent color
    // - Inverted foreground (dark text on light background)
    // - Bold name, dimmed description
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();

    // Create a panel with multiple items
    let panel = Panel::new("cmds", "Commands")
        .item("help Show help", runie_core::dialog::ItemAction::Close)
        .item(
            "settings Configure settings",
            runie_core::dialog::ItemAction::Close,
        )
        .item(
            "quit Quit application",
            runie_core::dialog::ItemAction::Close,
        );
    open_panel(&mut state, panel);

    let buf = render(&mut state);
    let accent = color_accent();
    let bg = color_bg();
    let r = content_rect();

    // Find the first item (selected by default)
    let first_item_y = item_y(&buf, "help").expect("should find 'help' item");

    // Selected item should have accent background
    let selected_has_accent_bg = (r.x..r.x + r.width).any(|x| buf[(x, first_item_y)].style().bg == Some(accent));
    assert!(
        selected_has_accent_bg,
        "Selected item should have accent background color"
    );

    // Find the glyph prefix position
    let glyph_x = find_symbol_x(&buf, first_item_y, '▸').expect("should find selection glyph '▸'");

    // Glyph should be inverted (dark on accent)
    let glyph_cell = &buf[(glyph_x, first_item_y)];
    assert_eq!(
        glyph_cell.style().fg,
        Some(bg),
        "Selection glyph should be inverted (dark on accent bg)"
    );
    assert_eq!(
        glyph_cell.style().bg,
        Some(accent),
        "Selection glyph should have accent background"
    );

    // Name "help" should be bold and inverted
    let name_x = glyph_x + 1;
    let name_cell = &buf[(name_x, first_item_y)];
    assert_eq!(
        name_cell.style().fg,
        Some(bg),
        "Selected name should be inverted (dark on accent bg)"
    );
    assert_eq!(
        name_cell.style().bg,
        Some(accent),
        "Selected name should have accent background"
    );
    assert!(
        name_cell.style().add_modifier.contains(Modifier::BOLD),
        "Selected name should be bold"
    );

    // Description "Show help" should be dimmer (panel bg as fg)
    let desc_x = name_x + 4; // "help" is 4 chars
    let desc_cell = &buf[(desc_x, first_item_y)];
    let panel_bg = color_bg_panel();
    assert_eq!(
        desc_cell.style().fg,
        Some(panel_bg),
        "Selected description should use dimmed color (panel bg as fg)"
    );
    assert_eq!(
        desc_cell.style().bg,
        Some(accent),
        "Selected description should have accent background"
    );
}
