use ratatui::style::{Color, Style};

use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

fn key(code: KeyCode, mods: KeyModifiers) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: mods,
        kind: KeyEventKind::Press,
        state: crossterm::event::KeyEventState::NONE,
    }
}

#[test]
fn empty_enter_is_ignored() {
    let mut p = PromptWidget::new();
    let out = p.handle_key(key(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(out, PromptOutcome::Ignored);
}

#[test]
fn punctuation_emoticon_is_entered_as_prompt_text() {
    let mut p = PromptWidget::new();
    p.handle_key(key(KeyCode::Char(':'), KeyModifiers::NONE));
    p.handle_key(key(KeyCode::Char('('), KeyModifiers::SHIFT));
    assert_eq!(p.text(), ":(");
}

#[test]
fn ctrl_c_clears_non_empty_prompt() {
    let mut p = PromptWidget::new();
    p.handle_key(key(KeyCode::Char('x'), KeyModifiers::NONE));
    assert_eq!(
        p.handle_key(key(KeyCode::Char('c'), KeyModifiers::CONTROL)),
        PromptOutcome::Edited
    );
    assert!(p.is_empty());
}

#[test]
fn mode_cycles_through_normal_alternate_and_plan() {
    let mut p = PromptWidget::new();
    assert_eq!(p.mode(), InputMode::Normal);
    p.cycle_mode();
    assert_eq!(p.mode(), InputMode::Alternate);
    p.cycle_mode();
    assert_eq!(p.mode(), InputMode::Plan);
    p.cycle_mode();
    assert_eq!(p.mode(), InputMode::Normal);
}

#[test]
fn plan_mode_uses_gold_prompt_border_and_caption() {
    let mut p = PromptWidget::new();
    p.cycle_mode();
    p.cycle_mode();
    let mut buffer = Buffer::empty(Rect::new(0, 0, 40, 4));
    p.clone().render(Rect::new(0, 0, 40, 4), &mut buffer);
    assert_eq!(
        buffer.cell((0, 0)).expect("top border").fg,
        appearance::warning_style().fg.expect("warning token")
    );
    let text: String = (0..40)
        .map(|x| buffer.cell((x, 3)).expect("caption row").symbol())
        .collect();
    assert!(text.contains("plan"));
}

#[tokio::test]
async fn file_search_mode_is_owned_by_prompt_and_esc_exits_it() {
    let mut p = PromptWidget::new();
    p.open_file_search_async().await;
    assert!(p.file_search_active());
    assert_eq!(
        p.handle_key(key(KeyCode::Esc, KeyModifiers::NONE)),
        PromptOutcome::Edited
    );
    assert!(!p.file_search_active());
}

#[tokio::test]
async fn async_file_search_keeps_filesystem_work_out_of_sync_reducer() {
    let mut p = PromptWidget::new();
    p.open_file_search_async().await;
    assert!(p.file_search_active());
    assert!(p.file_candidates.iter().any(|name| name == "Cargo.toml"));
}

#[tokio::test]
async fn file_search_accepts_a_selected_candidate() {
    let mut p = PromptWidget::new();
    p.open_file_search_async().await;
    assert!(!p.file_candidates.is_empty());
    let expected = p.file_matches()[0].clone();
    assert_eq!(
        p.handle_key(key(KeyCode::Tab, KeyModifiers::NONE)),
        PromptOutcome::Edited
    );
    assert_eq!(p.text(), expected);
    assert!(!p.file_search_active());
}

#[tokio::test]
async fn file_dialog_arrow_navigation_wraps_at_both_boundaries() {
    let mut p = PromptWidget::new();
    p.open_file_search_async().await;
    let count = p.file_matches().len();
    assert!(
        count > 1,
        "fixture workspace needs multiple file candidates"
    );
    p.handle_key(key(KeyCode::Up, KeyModifiers::NONE));
    assert_eq!(p.file_candidate_index, count - 1);
    p.handle_key(key(KeyCode::Down, KeyModifiers::NONE));
    assert_eq!(p.file_candidate_index, 0);
}

#[tokio::test]
async fn file_search_hands_selected_file_to_bounded_viewer() {
    let mut p = PromptWidget::new();
    p.selected_file = Some("Cargo.toml".into());
    p.open_file_search_async().await;
    assert!(p.file_viewer_active());
    assert!(!p.viewer_lines.is_empty());
    assert!(p.render_height() >= 2);
    assert_eq!(
        p.handle_key(key(KeyCode::Esc, KeyModifiers::NONE)),
        PromptOutcome::Edited
    );
    assert!(!p.file_viewer_active());
}

#[test]
fn multiline_chrome_is_visible() {
    let mut p = PromptWidget::new();
    p.handle_key(key(KeyCode::Enter, KeyModifiers::SHIFT));
    let area = Rect::new(0, 0, 60, 4);
    let mut buffer = Buffer::empty(area);
    Widget::render(p, area, &mut buffer);
    let row = (0..area.width)
        .map(|x| buffer.cell((x, 3)).expect("caption cell").symbol())
        .collect::<String>();
    assert!(row.contains("multiline"));
}

#[test]
fn model_caption_is_read_only_projection_input() {
    let mut p = PromptWidget::new();
    p.set_model_caption("test-model (high)");
    let area = Rect::new(0, 0, 60, 4);
    let mut buffer = Buffer::empty(area);
    Widget::render(p, area, &mut buffer);
    let row = (0..area.width)
        .map(|x| buffer.cell((x, 3)).expect("caption cell").symbol())
        .collect::<String>();
    assert!(row.contains("test-model (high)"));
}

#[test]
fn model_caption_uses_grok_semantic_segments() {
    let mut prompt = PromptWidget::new();
    prompt.set_model_caption("Grok 4.5 (high) · always-approve");
    let area = Rect::new(0, 0, 80, 4);
    let mut buffer = Buffer::empty(area);
    Widget::render(prompt, area, &mut buffer);
    let row = (0..area.width)
        .map(|x| buffer.cell((x, 3)).expect("caption cell").symbol())
        .collect::<String>();
    let model_start = find_caption_text(&row, "Grok 4.5 (high)");
    let separator_start = find_caption_text(&row, " · ");
    assert_cell_color(
        &buffer,
        model_start,
        appearance::model_caption_style_for(ThemeKind::GrokNight),
    );
    assert_cell_color(
        &buffer,
        separator_start,
        appearance::header_path_style_for(ThemeKind::GrokNight),
    );
    assert_cell_color(
        &buffer,
        separator_start + 3,
        appearance::muted_style_for(ThemeKind::GrokNight),
    );
}

fn find_caption_text(row: &str, text: &str) -> u16 {
    row.chars()
        .collect::<Vec<_>>()
        .windows(text.chars().count())
        .position(|window| window == text.chars().collect::<Vec<_>>())
        .expect("caption text") as u16
}

fn assert_cell_color(buffer: &Buffer, x: u16, style: Style) {
    assert_eq!(
        buffer.cell((x, 3)).expect("caption style").fg,
        style.fg.expect("caption token")
    );
}

#[test]
fn renderer_adapter_preserves_prompt_projection_fields() {
    let mut source = PromptWidget::new();
    source.set_model_caption("adapter-model");
    source.handle_key(key(KeyCode::Char('x'), KeyModifiers::NONE));
    source.cycle_mode();
    source.push_history("previous");
    let snapshot = source.model_snapshot();
    let adapted = PromptWidget::from_model_snapshot(snapshot.clone());
    assert_eq!(adapted.model_snapshot(), snapshot);
}

#[test]
fn history_chrome_is_visible_while_browsing() {
    let mut p = PromptWidget::new();
    p.push_history("previous");
    p.handle_key(key(KeyCode::Up, KeyModifiers::NONE));
    let area = Rect::new(0, 0, 60, 4);
    let mut buffer = Buffer::empty(area);
    Widget::render(p, area, &mut buffer);
    let row = (0..area.width)
        .map(|x| buffer.cell((x, 3)).expect("caption cell").symbol())
        .collect::<String>();
    assert!(row.contains("history"));
}

#[test]
fn history_command_enters_search_and_filters() {
    let mut p = PromptWidget::new();
    p.push_history("alpha file");
    p.push_history("beta note");
    for ch in "/history".chars() {
        p.handle_key(key(KeyCode::Char(ch), KeyModifiers::NONE));
    }
    assert!(p.history_search_active());
    p.handle_key(key(KeyCode::Char('f'), KeyModifiers::NONE));
    p.handle_key(key(KeyCode::Up, KeyModifiers::NONE));
    assert_eq!(p.text(), "alpha file");
}

#[test]
fn char_then_enter_submits() {
    let mut p = PromptWidget::new();
    p.handle_key(key(KeyCode::Char('h'), KeyModifiers::NONE));
    p.handle_key(key(KeyCode::Char('i'), KeyModifiers::NONE));
    let out = p.handle_key(key(KeyCode::Enter, KeyModifiers::NONE));
    match out {
        PromptOutcome::Submitted(s) => assert_eq!(s, "hi"),
        other => panic!("expected Submitted, got {other:?}"),
    }
    assert!(p.is_empty());
}

#[test]
fn backspace_pops_last_char() {
    let mut p = PromptWidget::new();
    p.handle_key(key(KeyCode::Char('a'), KeyModifiers::NONE));
    p.handle_key(key(KeyCode::Char('b'), KeyModifiers::NONE));
    p.handle_key(key(KeyCode::Backspace, KeyModifiers::NONE));
    assert_eq!(p.text(), "a");
}

#[test]
fn shift_alt_enter_inserts_newline_instead_of_submitting() {
    let mut p = PromptWidget::new();
    p.handle_key(key(KeyCode::Char('a'), KeyModifiers::NONE));
    let out = p.handle_key(key(KeyCode::Enter, KeyModifiers::SHIFT));
    assert_eq!(out, PromptOutcome::Edited);
    assert_eq!(p.text(), "a\n");
    // Bare Enter still submits.
    p.handle_key(key(KeyCode::Char('b'), KeyModifiers::NONE));
    let out = p.handle_key(key(KeyCode::Enter, KeyModifiers::NONE));
    assert!(matches!(out, PromptOutcome::Submitted(_)));
}

#[test]
fn multiline_prompt_renders_each_line_with_one_gutter_prefix() {
    let mut p = PromptWidget::new();
    p.buffer = "first\nsecond".into();
    let lines = p.input_lines();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].to_string(), " ❯ first");
    assert_eq!(lines[1].to_string(), "   second");
}

#[test]
fn submitted_prompts_are_recorded_in_history() {
    let mut p = PromptWidget::new();
    for s in ["one", "two", "two"] {
        for ch in s.chars() {
            p.handle_key(key(KeyCode::Char(ch), KeyModifiers::NONE));
        }
        p.handle_key(key(KeyCode::Enter, KeyModifiers::NONE));
    }
    // Consecutive duplicate deduped, newest last.
    assert_eq!(p.history(), &["one".to_string(), "two".to_string()]);
}

#[test]
fn up_arrow_recalls_history_then_down_clears() {
    let mut p = PromptWidget::new();
    for s in ["alpha", "beta"] {
        for ch in s.chars() {
            p.handle_key(key(KeyCode::Char(ch), KeyModifiers::NONE));
        }
        p.handle_key(key(KeyCode::Enter, KeyModifiers::NONE));
    }
    p.handle_key(key(KeyCode::Up, KeyModifiers::NONE));
    assert_eq!(p.text(), "beta");
    p.handle_key(key(KeyCode::Up, KeyModifiers::NONE));
    assert_eq!(p.text(), "alpha");
    p.handle_key(key(KeyCode::Down, KeyModifiers::NONE));
    assert_eq!(p.text(), "beta");
    p.handle_key(key(KeyCode::Down, KeyModifiers::NONE));
    assert!(p.text().is_empty(), "down past newest clears the buffer");
}

#[test]
fn empty_prompt_uses_bare_cursor_glyph() {
    let p = PromptWidget::new();
    let mut buffer = Buffer::empty(Rect {
        x: 0,
        y: 0,
        width: 30,
        height: 3,
    });
    p.clone().render(
        Rect {
            x: 0,
            y: 0,
            width: 30,
            height: 3,
        },
        &mut buffer,
    );
    assert_eq!(buffer.cell((2, 1)).expect("cursor cell").symbol(), "❯");
    assert_eq!(
        buffer.cell((2, 1)).expect("cursor cell").fg,
        Color::Rgb(225, 225, 225)
    );
    let row = (0..30)
        .map(|x| buffer.cell((x, 1)).expect("prompt cell").symbol())
        .collect::<String>();
    assert!(row.contains('T'), "placeholder row: {row:?}");
}

#[test]
fn prompt_theme_projects_day_tokens() {
    let mut prompt = PromptWidget::new();
    prompt.set_theme(ThemeKind::GrokDay);
    let mut buffer = Buffer::empty(Rect {
        x: 0,
        y: 0,
        width: 30,
        height: 3,
    });
    prompt.render(
        Rect {
            x: 0,
            y: 0,
            width: 30,
            height: 3,
        },
        &mut buffer,
    );
    assert_eq!(
        buffer.cell((2, 1)).expect("cursor cell").fg,
        Color::Rgb(38, 38, 38)
    );
}

#[test]
fn cursor_position_counts_unicode_display_width() {
    let mut p = PromptWidget::new();
    p.handle_key(key(KeyCode::Char('界'), KeyModifiers::NONE));
    let pos = p.cursor_position(Rect {
        x: 4,
        y: 7,
        width: 20,
        height: 3,
    });
    assert_eq!(pos, ratatui::layout::Position::new(10, 8));
}

#[test]
fn test_backend_receives_prompt_cursor_position() {
    use ratatui::backend::Backend;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    let mut prompt = PromptWidget::new();
    prompt.handle_key(key(KeyCode::Char('a'), KeyModifiers::NONE));
    let mut terminal = Terminal::new(TestBackend::new(20, 4)).expect("terminal");
    terminal
        .draw(|frame| {
            let area = frame.area();
            frame.render_widget(prompt.clone(), area);
            frame.set_cursor_position(prompt.cursor_position(area));
        })
        .expect("draw prompt");
    assert_eq!(
        terminal
            .backend_mut()
            .get_cursor_position()
            .expect("cursor"),
        ratatui::layout::Position::new(5, 1)
    );
}
