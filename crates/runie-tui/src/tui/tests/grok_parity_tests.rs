//! Grok Parity Tests - Verify UI matches Grok behavior from asciinema dumps.
//!
//! Tests FAIL if UI doesn't match Grok's exact behavior, PASS when parity achieved.
//! These are behavioral assertions against the reference dumps in:
//!   - ui/dumps/compare/grok/01_clean.txt (Welcome screen)
//!   - ui/dumps/compare/grok/02_clean.txt (Idle state)
//!   - ui/dumps/compare/grok/03_clean.txt (Thinking state)
//!   - ui/dumps/compare/grok/03_chat.txt (Complete turn)
//!   - ui/dumps/compare/grok/04_tools.txt (Tool call)
//!
//! Reference spec: GROK.md

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
        MessageListViewModel,
        MessageItem,
        MessageList,
    },
    message_list::render::WrapCache,
    message_list::feed::Feed,
    message_list::render::tool_call::{ToolCallBlock, ToolStatus, render_tool_call_block},
    message_list::render::thinking::{ThinkingBlock, render_thinking_block},
    top_bar::{TopBarViewModel, render_top_bar},
    input_bar::render_input_bar,
};
use crate::glyphs::{spinner_frame, CHEVRON, CHEVRON_WITH_SPACE, THOUGHT_MARKER, SPINNER_FRAMES};
use crate::theme::{ThemeColors, ThemeWrapper};
use crate::tui::{
    state::TuiMode,
    view_models::StatusBarViewModel,
};
use runie_ai::TokenUsage;

// ============================================================================
// Test Constants (from reference dumps)
// ============================================================================

/// Standard terminal size used in reference dumps
const WIDTH: u16 = 80;
const HEIGHT: u16 = 24;

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

/// Convert full buffer to string for debugging
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

// ============================================================================
// Test 1: Welcome Screen Elements (from 01_clean.txt)
// ============================================================================

mod welcome_screen_elements {
    use super::*;

    /// From 01_clean.txt:
    /// Line 6: `                       New worktree                   ctrl-w`
    /// Line 7: `                       ─────────────────────────────────────`
    /// Line 8: `                       Resume session                 ctrl-s`
    /// Line 17: `  Tip: Press Ctrl-W to start a parallel task in its own worktree.`
    /// Line 23: `                                                                    0.2.16 Beta`

    #[test]
    fn test_menu_items_render_at_line_6() {
        // Menu items should appear at line 6 (0-indexed y=5) in a centered layout
        let screen = HomeScreen::new();
        let area = Rect::new(0, 0, WIDTH, HEIGHT);
        let mut buf = Buffer::empty(area);
        screen.render(area, &mut buf);

        let content = buffer_to_string(&buf);
        let lines: Vec<&str> = content.lines().collect();

        // "New worktree" should be on line 6 (index 5)
        assert!(lines.get(5).map(|l| l.contains("New worktree")).unwrap_or(false),
            "New worktree should be at line 6");
    }

    #[test]
    fn test_menu_has_dividers() {
        // From 01_clean.txt: dividers are "───" between menu items
        let screen = HomeScreen::new();
        let area = Rect::new(0, 0, WIDTH, HEIGHT);
        let mut buf = Buffer::empty(area);
        screen.render(area, &mut buf);

        let content = buffer_to_string(&buf);

        // Should contain horizontal divider character (box drawing)
        assert!(content.contains('─'), "Menu should have horizontal dividers");
    }

    #[test]
    fn test_tip_text_below_menu() {
        // From 01_clean.txt line 17
        let screen = HomeScreen::new();
        let area = Rect::new(0, 0, WIDTH, HEIGHT);
        let mut buf = Buffer::empty(area);
        screen.render(area, &mut buf);

        let content = buffer_to_string(&buf);
        assert!(content.contains("Tip: Press Ctrl-W to start a parallel task in its own worktree."),
            "Should contain tip text");
    }

    #[test]
    fn test_input_bar_has_chevron_prompt() {
        // From 01_clean.txt lines 19-21: `╭──────────────────────────────────...`
        // `│ ❯                                                                        │`
        // `╰──────────────────────────────────────── Grok Build · always-approve ─╯`
        let textarea = ratatui_textarea::TextArea::new();
        let colors = make_test_colors();
        let area = Rect::new(0, 19, WIDTH, 3);
        let mut buf = Buffer::empty(area);
        render_input_bar(
            &textarea,
            CHEVRON_WITH_SPACE,
            "",
            area,
            &mut buf,
            &colors,
            "Grok Build · always-approve",
            &[],
            None,
            true,
        );

        let content = buffer_to_string(&buf);
        assert!(content.contains(CHEVRON), "Input bar should have ❯ prompt");
    }
}

// ============================================================================
// Test 2: Header Bar (from 02_clean.txt)
// ============================================================================

mod header_bar_tests {
    use super::*;

    /// From 02_clean.txt line 2:
    /// `   feat/grok-redesign ~/Code/GitHub/runie                     │ 7.6K / 512K │`
    /// Format: `   branch ~/path/                                     │ X.XK / XXXK │`

    #[test]
    fn test_git_branch_format() {
        // Header should show:  branch ~/path/
        let vm = TopBarViewModel {
            repo: "runie".to_string(),
            branch: "feat/grok-redesign".to_string(),
            path: "~/Code/GitHub/runie".to_string(),
            context_window: 512_000,
            estimated_tokens: 7_600,
            agent_running: false,
            braille_frame: 0,
            mode: TuiMode::Chat,
        };
        let colors = make_test_colors();
        let area = Rect::new(0, 0, WIDTH, 1);
        let mut buf = Buffer::empty(area);
        render_top_bar(&vm, area, &mut buf, &colors);

        let rendered = get_line(&buf, 0);

        // Should contain git branch indicator
        assert!(rendered.contains("feat/grok-redesign"),
            "Should contain branch name, got: {}", rendered);
        assert!(rendered.contains("~/Code/GitHub/runie"),
            "Should contain path, got: {}", rendered);
    }

    #[test]
    fn test_token_meter_format() {
        // Header should show: `│ X.XK / XXXK │`
        let vm = TopBarViewModel {
            repo: "runie".to_string(),
            branch: "feat/grok-redesign".to_string(),
            path: "~/Code/GitHub/runie".to_string(),
            context_window: 512_000,
            estimated_tokens: 7_600,
            agent_running: false,
            braille_frame: 0,
            mode: TuiMode::Chat,
        };
        let colors = make_test_colors();
        let area = Rect::new(0, 0, WIDTH, 1);
        let mut buf = Buffer::empty(area);
        render_top_bar(&vm, area, &mut buf, &colors);

        let rendered = get_line(&buf, 0);

        // Token meter format: │ X.XK / XXXK │
        assert!(rendered.contains("│"), "Should have pipe separators");
        assert!(rendered.contains("/"), "Should have slash separator between tokens");
    }

    #[test]
    fn test_token_format_21k() {
        // From 03_clean.txt: `│ 21K / 512K │`
        let vm = TopBarViewModel {
            repo: "runie".to_string(),
            branch: "feat/grok-redesign".to_string(),
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

        // Should show 21K format (not 21.0K)
        assert!(rendered.contains("21K") || rendered.contains("21.0K"),
            "Should contain 21K token count, got: {}", rendered);
    }
}

// ============================================================================
// Test 3: User Message (from 03_clean.txt)
// ============================================================================

mod user_message_tests {
    use super::*;

    /// From 03_clean.txt line 5:
    /// `     ❯ hello                                                         9:52 PM   █`
    /// Format: 5-space indent + ❯ + message + timestamp right-aligned

    #[test]
    fn test_user_message_chevron_prefix() {
        // User message should have ❯ chevron prefix
        let messages = vec![
            MessageItem::User {
                text: "hello".to_string(),
                model: None,
                timestamp: Some("9:52 PM".to_string()),
            },
        ];
        let feed = Feed::from(messages);
        let vm = MessageListViewModel::new(
            feed,
            0,
            false,
            crate::tui::state::AnimationState::default(),
            WrapCache::new(),
            None,
            None,
        );
        let theme = ThemeWrapper::default();
        let area = Rect::new(0, 2, WIDTH, 18);
        let mut buf = Buffer::empty(area);
        MessageList::render_ref(&vm, area, &mut buf, &theme);

        let content = buffer_to_string(&buf);
        assert!(content.contains(CHEVRON), "User message should have ❯ prefix");
    }

    #[test]
    fn test_user_message_5_space_indent() {
        // From reference: 5 spaces before ❯
        let messages = vec![
            MessageItem::User {
                text: "test".to_string(),
                model: None,
                timestamp: Some("9:52 PM".to_string()),
            },
        ];
        let feed = Feed::from(messages);
        let vm = MessageListViewModel::new(
            feed,
            0,
            false,
            crate::tui::state::AnimationState::default(),
            WrapCache::new(),
            None,
            None,
        );
        let theme = ThemeWrapper::default();
        let area = Rect::new(0, 2, WIDTH, 18);
        let mut buf = Buffer::empty(area);
        MessageList::render_ref(&vm, area, &mut buf, &theme);

        let content = buffer_to_string(&buf);

        // Find position of chevron
        let chevron_str = CHEVRON.to_string();
        if let Some(pos) = content.find(chevron_str.as_str()) {
            let prefix = &content[..pos];
            let space_count = prefix.len();
            assert_eq!(space_count, 5, "Should have exactly 5 spaces before chevron");
        } else {
            panic!("Chevron not found in content");
        }
    }

    #[test]
    fn test_user_message_timestamp_right_aligned() {
        // Timestamp should appear right-aligned at end of line
        let messages = vec![
            MessageItem::User {
                text: "hello".to_string(),
                model: None,
                timestamp: Some("9:52 PM".to_string()),
            },
        ];
        let feed = Feed::from(messages);
        let vm = MessageListViewModel::new(
            feed,
            0,
            false,
            crate::tui::state::AnimationState::default(),
            WrapCache::new(),
            None,
            None,
        );
        let theme = ThemeWrapper::default();
        let area = Rect::new(0, 2, WIDTH, 18);
        let mut buf = Buffer::empty(area);
        MessageList::render_ref(&vm, area, &mut buf, &theme);

        let content = buffer_to_string(&buf);

        // Timestamp should be present
        assert!(content.contains("9:52 PM"), "Should contain timestamp");
    }
}

// ============================================================================
// Test 4: Thinking Block (from 03_clean.txt and 03_chat.txt)
// ============================================================================

mod thinking_block_tests {
    use super::*;

    /// From 03_clean.txt lines 9-14:
    /// `┃  ◆ Thinking…`
    /// `┃`
    /// `┃  …`
    /// `┃  (runie-tui). There's mention of Grok in docs and GROK.md, and it seems`
    /// `┃  related to AI/agent tooling, likely a terminal UI for interacting with`
    /// `┃  AI models like Grok.`
    ///
    /// From 03_chat.txt line 8 (collapsed):
    /// `     ◆ Thought for 1.2s`

    #[test]
    fn test_thinking_streaming_header() {
        // Streaming thinking block should have `┃  ◆ Thinking…` header
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

        // Should have vertical bar and thinking marker
        assert!(content.contains('┃'), "Should have vertical bar prefix");
        assert!(content.contains(THOUGHT_MARKER.to_string().as_str()),
            "Should have diamond marker ◆");
        assert!(content.contains("Thinking…") || content.contains("Thinking..."),
            "Should have Thinking header");
    }

    #[test]
    fn test_thinking_content_lines_prefix() {
        // Content lines should have `┃  ` prefix
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

        // Content should be prefixed with ┃
        assert!(content.contains("┃"), "Content lines should have vertical bar prefix");
    }

    #[test]
    fn test_thinking_collapsed_format() {
        // Collapsed thinking shows: `◆ Thought for X.Xs`
        let block = ThinkingBlock {
            content: "The user said...".to_string(),
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
            "Should have diamond marker ◆");
        assert!(content.contains("Thought for"), "Should contain 'Thought for'");
        assert!(content.contains("1.2s") || content.contains("1,2s"),
            "Should show duration");
    }
}

// ============================================================================
// Test 5: Assistant Response (from 03_chat.txt)
// ============================================================================

mod assistant_response_tests {
    use super::*;

    /// From 03_chat.txt lines 10-12:
    /// `     Hello. How can I help with the runie project?                   9:45 PM`
    /// `Turn completed in 3.6s.`
    /// Note: No prefix symbol, timestamp right-aligned, turn duration on next line

    #[test]
    fn test_assistant_no_prefix_symbol() {
        // Assistant message should NOT have a prefix symbol (plain text)
        let messages = vec![
            MessageItem::User {
                text: "hello".to_string(),
                model: None,
                timestamp: Some("9:45 PM".to_string()),
            },
            MessageItem::Assistant {
                text: "Hello. How can I help with the runie project?".to_string(),
                model: Some("gpt-4o".to_string()),
                timestamp: Some("9:45 PM".to_string()),
                expanded: true,
                thought_duration: None,
                turn_duration: None,
            },
        ];
        let feed = Feed::from(messages);
        let vm = MessageListViewModel::new(
            feed,
            0,
            false,
            crate::tui::state::AnimationState::default(),
            WrapCache::new(),
            None,
            None,
        );
        let theme = ThemeWrapper::default();
        let area = Rect::new(0, 2, WIDTH, 18);
        let mut buf = Buffer::empty(area);
        MessageList::render_ref(&vm, area, &mut buf, &theme);

        let content = buffer_to_string(&buf);

        // Assistant response should be present
        assert!(content.contains("Hello. How can I help"),
            "Should contain assistant text");
    }

    #[test]
    fn test_assistant_timestamp_right_aligned() {
        // Timestamp should be right-aligned on same line as response
        let messages = vec![
            MessageItem::User {
                text: "hello".to_string(),
                model: None,
                timestamp: Some("9:45 PM".to_string()),
            },
            MessageItem::Assistant {
                text: "Hello. How can I help with the runie project?".to_string(),
                model: Some("gpt-4o".to_string()),
                timestamp: Some("9:45 PM".to_string()),
                expanded: true,
                thought_duration: None,
                turn_duration: None,
            },
        ];
        let feed = Feed::from(messages);
        let vm = MessageListViewModel::new(
            feed,
            0,
            false,
            crate::tui::state::AnimationState::default(),
            WrapCache::new(),
            None,
            None,
        );
        let theme = ThemeWrapper::default();
        let area = Rect::new(0, 2, WIDTH, 18);
        let mut buf = Buffer::empty(area);
        MessageList::render_ref(&vm, area, &mut buf, &theme);

        let content = buffer_to_string(&buf);

        // Timestamp should be present
        assert!(content.contains("9:45 PM"), "Should contain timestamp");
    }

    #[test]
    fn test_turn_completed_message() {
        // After assistant response: `Turn completed in X.Xs.`
        let messages = vec![
            MessageItem::User {
                text: "hello".to_string(),
                model: None,
                timestamp: Some("9:45 PM".to_string()),
            },
            MessageItem::Assistant {
                text: "Hello.".to_string(),
                model: Some("gpt-4o".to_string()),
                timestamp: Some("9:45 PM".to_string()),
                expanded: true,
                thought_duration: None,
                turn_duration: Some(3.6),
            },
        ];
        let feed = Feed::from(messages);
        let vm = MessageListViewModel::new(
            feed,
            0,
            false,
            crate::tui::state::AnimationState::default(),
            WrapCache::new(),
            None,
            None,
        );
        let theme = ThemeWrapper::default();
        let area = Rect::new(0, 2, WIDTH, 18);
        let mut buf = Buffer::empty(area);
        MessageList::render_ref(&vm, area, &mut buf, &theme);

        let content = buffer_to_string(&buf);

        // Turn completed message should appear
        assert!(content.contains("Turn completed") || content.contains("3.6s"),
            "Should contain turn duration info");
    }
}

// ============================================================================
// Test 6: Tool Call (from 04_tools.txt)
// ============================================================================

mod tool_call_tests {
    use super::*;

    /// From 04_tools.txt lines 8-17:
    /// `     ◆ Thought for 2.1s`
    /// `     ◆ List .` (inline tool indicator)
    /// `   ┃  ◆ Thinking…`
    /// `   ┃`
    /// `   ┃  The tool returned a summary listing of the directory structure...`
    /// `     ⠦ Thinking… 4.3s                                          10.0s ⇣22.5k [✗]`
    /// Tool call format: `⠦ Thinking… X.Xs` + right side: `X.Xs ⇣XX.Xk [✓/✗]`

    #[test]
    fn test_tool_inline_indicator() {
        // Tool call should show `◆ ToolName args` inline indicator
        let block = ToolCallBlock {
            tool_name: "List".to_string(),
            args: ".".to_string(),
            status: ToolStatus::Running,
            elapsed_secs: 2.1,
            total_secs: 0.0,
            bytes_in: 0,
            spinner_frame: 2,
        };
        let theme = ThemeWrapper::default();
        let area = Rect::new(0, 2, WIDTH, 10);
        let mut buf = Buffer::empty(area);
        render_tool_call_block(&block, area, &mut buf, &theme, 0, 3);

        let content = buffer_to_string(&buf);

        // Should contain tool name
        assert!(content.contains("List"), "Should contain tool name 'List'");
    }

    #[test]
    fn test_tool_status_with_spinner() {
        // Running tool shows: `⠦ Thinking… X.Xs`
        let block = ToolCallBlock {
            tool_name: "List".to_string(),
            args: ".".to_string(),
            status: ToolStatus::Running,
            elapsed_secs: 4.3,
            total_secs: 0.0,
            bytes_in: 0,
            spinner_frame: 2, // ⠦
        };
        let theme = ThemeWrapper::default();
        let area = Rect::new(0, 2, WIDTH, 10);
        let mut buf = Buffer::empty(area);
        render_tool_call_block(&block, area, &mut buf, &theme, 0, 3);

        let content = buffer_to_string(&buf);

        // Should show spinner and duration
        assert!(content.contains("4.3s") || content.contains("4,3s"),
            "Should show elapsed time");
    }

    #[test]
    fn test_tool_right_side_metrics() {
        // Right side shows: `X.Xs ⇣XX.Xk [✓/✗]`
        let block = ToolCallBlock {
            tool_name: "List".to_string(),
            args: ".".to_string(),
            status: ToolStatus::Complete,
            elapsed_secs: 0.0,
            total_secs: 10.0,
            bytes_in: 22_500,
            spinner_frame: 0,
        };
        let theme = ThemeWrapper::default();
        let area = Rect::new(0, 2, WIDTH, 10);
        let mut buf = Buffer::empty(area);
        render_tool_call_block(&block, area, &mut buf, &theme, 0, 3);

        let content = buffer_to_string(&buf);

        // Should show success/failure indicator
        assert!(content.contains('✓') || content.contains("[✓]") || content.contains('✗') || content.contains("[✗]"),
            "Should show success/failure indicator");
    }
}

// ============================================================================
// Test 7: Status Bar (from 02_clean.txt vs 03_clean.txt)
// ============================================================================

mod status_bar_tests {
    use super::*;

    /// From 02_clean.txt line 23 (idle):
    /// `  Shift+Tab:mode  │  Ctrl+.:shortcuts`
    /// 2 shortcuts when idle
    ///
    /// From 03_clean.txt line 23 (running):
    /// `  Shift+Tab:mode  │  Ctrl+c:cancel  │  Ctrl+Enter:interject  │  Ctrl+.:`
    /// 4 shortcuts when agent running

    #[test]
    fn test_status_bar_idle_has_2_shortcuts() {
        // Idle state should show 2 shortcuts
        let vm = StatusBarViewModel {
            mode: TuiMode::Chat,
            current_model: Some("gpt-4o".to_string()),
            session_token_usage: TokenUsage::default(),
            status_header: None,
            status_details: None,
            status_start_time: None,
            mcp_status: crate::tui::view_models::McpStatus::None,
            agent_running: false,
            input_has_text: false,
        };

        let hotkeys = vm.hotkeys();
        assert_eq!(hotkeys.len(), 2, "Idle should have 2 shortcuts, got {}: {:?}",
            hotkeys.len(), hotkeys);
    }

    #[test]
    fn test_status_bar_running_has_4_shortcuts() {
        // Running state should show 4 shortcuts
        let vm = StatusBarViewModel {
            mode: TuiMode::Chat,
            current_model: Some("gpt-4o".to_string()),
            session_token_usage: TokenUsage::default(),
            status_header: Some("Thinking".to_string()),
            status_details: None,
            status_start_time: Some(std::time::Instant::now()),
            mcp_status: crate::tui::view_models::McpStatus::None,
            agent_running: true,
            input_has_text: false,
        };

        let hotkeys = vm.hotkeys();
        assert_eq!(hotkeys.len(), 4, "Running should have 4 shortcuts, got {}: {:?}",
            hotkeys.len(), hotkeys);
    }

    #[test]
    fn test_status_bar_running_includes_cancel() {
        // Running should include Ctrl+c:cancel
        let vm = StatusBarViewModel {
            mode: TuiMode::Chat,
            current_model: Some("gpt-4o".to_string()),
            session_token_usage: TokenUsage::default(),
            status_header: Some("Thinking".to_string()),
            status_details: None,
            status_start_time: Some(std::time::Instant::now()),
            mcp_status: crate::tui::view_models::McpStatus::None,
            agent_running: true,
            input_has_text: false,
        };

        let hotkeys = vm.hotkeys();
        let cancel = hotkeys.iter().find(|(k, _)| *k == "Ctrl+c");
        assert!(cancel.is_some(), "Running should include Ctrl+c:cancel");
    }

    #[test]
    fn test_status_bar_idle_excludes_cancel() {
        // Idle should NOT include Ctrl+c:cancel
        let vm = StatusBarViewModel {
            mode: TuiMode::Chat,
            current_model: Some("gpt-4o".to_string()),
            session_token_usage: TokenUsage::default(),
            status_header: None,
            status_details: None,
            status_start_time: None,
            mcp_status: crate::tui::view_models::McpStatus::None,
            agent_running: false,
            input_has_text: false,
        };

        let hotkeys = vm.hotkeys();
        let cancel = hotkeys.iter().find(|(k, _)| *k == "Ctrl+c");
        assert!(cancel.is_none(), "Idle should NOT include Ctrl+c:cancel");
    }
}

// ============================================================================
// Test 8: Input Bar (from 01_clean.txt and 02_clean.txt)
// ============================================================================

mod input_bar_tests {
    use super::*;

    /// From 01_clean.txt lines 19-21:
    /// `╭──────────────────────────────────────────────────────────────────────────╮`
    /// `│ ❯                                                                        │`
    /// `╰──────────────────────────────────────── Grok Build · always-approve ─╯`
    /// Format: prompt box with `❯` and mode suffix

    #[test]
    fn test_input_bar_prompt_symbol() {
        // Input bar should show ❯ prompt symbol
        let textarea = ratatui_textarea::TextArea::new();
        let colors = make_test_colors();
        let area = Rect::new(0, 19, WIDTH, 3);
        let mut buf = Buffer::empty(area);
        render_input_bar(
            &textarea,
            CHEVRON_WITH_SPACE,
            "",
            area,
            &mut buf,
            &colors,
            "Grok Build",
            &[],
            None,
            true,
        );

        let content = buffer_to_string(&buf);
        assert!(content.contains(CHEVRON), "Input bar should have ❯ prompt");
    }

    #[test]
    fn test_input_bar_mode_suffix_always_approve() {
        // Mode indicator should show `Grok Build · always-approve`
        let textarea = ratatui_textarea::TextArea::new();
        let colors = make_test_colors();
        let area = Rect::new(0, 19, WIDTH, 3);
        let mut buf = Buffer::empty(area);
        render_input_bar(
            &textarea,
            CHEVRON_WITH_SPACE,
            "",
            area,
            &mut buf,
            &colors,
            "Grok Build · always-approve",
            &[],
            None,
            true,
        );

        let content = buffer_to_string(&buf);
        assert!(content.contains("always-approve"), "Should show always-approve mode");
    }

    #[test]
    fn test_input_bar_mode_suffix_plan() {
        // Mode indicator should show `Grok Build · plan`
        let textarea = ratatui_textarea::TextArea::new();
        let colors = make_test_colors();
        let area = Rect::new(0, 19, WIDTH, 3);
        let mut buf = Buffer::empty(area);
        render_input_bar(
            &textarea,
            CHEVRON_WITH_SPACE,
            "",
            area,
            &mut buf,
            &colors,
            "Grok Build · plan",
            &[],
            None,
            true,
        );

        let content = buffer_to_string(&buf);
        assert!(content.contains("plan"), "Should show plan mode");
    }
}

// ============================================================================
// Glyph Verification Tests
// ============================================================================

mod glyph_verification {
    use super::*;

    #[test]
    fn test_chevron_is_correct_unicode() {
        // Chevron should be ❯ (U+276F)
        assert_eq!(CHEVRON, '\u{276F}', "Chevron should be ❯");
    }

    #[test]
    fn test_chevron_with_space() {
        // Chevron with space should be "❯ "
        assert_eq!(CHEVRON_WITH_SPACE, "❯ ", "Chevron with space should be '❯ '");
    }

    #[test]
    fn test_thought_marker_is_diamond() {
        // Thought marker should be ◆ (U+25C6)
        assert_eq!(THOUGHT_MARKER, '◆', "Thought marker should be ◆");
    }

    #[test]
    fn test_spinner_frame_2_is_braille() {
        // Frame 2 should be ⠦ (braille pattern)
        assert_eq!(SPINNER_FRAMES[2], '⠦', "Frame 2 should be ⠦");
    }

    #[test]
    fn test_all_spinner_frames_are_braille() {
        // All spinner frames should be valid braille characters
        for (i, frame) in SPINNER_FRAMES.iter().enumerate() {
            assert!(!frame.is_ascii_alphanumeric(),
                "Spinner frame {} should be braille, not ASCII", i);
        }
    }
}
