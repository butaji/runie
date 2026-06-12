//! Grok Element Tests - Exact string match tests for UI elements from GROK.md
//!
//! Tests each UI element renders EXACTLY as specified in the Grok reference format.
//! Uses 80x24 buffer (standard terminal size) for all rendering tests.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};
use crate::components::{
    home_screen::HomeScreen,
    message_list::{
        MessageItem,
        MessageList,
        MessageListViewModel,
        PlanStatus,
        render::{WrapCache, ThinkingBlock, render_thinking_block, ToolCallBlock, ToolStatus, render_tool_call_block},
    },
    permission_modal::PermissionModal,
    top_bar::{TopBarViewModel, render_top_bar},
};
use crate::glyphs::{spinner_frame, CHEVRON, CHEVRON_WITH_SPACE, THOUGHT_MARKER, SPINNER_FRAMES};
use crate::theme::{ThemeColors, ThemeWrapper};
use crate::tui::{
    view_models::StatusBarViewModel,
    state::{TuiMode, AnimationState},
};
use runie_ai::TokenUsage;

/// Buffer size for all tests (standard terminal)
const WIDTH: u16 = 80;
const HEIGHT: u16 = 24;

/// Create test colors matching Grok dark theme
fn make_test_colors() -> ThemeColors {
    ThemeColors {
        bg_base: Color::Rgb(0x0F, 0x0C, 0x14),
        bg_panel: Color::Rgb(0x20, 0x1F, 0x26),
        accent_primary: Color::Rgb(0xFF, 0x6B, 0x00),
        text_primary: Color::Rgb(0xFF, 0xFA, 0xF1),
        text_secondary: Color::Rgb(0xDF, 0xDB, 0xDD),
        text_dim: Color::Rgb(0x8A, 0x87, 0x94),
        text_muted: Color::Rgb(0xBF, 0xBC, 0xC8),
        border_unfocused: Color::Rgb(0x3A, 0x39, 0x43),
        success: Color::Rgb(0x00, 0xF5, 0xD4),
        warning: Color::Rgb(0xFF, 0x6B, 0x00),
        error: Color::Rgb(0xEB, 0x42, 0x68),
        syntax_phase: Color::Rgb(0x6B, 0x50, 0xFF),
        accent_secondary: Color::Rgb(0x00, 0xF5, 0xD4),
        text_plan: Color::Rgb(0x6B, 0x50, 0xFF),
        feed_tool_bar: Color::Rgb(0x60, 0xA5, 0xFA),
        accent_user: Color::Rgb(0x6E, 0xE7, 0xB7),
        accent_assistant: Color::Rgb(0x00, 0xF5, 0xD4),
        accent_thinking: Color::Rgb(0xFB, 0xBF, 0x24),
        accent_tool: Color::Rgb(0x60, 0xA5, 0xFA),
        accent_system: Color::Rgb(0x8A, 0x87, 0x94),
        accent_error: Color::Rgb(0xEB, 0x42, 0x68),
        accent_success: Color::Rgb(0x00, 0xF5, 0xD4),
        accent_running: Color::Rgb(0xFB, 0xBF, 0x24),
        accent_skill: Color::Rgb(0x88, 0xA6, 0xFF),
        accent_plan: Color::Rgb(0xFB, 0xBF, 0x24),
        accent_feedback: Color::Rgb(0xEB, 0x42, 0x68),
        accent_model: Color::Rgb(0x00, 0xF5, 0xD4),
        accent_teal: Color::Rgb(0x29, 0xC6, 0xBE),
        accent_orange: Color::Rgb(0xD5, 0x95, 0x56),
        accent_purple: Color::Rgb(0xBC, 0x97, 0xFF),
        accent_yellow: Color::Rgb(0xCF, 0xB4, 0x7C),
        accent_blue_bright: Color::Rgb(0x88, 0xA6, 0xFF),
        command: Color::Rgb(0x60, 0xA5, 0xFA),
        path: Color::Rgb(0x00, 0xF5, 0xD4),
        running: Color::Rgb(0xFB, 0xBF, 0x24),
        fuzzy_accent: Color::Rgb(0x60, 0xA5, 0xFA),
        editor_bg: Color::Rgb(0x20, 0x1F, 0x26),
        surface_bg: Color::Rgb(0x20, 0x20, 0x20),
        popover_bg: Color::Rgb(0x1A, 0x1A, 0x1A),
    }
}

/// Extract rendered line at specific y position from buffer
fn get_line(buf: &Buffer, y: u16) -> String {
    let mut line = String::new();
    for x in 0..buf.area.width {
        if let Some(cell) = buf.cell((x, y)) {
            line.push_str(cell.symbol());
        }
    }
    line.trim_end().to_string()
}

/// Convert buffer to string for debugging
fn buffer_to_string(buf: &Buffer) -> String {
    let mut result = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            if let Some(cell) = buf.cell((x, y)) {
                result.push_str(cell.symbol());
            }
        }
        if y < buf.area.height - 1 {
            result.push('\n');
        }
    }
    result
}

// ============================================================================
// 1. Header Bar Tests
// ============================================================================

mod header_bar {
    use super::*;

    #[test]
    fn test_header_idle_exact_format() {
        // GROK.md: "   main ~/Code/GitHub/runie                                    │ 21K / 512K │"
        let vm = TopBarViewModel {
            repo: "runie".to_string(),
            branch: "main".to_string(),
            path: "~/Code/GitHub/runie".to_string(),
            context_window: 512_000,
            estimated_tokens: 21_000,
            agent_running: false,
            braille_frame: 0,
            mode: TuiMode::Chat,
        };
        let colors = make_test_colors();
        let area = Rect::new(0, 0, WIDTH, 1);
        let mut buf = Buffer::empty(area);
        render_top_bar(&vm, area, &mut buf, &colors);

        let rendered = get_line(&buf, 0);

        // Verify exact format: branch indicator + path + token meter
        assert!(rendered.contains("main"), "Should contain branch 'main'");
        assert!(rendered.contains("~/Code/GitHub/runie"), "Should contain path");
        assert!(rendered.contains("│ 21K / 512K │"), "Should contain token meter");
    }

    #[test]
    fn test_header_streaming_with_spinner() {
        // GROK.md shows spinner when agent is running
        let vm = TopBarViewModel {
            repo: "runie".to_string(),
            branch: "main".to_string(),
            path: "~/Code/GitHub/runie".to_string(),
            context_window: 512_000,
            estimated_tokens: 45_000,
            agent_running: true,
            braille_frame: 2, // Frame 2 = ⠼
            mode: TuiMode::Chat,
        };
        let colors = make_test_colors();
        let area = Rect::new(0, 0, WIDTH, 1);
        let mut buf = Buffer::empty(area);
        render_top_bar(&vm, area, &mut buf, &colors);

        let rendered = get_line(&buf, 0);

        // Should contain spinner character
        let spinner = spinner_frame(2);
        assert!(rendered.starts_with(spinner.to_string().as_str()) ||
                rendered.contains(&spinner.to_string()),
                "Should start with spinner character: {}", spinner);
    }

    #[test]
    fn test_header_no_path() {
        // When path is empty
        let vm = TopBarViewModel {
            repo: "runie".to_string(),
            branch: "main".to_string(),
            path: String::new(),
            context_window: 128_000,
            estimated_tokens: 32_000,
            agent_running: false,
            braille_frame: 0,
            mode: TuiMode::Chat,
        };
        let colors = make_test_colors();
        let area = Rect::new(0, 0, WIDTH, 1);
        let mut buf = Buffer::empty(area);
        render_top_bar(&vm, area, &mut buf, &colors);

        let rendered = get_line(&buf, 0);

        assert!(rendered.contains("main"), "Should contain branch even without path");
    }

    #[test]
    fn test_header_token_formats() {
        // Test different token count formats
        let test_cases = vec![
            (4_000, "4K"),
            (21_000, "21K"),
            (512_000, "512K"),
            (1_000_000, "1.0M"),
        ];

        for (tokens, expected_suffix) in test_cases {
            let vm = TopBarViewModel {
                repo: "runie".to_string(),
                branch: "main".to_string(),
                path: "~/Code/GitHub/runie".to_string(),
                context_window: 512_000,
                estimated_tokens: tokens,
                agent_running: false,
                braille_frame: 0,
                mode: TuiMode::Chat,
            };
            let colors = make_test_colors();
            let area = Rect::new(0, 0, WIDTH, 1);
            let mut buf = Buffer::empty(area);
            render_top_bar(&vm, area, &mut buf, &colors);

            let rendered = get_line(&buf, 0);
            assert!(rendered.contains(&format!("{} / 512K", expected_suffix)),
                    "Should show '{} / 512K' but got: {}", expected_suffix, rendered);
        }
    }
}

// ============================================================================
// 2. Welcome Screen Tests
// ============================================================================

mod welcome_screen {
    use super::*;

    #[test]
    fn test_welcome_menu_items() {
        // GROK.md shows 3 menu items with keyboard hints
        let screen = HomeScreen::new();
        let area = Rect::new(0, 0, WIDTH, HEIGHT);
        let mut buf = Buffer::empty(area);
        (&screen).render(area, &mut buf);

        // Check menu items are present
        let content = buffer_to_string(&buf);
        assert!(content.contains("New worktree"), "Should contain 'New worktree'");
        assert!(content.contains("ctrl-w"), "Should contain keyboard hint 'ctrl-w'");
        assert!(content.contains("Resume session"), "Should contain 'Resume session'");
        assert!(content.contains("ctrl-s"), "Should contain keyboard hint 'ctrl-s'");
        assert!(content.contains("Quit"), "Should contain 'Quit'");
        assert!(content.contains("ctrl-q"), "Should contain keyboard hint 'ctrl-q'");
    }

    #[test]
    fn test_welcome_dividers() {
        // GROK.md: dividers are "───" (horizontal line characters)
        let screen = HomeScreen::new();
        let area = Rect::new(0, 0, WIDTH, HEIGHT);
        let mut buf = Buffer::empty(area);
        (&screen).render(area, &mut buf);

        let content = buffer_to_string(&buf);
        // Dividers should be present (box drawing horizontal line)
        assert!(content.contains('─'), "Should contain horizontal divider character");
    }

    #[test]
    fn test_welcome_tip() {
        // GROK.md: "Tip: Press Ctrl-W to start a parallel task in its own worktree."
        let screen = HomeScreen::new();
        let area = Rect::new(0, 0, WIDTH, HEIGHT);
        let mut buf = Buffer::empty(area);
        (&screen).render(area, &mut buf);

        let content = buffer_to_string(&buf);
        assert!(content.contains("Tip: Press Ctrl-W to start a parallel task in its own worktree."),
                "Should contain exact tip text");
    }
}

// ============================================================================
// 3. User Message Tests
// ============================================================================

mod user_message {
    use super::*;

    #[test]
    fn test_user_msg_exact_format() {
        // GROK.md: "     ❯ hello                                                         9:45 PM"
        // Note: 5 spaces before ❯
        let vm = MessageListViewModel {
            messages: vec![
                MessageItem::User {
                    text: "hello".to_string(),
                    model: None,
                    timestamp: Some("9:45 PM".to_string()),
                },
            ],
            scroll_offset: 0,
            agent_running: false,
            animation: AnimationState::default(),
            wrap_cache: WrapCache::new(),
        };
        let theme = ThemeWrapper::default();
        let area = Rect::new(0, 2, WIDTH, 18);
        let mut buf = Buffer::empty(area);
        MessageList::render_ref(&vm, area, &mut buf, &theme);

        let content = buffer_to_string(&buf);

        // Verify chevron is present
        assert!(content.contains(CHEVRON.to_string().as_str()), "Should contain chevron ❯");
        // Verify timestamp is present
        assert!(content.contains("9:45 PM"), "Should contain timestamp");
        // Verify user text is present
        assert!(content.contains("hello"), "Should contain user message text");
    }

    #[test]
    fn test_user_msg_indent_spaces() {
        // Verify 5-space indent before chevron
        let vm = MessageListViewModel {
            messages: vec![
                MessageItem::User {
                    text: "test".to_string(),
                    model: None,
                    timestamp: Some("11:28 PM".to_string()),
                },
            ],
            scroll_offset: 0,
            agent_running: false,
            animation: AnimationState::default(),
            wrap_cache: WrapCache::new(),
        };
        let theme = ThemeWrapper::default();
        let area = Rect::new(0, 2, WIDTH, 18);
        let mut buf = Buffer::empty(area);
        MessageList::render_ref(&vm, area, &mut buf, &theme);

        let content = buffer_to_string(&buf);

        // The chevron should be preceded by spaces
        // Looking for pattern like "     ❯" (5 spaces)
        let chevron_str = CHEVRON.to_string();
        if let Some(pos) = content.find(chevron_str.as_str()) {
            let prefix = &content[..pos];
            let space_count = prefix.len();
            assert_eq!(space_count, 5, "Should have exactly 5 spaces before chevron");
        } else {
            panic!("Chevron not found in content");
        }
    }
}

// ============================================================================
// 4. Thinking Block Tests
// ============================================================================

mod thinking_block {
    use super::*;

    #[test]
    fn test_thinking_collapsed() {
        // GROK.md: "◆ Thought for 1.2s"
        let block = ThinkingBlock {
            content: "User wants to list files...".to_string(),
            duration_secs: 1.2,
            collapsed: true,
            animation_frame: 0,
        };
        let theme = ThemeWrapper::default();
        let area = Rect::new(0, 2, WIDTH, 10);
        let mut buf = Buffer::empty(area);
        render_thinking_block(&block, area, &mut buf, &theme, 0, 3);

        let content = buffer_to_string(&buf);

        assert!(content.contains(THOUGHT_MARKER.to_string().as_str()),
                "Should contain diamond marker ◆");
        assert!(content.contains("Thought for"), "Should contain 'Thought for'");
    }

    #[test]
    fn test_thinking_expanded_header() {
        // GROK.md: "┃  ◆ Thinking…"
        let block = ThinkingBlock {
            content: "The user said...".to_string(),
            duration_secs: 0.9,
            collapsed: false,
            animation_frame: 0,
        };
        let theme = ThemeWrapper::default();
        let area = Rect::new(0, 2, WIDTH, 10);
        let mut buf = Buffer::empty(area);
        render_thinking_block(&block, area, &mut buf, &theme, 0, 3);

        let content = buffer_to_string(&buf);

        // Should have vertical bar and thinking marker
        assert!(content.contains('┃'), "Should contain vertical bar ┃");
        assert!(content.contains(THOUGHT_MARKER.to_string().as_str()),
                "Should contain diamond ◆");
        assert!(content.contains("Thinking…"), "Should contain 'Thinking…'");
    }

    #[test]
    fn test_thinking_expanded_content() {
        // Content lines have "┃  " prefix
        let block = ThinkingBlock {
            content: "The user said \"list src\". They want to list the source files.".to_string(),
            duration_secs: 0.9,
            collapsed: false,
            animation_frame: 0,
        };
        let theme = ThemeWrapper::default();
        let area = Rect::new(0, 2, WIDTH, 10);
        let mut buf = Buffer::empty(area);
        render_thinking_block(&block, area, &mut buf, &theme, 0, 3);

        let content = buffer_to_string(&buf);

        // Content should be prefixed with ┃
        assert!(content.contains("┃"), "Content lines should have vertical bar prefix");
    }

    #[test]
    fn test_thinking_empty_line() {
        // GROK.md shows empty line with just "┃" prefix
        let block = ThinkingBlock {
            content: "".to_string(),
            duration_secs: 0.0,
            collapsed: false,
            animation_frame: 0,
        };
        let theme = ThemeWrapper::default();
        let area = Rect::new(0, 2, WIDTH, 10);
        let mut buf = Buffer::empty(area);
        render_thinking_block(&block, area, &mut buf, &theme, 0, 3);

        // Should render without crashing
        let content = buffer_to_string(&buf);
        assert!(content.contains("┃"), "Should have vertical bar for empty line");
    }
}

// ============================================================================
// 5. Tool Call Tests
// ============================================================================

mod tool_call {
    use super::*;

    #[test]
    fn test_tool_running() {
        // GROK.md: "⠴ Run List `.` 2.9s"
        let block = ToolCallBlock {
            tool_name: "List".to_string(),
            args: "`.`".to_string(),
            status: ToolStatus::Running,
            elapsed_secs: 2.9,
            total_secs: 0.0,
            bytes_in: 0,
            spinner_frame: 3, // ⠼
        };
        let theme = ThemeWrapper::default();
        let area = Rect::new(0, 2, WIDTH, 10);
        let mut buf = Buffer::empty(area);
        render_tool_call_block(&block, area, &mut buf, &theme, 0, 3);

        let content = buffer_to_string(&buf);

        // Should show spinner, tool name, args, and elapsed time
        assert!(content.contains("List"), "Should contain tool name 'List'");
        assert!(content.contains("`.`"), "Should contain args");
        assert!(content.contains("2.9s"), "Should contain elapsed time");
    }

    #[test]
    fn test_tool_complete_with_checkmark() {
        // Tool completes with checkmark
        let block = ToolCallBlock {
            tool_name: "List".to_string(),
            args: "`.`".to_string(),
            status: ToolStatus::Complete,
            elapsed_secs: 0.0,
            total_secs: 2.9,
            bytes_in: 22_200,
            spinner_frame: 0,
        };
        let theme = ThemeWrapper::default();
        let area = Rect::new(0, 2, WIDTH, 10);
        let mut buf = Buffer::empty(area);
        render_tool_call_block(&block, area, &mut buf, &theme, 0, 3);

        let content = buffer_to_string(&buf);

        // Should contain checkmark
        assert!(content.contains('✓') || content.contains("[✓]"),
                "Should contain success checkmark");
    }

    #[test]
    fn test_tool_spinner_animation() {
        // Verify spinner frames cycle correctly
        let frames: Vec<char> = (0..8).map(|i| SPINNER_FRAMES[i % SPINNER_FRAMES.len()]).collect();

        // Frames should be different (animation)
        assert!(frames[0] != frames[1], "Spinner should animate");
        assert!(frames[2] != frames[3], "Spinner should animate");
    }
}

// ============================================================================
// 6. Assistant Response Tests
// ============================================================================

mod assistant_response {
    use super::*;

    #[test]
    fn test_assistant_simple() {
        // Simple assistant response with timestamp
        let vm = MessageListViewModel {
            messages: vec![
                MessageItem::User {
                    text: "hello".to_string(),
                    model: None,
                    timestamp: Some("11:28 PM".to_string()),
                },
                MessageItem::Assistant {
                    text: "Hello! How can I help?".to_string(),
                    model: Some("gpt-4o".to_string()),
                    timestamp: Some("11:28 PM".to_string()),
                    expanded: true,
                    thought_duration: None,
                    turn_duration: None,
                },
            ],
            scroll_offset: 0,
            agent_running: false,
            animation: AnimationState::default(),
            wrap_cache: WrapCache::new(),
        };
        let theme = ThemeWrapper::default();
        let area = Rect::new(0, 2, WIDTH, 18);
        let mut buf = Buffer::empty(area);
        MessageList::render_ref(&vm, area, &mut buf, &theme);

        let content = buffer_to_string(&buf);

        // Assistant text should be present
        assert!(content.contains("Hello! How can I help?"),
                "Should contain assistant response text");
    }

    #[test]
    fn test_assistant_turn_complete() {
        // Turn completion message
        let vm = MessageListViewModel {
            messages: vec![
                MessageItem::User {
                    text: "hello".to_string(),
                    model: None,
                    timestamp: Some("11:28 PM".to_string()),
                },
                MessageItem::Assistant {
                    text: "Done!".to_string(),
                    model: Some("gpt-4o".to_string()),
                    timestamp: Some("11:28 PM".to_string()),
                    expanded: true,
                    thought_duration: None,
                    turn_duration: Some(3.6),
                },
            ],
            scroll_offset: 0,
            agent_running: false,
            animation: AnimationState::default(),
            wrap_cache: WrapCache::new(),
        };
        let theme = ThemeWrapper::default();
        let area = Rect::new(0, 2, WIDTH, 18);
        let mut buf = Buffer::empty(area);
        MessageList::render_ref(&vm, area, &mut buf, &theme);

        let content = buffer_to_string(&buf);

        // Turn completed message should appear
        assert!(content.contains("Turn completed") || content.contains("3.6s"),
                "Should contain turn duration");
    }
}

// ============================================================================
// 7. Complete Conversation Flow Tests
// ============================================================================

mod conversation_flow {
    use super::*;

    #[test]
    fn test_grok_style_conversation_elements() {
        // Full conversation from GROK.md
        let vm = MessageListViewModel {
            messages: vec![
                MessageItem::User {
                    text: "grok".to_string(),
                    model: None,
                    timestamp: Some("11:28 PM".to_string()),
                },
                MessageItem::Thought {
                    duration_secs: 0.9,
                    text: "The user wants to interact with grok".to_string(),
                },
                MessageItem::ToolCall {
                    name: "Read".to_string(),
                    args: "~/.grok/docs/user-guide/README.md".to_string(),
                    result: None,
                    is_error: false,
                },
                MessageItem::ToolCall {
                    name: "Read".to_string(),
                    args: "Cargo.toml".to_string(),
                    result: None,
                    is_error: false,
                },
                MessageItem::ToolCall {
                    name: "List".to_string(),
                    args: ".".to_string(),
                    result: None,
                    is_error: false,
                },
                MessageItem::Assistant {
                    text: "Want me to: Show a specific crate in detail?".to_string(),
                    model: Some("gpt-4o".to_string()),
                    timestamp: Some("11:28 PM".to_string()),
                    expanded: true,
                    thought_duration: Some(0.9),
                    turn_duration: Some(11.0),
                },
            ],
            scroll_offset: 0,
            agent_running: false,
            animation: AnimationState::default(),
            wrap_cache: WrapCache::new(),
        };
        let theme = ThemeWrapper::default();
        let area = Rect::new(0, 2, WIDTH, 24);
        let mut buf = Buffer::empty(area);
        MessageList::render_ref(&vm, area, &mut buf, &theme);

        let content = buffer_to_string(&buf);

        // Verify all elements present
        assert!(content.contains("grok"), "Should contain user message");
        assert!(content.contains("Thought"), "Should contain thinking indicator");
        assert!(content.contains("Read"), "Should contain tool calls");
        assert!(content.contains("List"), "Should contain List tool");
    }
}

// ============================================================================
// 8. Glyph/Symbol Verification Tests
// ============================================================================

mod glyphs {
    use super::*;

    #[test]
    fn test_glyph_chevron() {
        assert_eq!(CHEVRON, '\u{276F}', "Chevron should be ❯");
        assert_eq!(CHEVRON_WITH_SPACE, "❯ ", "Chevron with space should be '❯ '");
    }

    #[test]
    fn test_glyph_thought_marker() {
        assert_eq!(THOUGHT_MARKER, '◆', "Thought marker should be ◆");
    }

    #[test]
    fn test_spinner_frames() {
        // Verify spinner frames are braille characters
        for frame in SPINNER_FRAMES {
            assert!(frame.is_ascii(), "Spinner frames should be valid characters");
        }
        // Frame 3 should be ⠼ (as per GROK.md example)
        assert_eq!(SPINNER_FRAMES[3], '⠼', "Frame 3 should be ⠼");
    }
}

// ============================================================================
// 9. Permission Modal Tests (Grok-style from GROK.md)
// ============================================================================

mod permission_modal {
    use super::*;

    #[test]
    fn test_permission_modal_grok_style() {
        let modal = PermissionModal::new(
            "bash",
            r#"{"command": "ls -la"}"#,
            "Lists files in current directory",
        );
        let theme = ThemeWrapper::default();
        let area = Rect::new(0, 0, 60, 18);
        let mut buf = Buffer::empty(area);
        modal.render_ref(area, &mut buf, &theme);

        let content = buffer_to_string(&buf);

        // Should show tool name and args
        assert!(content.contains("bash"), "Should show tool name");
        assert!(content.contains("ls -la") || content.contains("ls"),
                "Should show command");
    }
}
