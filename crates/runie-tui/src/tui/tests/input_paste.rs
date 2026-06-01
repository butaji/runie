//! Paste handling tests.
//!
//! Tests for paste behavior including:
//! - Single line paste
//! - Multi-line paste
//! - Paste in different modes (Permission, Overlay blocked)
//! - Paste with special characters

use crate::tui::state::{AppState, CommandPaletteState, Msg, TuiMode, TopBarState};
use crate::components::CommandPalette;
use crate::tui::update::update;
use ratatui_textarea::{TextArea, Input, Key};
use runie_ai::TokenUsage as AiTokenUsage;

fn make_state() -> AppState {
    AppState {
        messages: vec![],
        textarea: TextArea::default(),
        input_right_info: String::new(),
        mode: TuiMode::Chat,
        running: true,
        show_sidebar: false,
        agent_running: false,
        current_model: None,
        context: Default::default(),
        permission_modal: Default::default(),
        command_palette: CommandPaletteState::default(),
        scroll: Default::default(),
        animation: Default::default(),
        diff_viewer: None,
        token_usage: AiTokenUsage::default(),
        session_token_usage: AiTokenUsage::default(),
        session_tree: Default::default(),
        background_jobs: Vec::new(),
        onboarding: None,
        terminal_size: (0, 0),
        clear_input_confirm: Default::default(),
        model_picker: None,
        agent_start_time: None,
        input_history: Vec::new(),
        input_history_index: None,
        input_draft: String::new(),
        status_header: None,
        status_details: None,
        status_start_time: None,
        thinking: None,
        mock_mode: false,
        top_bar: TopBarState::default(),
        show_thoughts: false,
    }
}

// ─── Single Line Paste ────────────────────────────────────────────────────────

#[test]
fn test_paste_single_line() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Paste("hello".to_string()));

    let text = state.textarea.lines().join("\n");
    assert_eq!(text, "hello");
}

#[test]
fn test_paste_appends_to_existing() {
    let mut state = make_state();
    state.textarea = TextArea::new(vec!["existing".to_string()]);
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Paste(" more".to_string()));

    let text = state.textarea.lines().join("\n");
    assert_eq!(text, "existing more");
}

// ─── Multi-line Paste ─────────────────────────────────────────────────────────

#[test]
fn test_paste_multi_line() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Paste("line1\nline2\nline3".to_string()));

    let lines = state.textarea.lines();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], "line1");
    assert_eq!(lines[1], "line2");
    assert_eq!(lines[2], "line3");
}

#[test]
fn test_paste_preserves_exact_newlines() {
    let mut state = make_state();
    state.textarea = TextArea::new(vec!["start".to_string()]);
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Paste("\nmid\n".to_string()));

    let lines = state.textarea.lines();
    assert_eq!(lines.len(), 4);
    assert_eq!(lines[0], "start");
    assert_eq!(lines[1], "");
    assert_eq!(lines[2], "mid");
    assert_eq!(lines[3], "");
}

#[test]
fn test_paste_empty_string() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Paste("".to_string()));

    assert!(state.textarea.lines().join("\n").is_empty());
}

#[test]
fn test_paste_only_newlines() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Paste("\n\n\n".to_string()));

    let lines = state.textarea.lines();
    assert_eq!(lines.len(), 4);
    assert!(lines.iter().all(|l| l.is_empty()));
}

// ─── Paste in Blocking Modes ──────────────────────────────────────────────────

#[test]
fn test_paste_blocked_in_permission_mode() {
    let mut state = make_state();
    state.mode = TuiMode::Permission;
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Paste("test".to_string()));

    // Paste should not be processed in Permission mode
    assert!(state.textarea.lines().join("\n").is_empty());
}

#[test]
fn test_paste_blocked_in_overlay_mode() {
    let mut state = make_state();
    state.mode = TuiMode::Overlay;
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Paste("test".to_string()));

    assert!(state.textarea.lines().join("\n").is_empty());
}

#[test]
fn test_paste_allowed_in_chat_mode() {
    let mut state = make_state();
    state.mode = TuiMode::Chat;
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Paste("hello".to_string()));

    assert_eq!(state.textarea.lines().join("\n"), "hello");
}

#[test]
fn test_paste_allowed_in_diff_viewer_mode() {
    let mut state = make_state();
    state.mode = TuiMode::DiffViewer;
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Paste("test".to_string()));

    // DiffViewer is not a blocking mode for paste
    assert_eq!(state.textarea.lines().join("\n"), "test");
}

// ─── Paste with Special Characters ───────────────────────────────────────────

#[test]
fn test_paste_with_tabs() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Paste("col1\tcol2".to_string()));

    let text = state.textarea.lines().join("\n");
    assert!(text.contains('\t'));
}

#[test]
fn test_paste_with_trailing_whitespace() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Paste("text   ".to_string()));

    let text = state.textarea.lines().join("\n");
    assert_eq!(text, "text   ");
}

#[test]
fn test_paste_with_leading_whitespace() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Paste("   text".to_string()));

    let text = state.textarea.lines().join("\n");
    assert_eq!(text, "   text");
}

#[test]
fn test_paste_very_long_single_line() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();
    let long_text = "x".repeat(10000);

    update(&mut state, &mut palette, Msg::Paste(long_text.clone()));

    let text = state.textarea.lines().join("\n");
    assert_eq!(text.len(), 10000);
}

#[test]
fn test_paste_very_long_multi_line() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();
    let long_text = "line\n".repeat(1000);

    update(&mut state, &mut palette, Msg::Paste(long_text));

    assert_eq!(state.textarea.lines().len(), 1001); // 1000 \n + empty final
}

// ─── Paste Characters Inserted Correctly ──────────────────────────────────────

#[test]
fn test_paste_inserts_each_char() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Paste("abc".to_string()));

    // Verify by typing the same - should result in same textarea content
    let mut state2 = make_state();
    type_char(&mut state2, 'a');
    type_char(&mut state2, 'b');
    type_char(&mut state2, 'c');

    assert_eq!(state.textarea.lines().join("\n"), state2.textarea.lines().join("\n"));
}

fn type_char(state: &mut AppState, c: char) {
    state.textarea.input(Input { key: Key::Char(c), ctrl: false, alt: false, shift: false });
}

#[test]
fn test_paste_code_block() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    let code = r#"fn main() {
    println!("Hello");
}"#;
    update(&mut state, &mut palette, Msg::Paste(code.to_string()));

    let lines = state.textarea.lines();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], "fn main() {");
    assert_eq!(lines[1], "    println!(\"Hello\");");
    assert_eq!(lines[2], "}");
}

#[test]
fn test_paste_markdown_list() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    let markdown = "- item1\n- item2\n- item3";
    update(&mut state, &mut palette, Msg::Paste(markdown.to_string()));

    let lines = state.textarea.lines();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], "- item1");
    assert_eq!(lines[1], "- item2");
    assert_eq!(lines[2], "- item3");
}

// ─── Paste After Submit ────────────────────────────────────────────────────────

#[test]
fn test_paste_after_submit() {
    let mut state = make_state();
    state.current_model = Some("gpt-4".to_string());
    state.textarea = TextArea::new(vec!["Hello".to_string()]);
    let mut palette = CommandPalette::new();

    // Submit clears textarea
    update(&mut state, &mut palette, Msg::Submit);
    assert!(state.textarea.is_empty());

    // Paste after submit
    update(&mut state, &mut palette, Msg::Paste("world".to_string()));

    assert_eq!(state.textarea.lines().join("\n"), "world");
}

// ─── Paste Interaction with History ───────────────────────────────────────────

#[test]
fn test_paste_while_browsing_history() {
    let mut state = make_state();
    state.input_history = vec!["first".to_string(), "second".to_string()];
    let mut palette = CommandPalette::new();

    // Navigate to history
    update(&mut state, &mut palette, Msg::HistoryUp);
    assert_eq!(state.textarea.lines().join("\n"), "second");

    // Paste should work (and cancel history browsing)
    update(&mut state, &mut palette, Msg::Paste("pasted".to_string()));

    assert_eq!(state.textarea.lines().join("\n"), "pasted");
    // History navigation state might be affected
}
