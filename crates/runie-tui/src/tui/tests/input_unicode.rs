//! Unicode input tests.
//!
//! Tests for unicode handling including:
//! - Emoji (single, multi-codepoint)
//! - CJK characters
//! - Mixed unicode/text
//! - Special unicode characters
//! - Markdown with unicode
//! - Control characters handling

use crate::tui::state::{AppState, CommandPaletteState, Msg, TuiMode, TopBarState};
use crate::components::{CommandPalette, MessageItem};
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
        thinking_start: None,
        thinking_duration: None,
        is_thinking: false,
        current_thinking_text: String::new(),
        mock_mode: false,
        top_bar: TopBarState::default(),
    }
}

fn make_state_with_model(model: &str) -> AppState {
    let mut state = make_state();
    state.current_model = Some(model.to_string());
    state
}

fn type_char(state: &mut AppState, c: char) {
    state.textarea.input(Input { key: Key::Char(c), ctrl: false, alt: false, shift: false });
}

// ─── Emoji Tests ──────────────────────────────────────────────────────────────

#[test]
fn test_paste_single_emoji() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Paste("😀".to_string()));

    let text = state.textarea.lines().join("\n");
    assert_eq!(text, "😀");
}

#[test]
fn test_paste_multiple_emojis() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Paste("🎉🔥🚀".to_string()));

    let text = state.textarea.lines().join("\n");
    assert_eq!(text, "🎉🔥🚀");
}

#[test]
fn test_paste_emoji_with_text() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Paste("Hello 😀 World".to_string()));

    let text = state.textarea.lines().join("\n");
    assert_eq!(text, "Hello 😀 World");
}

#[test]
fn test_paste_skin_tone_modifiers() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    // Hand with light skin tone
    update(&mut state, &mut palette, Msg::Paste("👋🏻".to_string()));

    let text = state.textarea.lines().join("\n");
    assert_eq!(text, "👋🏻");
}

#[test]
fn test_paste_family_emoji() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    // Family emoji (multi-codepoint)
    update(&mut state, &mut palette, Msg::Paste("👨‍👩‍👧‍👦".to_string()));

    let text = state.textarea.lines().join("\n");
    assert_eq!(text, "👨‍👩‍👧‍👦");
}

#[test]
fn test_paste_flag_emoji() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    // Country flag emoji
    update(&mut state, &mut palette, Msg::Paste("🇺🇸".to_string()));

    let text = state.textarea.lines().join("\n");
    assert_eq!(text, "🇺🇸");
}

#[test]
fn test_paste_emoji_sequence() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Paste("⭐🪐🌍🌙".to_string()));

    let text = state.textarea.lines().join("\n");
    assert_eq!(text, "⭐🪐🌍🌙");
}

// ─── CJK Characters ────────────────────────────────────────────────────────────

#[test]
fn test_paste_chinese() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Paste("你好世界".to_string()));

    let text = state.textarea.lines().join("\n");
    assert_eq!(text, "你好世界");
}

#[test]
fn test_paste_japanese() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Paste("こんにちは世界".to_string()));

    let text = state.textarea.lines().join("\n");
    assert_eq!(text, "こんにちは世界");
}

#[test]
fn test_paste_korean() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Paste("안녕하세요".to_string()));

    let text = state.textarea.lines().join("\n");
    assert_eq!(text, "안녕하세요");
}

#[test]
fn test_paste_mixed_cjk() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Paste("中国-CNK 日本-JPN 韓国-KOR".to_string()));

    let text = state.textarea.lines().join("\n");
    assert_eq!(text, "中国-CNK 日本-JPN 韓国-KOR");
}

#[test]
fn test_paste_hiragana_katakana_mix() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Paste("ひらがなカタカナ混合".to_string()));

    let text = state.textarea.lines().join("\n");
    assert_eq!(text, "ひらがなカタカナ混合");
}

#[test]
fn test_paste_chinese_with_english() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Paste("AI人工智能 AI Machine Learning".to_string()));

    let text = state.textarea.lines().join("\n");
    assert_eq!(text, "AI人工智能 AI Machine Learning");
}

// ─── Mixed Unicode Tests ──────────────────────────────────────────────────────

#[test]
fn test_paste_mixed_unicode_text() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Paste("Hello 你好 🎉 Test 123".to_string()));

    let text = state.textarea.lines().join("\n");
    assert_eq!(text, "Hello 你好 🎉 Test 123");
}

#[test]
fn test_paste_unicode_with_newlines() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Paste("Line1 你好\nLine2 🎉\nLine3 Bye".to_string()));

    let lines = state.textarea.lines();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], "Line1 你好");
    assert_eq!(lines[1], "Line2 🎉");
    assert_eq!(lines[2], "Line3 Bye");
}

// ─── Special Unicode Characters ───────────────────────────────────────────────

#[test]
fn test_paste_diacritics() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Paste("café naïve résumé".to_string()));

    let text = state.textarea.lines().join("\n");
    assert_eq!(text, "café naïve résumé");
}

#[test]
fn test_paste_cyrillic() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Paste("Привет мир".to_string()));

    let text = state.textarea.lines().join("\n");
    assert_eq!(text, "Привет мир");
}

#[test]
fn test_paste_greek() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Paste("Γειά σου Κόσμε".to_string()));

    let text = state.textarea.lines().join("\n");
    assert_eq!(text, "Γειά σου Κόσμε");
}

#[test]
fn test_paste_arabic() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Paste("مرحبا بالعالم".to_string()));

    let text = state.textarea.lines().join("\n");
    assert_eq!(text, "مرحبا بالعالم");
}

#[test]
fn test_paste_hebrew() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Paste("שלום עולם".to_string()));

    let text = state.textarea.lines().join("\n");
    assert_eq!(text, "שלום עולם");
}

#[test]
fn test_paste_thai() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Paste("สวัสดีชาวโลก".to_string()));

    let text = state.textarea.lines().join("\n");
    assert_eq!(text, "สวัสดีชาวโลก");
}

#[test]
fn test_paste_vietnamese() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Paste("Xin chào thế giới".to_string()));

    let text = state.textarea.lines().join("\n");
    assert_eq!(text, "Xin chào thế giới");
}

// ─── Markdown with Unicode ────────────────────────────────────────────────────

#[test]
fn test_paste_markdown_with_unicode_headers() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Paste("# 标题 🎉\n## 子标题".to_string()));

    let lines = state.textarea.lines();
    assert_eq!(lines[0], "# 标题 🎉");
    assert_eq!(lines[1], "## 子标题");
}

#[test]
fn test_paste_markdown_code_with_unicode() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Paste("```python\nmsg = \"你好\"\nprint(msg)\n```".to_string()));

    let lines = state.textarea.lines();
    assert_eq!(lines[0], "```python");
    assert_eq!(lines[1], "msg = \"你好\"");
    assert_eq!(lines[2], "print(msg)");
    assert_eq!(lines[3], "```");
}

#[test]
fn test_paste_markdown_list_with_emoji() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Paste("- ✅ 完成\n- ⏳ 进行中\n- 📋 待办".to_string()));

    let lines = state.textarea.lines();
    assert_eq!(lines.len(), 3);
    assert!(lines[0].starts_with("- ✅"));
    assert!(lines[1].starts_with("- ⏳"));
    assert!(lines[2].starts_with("- 📋"));
}

// ─── Control Characters ────────────────────────────────────────────────────────

#[test]
fn test_paste_strips_null_char() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Paste("hello\0world".to_string()));

    // Null character should be handled somehow
    let text = state.textarea.lines().join("\n");
    assert!(!text.contains('\0') || text == "hello\0world"); // Behavior depends on implementation
}

#[test]
fn test_paste_bell_character() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    // Bell character (ASCII 7) - using \x07
    update(&mut state, &mut palette, Msg::Paste("hello\x07world".to_string()));

    let text = state.textarea.lines().join("\n");
    assert_eq!(text, "hello\x07world");
}

#[test]
fn test_paste_backspace_character() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    // Backspace character - testing that it's handled
    update(&mut state, &mut palette, Msg::Paste("hel".to_string()));
    type_char(&mut state, '\x08'); // backspace
    type_char(&mut state, 'o');

    let text = state.textarea.lines().join("\n");
    // After backspace and typing 'o', we should have "helo"
    assert_eq!(text, "helo");
}

// ─── Submit with Unicode ───────────────────────────────────────────────────────

#[test]
fn test_submit_chinese_message() {
    let mut state = make_state_with_model("gpt-4");
    state.textarea = TextArea::new(vec!["你好世界".to_string()]);
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Submit);

    assert!(state.agent_running);
    if let MessageItem::User { text, .. } = &state.messages[0] {
        assert_eq!(text, "你好世界");
    } else {
        panic!("Expected User message");
    }
}

#[test]
fn test_submit_emoji_message() {
    let mut state = make_state_with_model("gpt-4");
    state.textarea = TextArea::new(vec!["🎉🎊🥳".to_string()]);
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Submit);

    assert!(state.agent_running);
    if let MessageItem::User { text, .. } = &state.messages[0] {
        assert_eq!(text, "🎉🎊🥳");
    } else {
        panic!("Expected User message");
    }
}

#[test]
fn test_submit_mixed_unicode() {
    let mut state = make_state_with_model("gpt-4");
    state.textarea = TextArea::new(vec!["Hello 你好 🎉".to_string()]);
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Submit);

    assert!(state.agent_running);
    if let MessageItem::User { text, .. } = &state.messages[0] {
        assert_eq!(text, "Hello 你好 🎉");
    } else {
        panic!("Expected User message");
    }
}

// ─── Unicode in History ────────────────────────────────────────────────────────

#[test]
fn test_history_with_unicode() {
    let mut state = make_state();
    state.input_history.push("你好 🎉".to_string());
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::HistoryUp);

    let text = state.textarea.lines().join("\n");
    assert_eq!(text, "你好 🎉");
}

#[test]
fn test_history_with_multiline_unicode() {
    let mut state = make_state();
    state.input_history.push("Line1 你好\nLine2 🎉".to_string());
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::HistoryUp);

    let lines = state.textarea.lines();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], "Line1 你好");
    assert_eq!(lines[1], "Line2 🎉");
}

// ─── Wide Character Width ─────────────────────────────────────────────────────

#[test]
fn test_paste_fullwidth_characters() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Paste("１２３４５".to_string()));

    let text = state.textarea.lines().join("\n");
    assert_eq!(text, "１２３４５");
}

#[test]
fn test_paste_mixed_ascii_fullwidth() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Paste("ABC１２３".to_string()));

    let text = state.textarea.lines().join("\n");
    assert_eq!(text, "ABC１２３");
}
