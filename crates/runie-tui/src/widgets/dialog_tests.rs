use super::*;
use crate::appearance;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use runie_tui_model::{
    CHANGELOG_DIALOG, COMMAND_DIALOG, FILE_SELECTOR_DIALOG, MODEL_SELECTOR_DIALOG,
    PALETTE_PARAMETERS_DIALOG, SESSION_DIALOG, SHORTCUTS_DIALOG, THEME_SELECTOR_DIALOG,
};

fn rendered_text(buffer: &Buffer) -> String {
    let area = buffer.area;
    (area.y..area.bottom())
        .flat_map(|y| (area.x..area.right()).map(move |x| buffer.cell((x, y)).unwrap().symbol()))
        .collect()
}

#[test]
fn grok_panel_renders_title_query_selection_and_actions() {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 100, 30));
    let frame = DialogFrame {
        spec: COMMAND_DIALOG,
        query: "model".into(),
        selected: 0,
    };
    DialogWidget::new(frame, vec!["Switch model".into()]).render(buffer.area, &mut buffer);
    let text = rendered_text(&buffer);
    assert!(text.contains("Commands"));
    assert!(text.contains("search: model"));
    assert!(text.contains("  Switch model"));
    assert!(text.contains("↑/↓:nav"));
}

#[test]
fn panel_is_suppressed_when_terminal_is_too_narrow() {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 20, 8));
    DialogWidget::new(
        DialogFrame {
            spec: COMMAND_DIALOG,
            query: String::new(),
            selected: 0,
        },
        vec!["New Session".into()],
    )
    .render(buffer.area, &mut buffer);
    assert!(rendered_text(&buffer).trim().is_empty());
}

#[test]
fn panel_geometry_is_stable_across_reference_terminal_sizes() {
    for (width, height) in [(80, 24), (100, 30), (64, 20), (120, 40)] {
        let mut buffer = Buffer::empty(Rect::new(0, 0, width, height));
        DialogWidget::new(
            DialogFrame {
                spec: COMMAND_DIALOG,
                query: String::new(),
                selected: 0,
            },
            vec!["New Session".into(), "Keyboard Shortcuts".into()],
        )
        .render(buffer.area, &mut buffer);
        let text = rendered_text(&buffer);
        assert!(
            text.contains("Commands"),
            "missing title at {width}x{height}"
        );
        assert!(
            text.contains("  New Session"),
            "missing selection at {width}x{height}"
        );
    }
}

#[test]
fn panel_uses_grok_modal_sizing_constants() {
    assert_eq!(
        centered_panel(Rect::new(0, 0, 120, 36), 10),
        Some(Rect::new(30, 9, 60, 17))
    );
    assert_eq!(
        centered_panel(Rect::new(0, 0, 80, 24), 10),
        Some(Rect::new(18, 4, 44, 16))
    );
    assert_eq!(
        centered_panel(Rect::new(0, 0, 100, 30), 10),
        Some(Rect::new(25, 6, 50, 17))
    );
    assert_eq!(
        centered_panel(Rect::new(0, 0, 64, 20), 10),
        Some(Rect::new(10, 4, 44, 12))
    );
}

#[test]
fn panel_matches_grok_chrome_and_bottom_footer() {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 80, 24));
    DialogWidget::new(
        DialogFrame {
            spec: COMMAND_DIALOG,
            query: String::new(),
            selected: 0,
        },
        vec!["New Session".into()],
    )
    .render(buffer.area, &mut buffer);
    let panel = centered_panel(buffer.area, 1).unwrap();
    assert_eq!(buffer.cell((panel.x, panel.y)).unwrap().symbol(), "┌");
    assert!(rendered_text(&buffer).contains("[✗]"));
    assert_eq!(
        buffer
            .cell((panel.x + panel.width / 2, panel.y + panel.height - 3))
            .unwrap()
            .symbol(),
        " "
    );
    assert!(rendered_text(&buffer).contains("↑/↓:nav  │  Enter:select  │  Esc:close"));
}

#[test]
fn panel_surface_paints_background_through_blank_rows() {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 80, 24));
    let frame = DialogFrame {
        spec: COMMAND_DIALOG,
        query: String::new(),
        selected: 0,
    };
    DialogWidget::new(frame, vec![]).render(buffer.area, &mut buffer);
    let panel = centered_panel(buffer.area, 0).unwrap();
    let cell = buffer
        .cell((panel.x + 2, panel.y + panel.height / 2))
        .unwrap();
    assert_eq!(
        cell.bg,
        appearance::background_style_for(runie_core::types::ThemeKind::GrokNight)
            .bg
            .unwrap()
    );
}

#[test]
fn panel_border_uses_the_app_prompt_border_color() {
    let theme = runie_core::types::ThemeKind::GrokNight;
    assert_eq!(
        dialog_border_style(theme).fg,
        appearance::prompt_border_style_for(theme).fg
    );
}

#[test]
fn panel_title_is_bold() {
    let style =
        dialog_border_style(runie_core::types::ThemeKind::GrokNight).add_modifier(Modifier::BOLD);
    assert!(style.add_modifier.contains(Modifier::BOLD));
}

#[test]
fn every_dialog_has_app_styled_hotkeys_on_its_bottom_row() {
    let theme = runie_core::types::ThemeKind::GrokNight;
    let expected = appearance::footer_hotkey_span(theme, "key").style;
    for spec in [
        COMMAND_DIALOG,
        FILE_SELECTOR_DIALOG,
        MODEL_SELECTOR_DIALOG,
        SHORTCUTS_DIALOG,
        SESSION_DIALOG,
        CHANGELOG_DIALOG,
        PALETTE_PARAMETERS_DIALOG,
        THEME_SELECTOR_DIALOG,
    ] {
        assert_dialog_hotkeys(spec, theme, expected.fg.unwrap_or_default());
    }
}

fn assert_dialog_hotkeys(
    spec: runie_tui_model::DialogSpec,
    theme: runie_core::types::ThemeKind,
    expected_fg: ratatui::style::Color,
) {
    assert!(
        !spec.actions.is_empty(),
        "{} has no footer actions",
        spec.id
    );
    assert!(
        spec.actions.iter().all(|action| action.hotkey.is_some()),
        "{} has an action without a styled hotkey",
        spec.id
    );
    let rows = (0..20).map(|index| format!("row-{index:02}")).collect();
    let mut buffer = Buffer::empty(Rect::new(0, 0, 100, 30));
    DialogWidget::new(
        DialogFrame {
            spec: spec.clone(),
            query: String::new(),
            selected: 19,
        },
        rows,
    )
    .with_theme(theme)
    .render(buffer.area, &mut buffer);
    assert_dialog_hotkey_cell(&buffer, &spec, expected_fg);
}

fn assert_dialog_hotkey_cell(
    buffer: &Buffer,
    spec: &runie_tui_model::DialogSpec,
    expected_fg: ratatui::style::Color,
) {
    let panel = centered_panel(buffer.area, 20).expect("dialog panel");
    let footer_y = panel.y + panel.height - 2;
    let key = spec.actions[0].hotkey.expect("first hotkey");
    let key_x = (panel.x + 1..panel.x + panel.width - 1)
        .find(|x| {
            buffer.cell((*x, footer_y)).expect("footer cell").symbol()
                == key.chars().next().unwrap().to_string()
        })
        .unwrap_or_else(|| panic!("{} footer is not on the bottom row", spec.id));
    let cell = buffer.cell((key_x, footer_y)).expect("hotkey cell");
    assert_eq!(cell.fg, expected_fg, "{} hotkey color", spec.id);
    assert!(
        cell.modifier.contains(Modifier::BOLD),
        "{} hotkey weight",
        spec.id
    );
    assert!(
        rendered_text(buffer).contains("row-19"),
        "{} did not scroll selection into view",
        spec.id
    );
}
