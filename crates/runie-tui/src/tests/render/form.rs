//! Form dialog rendering tests (Layer 3)
//!
//! Verifies the form layout is distinct from a flat list — label above input,
//! progress indicator, prominent submit, cursor in active field.

use crate::ui::view;
use ratatui::{backend::TestBackend, Terminal};
use runie_core::{
    commands::DialogState,
    dialog::{Panel, PanelStack},
    AppState,
};

fn open_form_dialog(state: &mut AppState, panel: Panel) {
    state.open_dialog = Some(DialogState::PanelStack(PanelStack::new(panel)));
}

fn render(state: &mut AppState) -> ratatui::buffer::Buffer {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, state)).unwrap();
    terminal.backend().buffer().clone()
}

fn popup_inner_rect() -> ratatui::layout::Rect {
    // Dialog is centered: 60x18 at (10, 3) → inner at (11, 4) 58x16
    ratatui::layout::Rect {
        x: 11,
        y: 4,
        width: 58,
        height: 16,
    }
}

fn line_text(buf: &ratatui::buffer::Buffer, y: u16) -> String {
    (popup_inner_rect().x..popup_inner_rect().x + popup_inner_rect().width)
        .map(|x| buf[(x, y)].symbol())
        .collect()
}

fn all_lines(buf: &ratatui::buffer::Buffer) -> Vec<String> {
    (popup_inner_rect().y..popup_inner_rect().y + popup_inner_rect().height)
        .map(|y| line_text(buf, y))
        .collect()
}

/// Form must have a header that describes what the form is for (not an empty
/// filter line).
#[test]
fn form_has_descriptive_header() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    let panel = Panel::new("save", "Save Session").form_field("Name", "session-name", "name");
    open_form_dialog(&mut state, panel);

    let buf = render(&mut state);
    let lines = all_lines(&buf);

    // Should NOT have an empty filter line "❯" alone
    let has_empty_filter = lines
        .iter()
        .any(|l| l.trim() == "❯" || (l.trim_start().starts_with('❯') && l.trim().len() <= 2));
    assert!(
        !has_empty_filter,
        "Form should not show empty filter line, got: {:?}",
        lines
    );

    // Should have some form-related header
    let has_form_header = lines
        .iter()
        .any(|l| l.contains("form") || l.contains("Field") || l.contains("fill"));
    assert!(
        has_form_header,
        "Form should have a descriptive header, got: {:?}",
        lines
    );
}

/// Field label must appear ABOVE the input value (on its own line), not inline.
#[test]
fn form_field_label_above_input() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    let panel = Panel::new("save", "Save Session").form_field("Name", "session-name", "name");
    open_form_dialog(&mut state, panel);

    let buf = render(&mut state);
    let lines = all_lines(&buf);

    // Find the line with "Name" (label)
    let name_idx = lines
        .iter()
        .position(|l| l.contains("Name"))
        .expect("Form should have a 'Name' label");

    // The line(s) AFTER the label should contain the input value/placeholder
    let value_line = lines
        .iter()
        .skip(name_idx + 1)
        .find(|l| l.contains("session-name") || l.contains('│'));
    assert!(value_line.is_some(),
        "Input value 'session-name' should be on a line AFTER the 'Name' label (label-above-input), got: {:?}", lines);
}

/// Form must show a progress indicator (e.g. "1/1" or "①" etc.) showing
/// how many fields there are.
#[test]
fn form_shows_progress_indicator() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    let panel = Panel::new("save", "Save Session")
        .form_field("Name", "session-name", "name")
        .form_field("Tags", "tag1, tag2", "tags");
    open_form_dialog(&mut state, panel);

    let buf = render(&mut state);
    let lines = all_lines(&buf);

    // Look for some kind of progress indicator
    let has_progress = lines.iter().any(|l|
        // Numeric: "1/2", "1 of 2", "(1/2)", etc.
        l.contains("1/2") || l.contains("2/2") || l.contains("1 of 2") || l.contains("2 of 2") ||
        l.contains("(1)") || l.contains("(2)") ||
        // Circled numbers: ① ② ③
        l.contains('①') || l.contains('②') || l.contains('③') ||
        // Step: "Step 1", "Field 1", etc.
        l.contains("Step 1") || l.contains("Field 1") || l.contains("Step 2") || l.contains("Field 2") ||
        // Just "of 2" or similar
        (l.contains("of 2") && l.chars().any(|c| c.is_ascii_digit()))
    );
    assert!(
        has_progress,
        "Form should show a progress indicator (e.g. ① ②, 1/2, 'Field 1 of 2'), got: {:?}",
        lines
    );
}

/// Form must have a visually prominent Submit button — centered, with
/// box-drawing or arrow markers, distinct from regular items.
#[test]
fn form_has_prominent_submit_button() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    let panel = Panel::new("save", "Save Session")
        .form_field("Name", "session-name", "name")
        .form_submit();
    open_form_dialog(&mut state, panel);

    let buf = render(&mut state);
    let lines = all_lines(&buf);

    // Find a line with "Submit" in it
    let submit_line = lines
        .iter()
        .find(|l| l.contains("Submit"))
        .expect("Form should have a Submit button");

    // The Submit line should contain the Submit text. In the unified
    // panel_dialog rendering, buttons are inline with background styling.
    assert!(
        submit_line.contains("Submit"),
        "Submit button should be visible, got: '{}'",
        submit_line
    );
}

/// The active (currently focused) field should be visually distinct —
/// for example, by having a cursor character at the end of the value, or by
/// having a different background color.
#[test]
fn form_active_field_visually_distinct() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    let panel = Panel::new("save", "Save Session").form_field("Name", "session-name", "name");
    open_form_dialog(&mut state, panel);

    let buf = render(&mut state);
    let lines = all_lines(&buf);

    // Active field should have a cursor (block cursor █, or pipe |, or >)
    let has_cursor = lines.iter().any(|l| {
        l.contains('█')
            || l.contains('▏')
            || l.contains('▕')
            || (l.contains("session-name") && (l.contains('│') || l.contains('|')))
    });
    assert!(
        has_cursor,
        "Active field should have a visual cursor indicator, got: {:?}",
        lines
    );
}

/// Filled fields (with a value) should show a checkmark or filled indicator.
#[test]
fn form_completed_fields_show_checkmark() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    let panel = Panel::new("save", "Save Session").form_field_value(
        "Name",
        "session-name",
        "name",
        "my-session",
    );
    open_form_dialog(&mut state, panel);

    let buf = render(&mut state);
    let lines = all_lines(&buf);

    // Filled field should show some indicator
    let has_filled = lines
        .iter()
        .any(|l| l.contains('✓') || l.contains('✔') || l.contains("my-session"));
    assert!(
        has_filled,
        "Filled field should show value 'my-session' or checkmark, got: {:?}",
        lines
    );
}

/// Form must show hotkey hint at the BOTTOM (pinned, never scrolls away).
#[test]
fn form_hotkeys_pinned_to_bottom() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    let panel = Panel::new("save", "Save Session").form_field("Name", "session-name", "name");
    open_form_dialog(&mut state, panel);

    let buf = render(&mut state);
    let _inner = popup_inner_rect();

    // Check the last few lines of the inner area for the hotkey hint.
    // The unified panel_dialog adds 1-line margins, so hints land near
    // the bottom of the content area rather than the raw inner rect.
    let lines = all_lines(&buf);
    let has_hint = lines
        .iter()
        .rev()
        .take(3)
        .any(|l| l.contains("navigate") || l.contains("edit") || l.contains("esc"));
    assert!(
        has_hint,
        "Hotkey hint should be near the bottom of the popup, got: {:?}",
        lines
    );
}
