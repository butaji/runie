//! Grok parity snapshot regression tests.
//!
//! Tests each UI element from GROK.md against the expected Grok format.
//! Uses the same patterns as existing snapshot tests in this module.

use ratatui::{buffer::Buffer, layout::Rect, style::Color};
use insta::assert_snapshot;

use crate::components::{
    top_bar::{TopBarViewModel, render_top_bar},
    message_list::{MessageListViewModel, MessageItem, MessageList, PlanStatus},
    message_list::render::WrapCache,
    message_list::render::tool_call::{ToolCallBlock, ToolStatus, render_tool_call_block},
    message_list::render::thinking::{ThinkingBlock, render_thinking_block},
    home_screen::HomeScreen,
    permission_modal::PermissionModal,
    diff_viewer::DiffViewer,
};
use crate::tui::render::{render_status_bar, render_agent_list};
use crate::tui::view_models::{StatusBarViewModel, AgentListViewModel};
use crate::tui::state::{TuiMode, AnimationState};
use crate::theme::ThemeWrapper;
use runie_ai::TokenUsage;

// Re-export helpers from parent module
use super::{buffer_to_string, make_test_colors, user_message, assistant_message, SIDEBAR_WIDTH};

// ============================================================================
// 1. Header Bar Tests
// ============================================================================

mod header_bar {
    use super::*;

    #[test]
    fn snapshot_header_bar_0_tokens() {
        let vm = TopBarViewModel {
            repo: "runie".to_string(),
            branch: "main".to_string(),
            path: "~/Code/GitHub/runie".to_string(),
            context_window: 512_000,
            estimated_tokens: 0,
            agent_running: false,
            braille_frame: 0,
            mode: TuiMode::Chat,
        };
        let colors = make_test_colors();
        let area = Rect::new(0, 0, 80, 1);
        let mut buf = Buffer::empty(area);
        render_top_bar(&vm, area, &mut buf, &colors);
        insta::assert_snapshot!("grok_header_bar_0_tokens", buffer_to_string(&buf));
    }

    #[test]
    fn snapshot_header_bar_21k_tokens() {
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
        let area = Rect::new(0, 0, 80, 1);
        let mut buf = Buffer::empty(area);
        render_top_bar(&vm, area, &mut buf, &colors);
        insta::assert_snapshot!("grok_header_bar_21k_tokens", buffer_to_string(&buf));
    }

    #[test]
    fn snapshot_header_bar_half_full() {
        let vm = TopBarViewModel {
            repo: "runie".to_string(),
            branch: "main".to_string(),
            path: "src".to_string(),
            context_window: 128_000,
            estimated_tokens: 64_000,
            agent_running: false,
            braille_frame: 0,
            mode: TuiMode::Chat,
        };
        let colors = make_test_colors();
        let area = Rect::new(0, 0, 80, 1);
        let mut buf = Buffer::empty(area);
        render_top_bar(&vm, area, &mut buf, &colors);
        insta::assert_snapshot!("grok_header_bar_half_full", buffer_to_string(&buf));
    }

    #[test]
    fn snapshot_header_bar_no_path() {
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
        let area = Rect::new(0, 0, 80, 1);
        let mut buf = Buffer::empty(area);
        render_top_bar(&vm, area, &mut buf, &colors);
        insta::assert_snapshot!("grok_header_bar_no_path", buffer_to_string(&buf));
    }

    #[test]
    fn snapshot_header_bar_agent_running() {
        let vm = TopBarViewModel {
            repo: "runie".to_string(),
            branch: "main".to_string(),
            path: "~/Code/GitHub/runie".to_string(),
            context_window: 512_000,
            estimated_tokens: 45_000,
            agent_running: true,
            braille_frame: 3,
            mode: TuiMode::Chat,
        };
        let colors = make_test_colors();
        let area = Rect::new(0, 0, 80, 1);
        let mut buf = Buffer::empty(area);
        render_top_bar(&vm, area, &mut buf, &colors);
        insta::assert_snapshot!("grok_header_bar_agent_running", buffer_to_string(&buf));
    }

    #[test]
    fn snapshot_header_bar_homescreen_mode() {
        // HomeScreen mode hides the token meter
        let vm = TopBarViewModel {
            repo: "runie".to_string(),
            branch: "main".to_string(),
            path: "~/Code/GitHub/runie".to_string(),
            context_window: 512_000,
            estimated_tokens: 45_000,
            agent_running: false,
            braille_frame: 0,
            mode: TuiMode::HomeScreen,
        };
        let colors = make_test_colors();
        let area = Rect::new(0, 0, 80, 1);
        let mut buf = Buffer::empty(area);
        render_top_bar(&vm, area, &mut buf, &colors);
        insta::assert_snapshot!("grok_header_bar_homescreen", buffer_to_string(&buf));
    }
}

// ============================================================================
// 2. Welcome Screen Tests
// ============================================================================

mod welcome_screen {
    use super::*;

    #[test]
    fn snapshot_welcome_screen_menu() {
        let screen = HomeScreen::new();
        let theme = ThemeWrapper::default();
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        screen.render(area, &mut buf);
        insta::assert_snapshot!("grok_welcome_screen_menu", buffer_to_string(&buf));
    }

    #[test]
    fn snapshot_welcome_screen_selected_new_worktree() {
        let mut screen = HomeScreen::new();
        screen.move_down(); // Select second item
        let theme = ThemeWrapper::default();
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        screen.render(area, &mut buf);
        insta::assert_snapshot!("grok_welcome_screen_selected", buffer_to_string(&buf));
    }
}

// ============================================================================
// 3. Input Prompt Tests
// ============================================================================

mod input_prompt {
    use super::*;

    #[test]
    fn snapshot_input_prompt_normal_mode() {
        let textarea = ratatui_textarea::TextArea::new();
        let colors = make_test_colors();
        let area = Rect::new(0, 20, 80, 3);
        let mut buf = Buffer::empty(area);
        crate::components::input_bar::render_input_bar(
            &textarea,
            "❯ ",
            "",
            area,
            &mut buf,
            &colors,
            "Grok Build",
            &[],
            None,
            true,
        );
        insta::assert_snapshot!("grok_input_prompt_normal", buffer_to_string(&buf));
    }

    #[test]
    fn snapshot_input_prompt_plan_mode() {
        let textarea = ratatui_textarea::TextArea::new();
        let colors = make_test_colors();
        let area = Rect::new(0, 20, 80, 3);
        let mut buf = Buffer::empty(area);
        crate::components::input_bar::render_input_bar(
            &textarea,
            "❯ ",
            "",
            area,
            &mut buf,
            &colors,
            "Grok Build · plan",
            &[],
            None,
            true,
        );
        insta::assert_snapshot!("grok_input_prompt_plan", buffer_to_string(&buf));
    }

    #[test]
    fn snapshot_input_prompt_always_approve_mode() {
        let textarea = ratatui_textarea::TextArea::new();
        let colors = make_test_colors();
        let area = Rect::new(0, 20, 80, 3);
        let mut buf = Buffer::empty(area);
        crate::components::input_bar::render_input_bar(
            &textarea,
            "❯ ",
            "",
            area,
            &mut buf,
            &colors,
            "Grok Build · always-approve",
            &[],
            None,
            true,
        );
        insta::assert_snapshot!("grok_input_prompt_always_approve", buffer_to_string(&buf));
    }

    #[test]
    fn snapshot_input_prompt_with_text() {
        let mut textarea = ratatui_textarea::TextArea::new();
        textarea.insert_str("list files");
        let colors = make_test_colors();
        let area = Rect::new(0, 20, 80, 3);
        let mut buf = Buffer::empty(area);
        crate::components::input_bar::render_input_bar(
            &textarea,
            "❯ ",
            "",
            area,
            &mut buf,
            &colors,
            "Grok Build",
            &[],
            Some(11),
            true,
        );
        insta::assert_snapshot!("grok_input_prompt_with_text", buffer_to_string(&buf));
    }

    #[test]
    fn snapshot_input_prompt_unfocused() {
        let textarea = ratatui_textarea::TextArea::new();
        let colors = make_test_colors();
        let area = Rect::new(0, 20, 80, 3);
        let mut buf = Buffer::empty(area);
        crate::components::input_bar::render_input_bar(
            &textarea,
            "❯ ",
            "",
            area,
            &mut buf,
            &colors,
            "Grok Build",
            &[],
            None,
            false,
        );
        insta::assert_snapshot!("grok_input_prompt_unfocused", buffer_to_string(&buf));
    }
}

// ============================================================================
// 4. User Message Tests
// ============================================================================

mod user_message_tests {
    use super::*;

    #[test]
    fn snapshot_user_message_simple() {
        let vm = MessageListViewModel {
            messages: vec![
                MessageItem::User {
                    text: "list files".to_string(),
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
        let area = Rect::new(0, 2, 80, 18);
        let mut buf = Buffer::empty(area);
        MessageList::render_ref(&vm, area, &mut buf, &theme);
        insta::assert_snapshot!("grok_user_message_simple", buffer_to_string(&buf));
    }

    #[test]
    fn snapshot_user_message_no_timestamp() {
        let vm = MessageListViewModel {
            messages: vec![
                MessageItem::User {
                    text: "show me the code".to_string(),
                    model: None,
                    timestamp: None,
                },
            ],
            scroll_offset: 0,
            agent_running: false,
            animation: AnimationState::default(),
            wrap_cache: WrapCache::new(),
        };
        let theme = ThemeWrapper::default();
        let area = Rect::new(0, 2, 80, 18);
        let mut buf = Buffer::empty(area);
        MessageList::render_ref(&vm, area, &mut buf, &theme);
        insta::assert_snapshot!("grok_user_message_no_timestamp", buffer_to_string(&buf));
    }

    #[test]
    fn snapshot_user_message_long() {
        let vm = MessageListViewModel {
            messages: vec![
                MessageItem::User {
                    text: "Can you show me all the files in the src directory and its subdirectories, along with their sizes and modification times?".to_string(),
                    model: None,
                    timestamp: Some("4:15 PM".to_string()),
                },
            ],
            scroll_offset: 0,
            agent_running: false,
            animation: AnimationState::default(),
            wrap_cache: WrapCache::new(),
        };
        let theme = ThemeWrapper::default();
        let area = Rect::new(0, 2, 80, 18);
        let mut buf = Buffer::empty(area);
        MessageList::render_ref(&vm, area, &mut buf, &theme);
        insta::assert_snapshot!("grok_user_message_long", buffer_to_string(&buf));
    }
}

// ============================================================================
// 5. Thinking Block Tests
// ============================================================================

mod thinking_block {
    use super::*;

    #[test]
    fn snapshot_thinking_block_collapsed() {
        let block = ThinkingBlock {
            content: "The user wants to list files...".to_string(),
            duration_secs: 0.9,
            collapsed: true,
            animation_frame: 0,
        };
        let theme = ThemeWrapper::default();
        let area = Rect::new(0, 2, 80, 10);
        let mut buf = Buffer::empty(area);
        render_thinking_block(&block, area, &mut buf, &theme, 0, 2);
        insta::assert_snapshot!("grok_thinking_block_collapsed", buffer_to_string(&buf));
    }

    #[test]
    fn snapshot_thinking_block_expanded() {
        let block = ThinkingBlock {
            content: "The user said \"list src\". They want to list the source files, probably the src directories across the crates.".to_string(),
            duration_secs: 0.9,
            collapsed: false,
            animation_frame: 0,
        };
        let theme = ThemeWrapper::default();
        let area = Rect::new(0, 2, 80, 10);
        let mut buf = Buffer::empty(area);
        let rendered = render_thinking_block(&block, area, &mut buf, &theme, 0, 2);
        insta::assert_snapshot!("grok_thinking_block_expanded", buffer_to_string(&buf));
    }

    #[test]
    fn snapshot_thinking_block_streaming() {
        let block = ThinkingBlock {
            content: "Looking at the project structure to understand the layout...".to_string(),
            duration_secs: 1.2,
            collapsed: false,
            animation_frame: 5,
        };
        let theme = ThemeWrapper::default();
        let area = Rect::new(0, 2, 80, 10);
        let mut buf = Buffer::empty(area);
        render_thinking_block(&block, area, &mut buf, &theme, 0, 2);
        insta::assert_snapshot!("grok_thinking_block_streaming", buffer_to_string(&buf));
    }
}

// ============================================================================
// 6. Assistant Response Tests
// ============================================================================

mod assistant_response {
    use super::*;

    #[test]
    fn snapshot_assistant_simple() {
        let vm = MessageListViewModel {
            messages: vec![
                MessageItem::User {
                    text: "Hello".to_string(),
                    model: None,
                    timestamp: Some("11:28 PM".to_string()),
                },
                MessageItem::Assistant {
                    text: "Hello! How can I help you today?".to_string(),
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
        let area = Rect::new(0, 2, 80, 18);
        let mut buf = Buffer::empty(area);
        MessageList::render_ref(&vm, area, &mut buf, &theme);
        insta::assert_snapshot!("grok_assistant_simple", buffer_to_string(&buf));
    }

    #[test]
    fn snapshot_assistant_with_thinking() {
        let vm = MessageListViewModel {
            messages: vec![
                MessageItem::User {
                    text: "Hello".to_string(),
                    model: None,
                    timestamp: Some("11:28 PM".to_string()),
                },
                MessageItem::Assistant {
                    text: "Hello! How can I help?".to_string(),
                    model: Some("gpt-4o".to_string()),
                    timestamp: Some("11:28 PM".to_string()),
                    expanded: true,
                    thought_duration: Some(0.9),
                    turn_duration: Some(1.5),
                },
            ],
            scroll_offset: 0,
            agent_running: false,
            animation: AnimationState::default(),
            wrap_cache: WrapCache::new(),
        };
        let theme = ThemeWrapper::default();
        let area = Rect::new(0, 2, 80, 18);
        let mut buf = Buffer::empty(area);
        MessageList::render_ref(&vm, area, &mut buf, &theme);
        insta::assert_snapshot!("grok_assistant_with_thinking", buffer_to_string(&buf));
    }

    #[test]
    fn snapshot_assistant_streaming() {
        let vm = MessageListViewModel {
            messages: vec![
                MessageItem::User {
                    text: "Hello".to_string(),
                    model: None,
                    timestamp: Some("11:28 PM".to_string()),
                },
                MessageItem::Assistant {
                    text: "Thinking...".to_string(),
                    model: Some("gpt-4o".to_string()),
                    timestamp: None,
                    expanded: true,
                    thought_duration: Some(0.5),
                    turn_duration: None,
                },
            ],
            scroll_offset: 0,
            agent_running: true,
            animation: AnimationState::default(),
            wrap_cache: WrapCache::new(),
        };
        let theme = ThemeWrapper::default();
        let area = Rect::new(0, 2, 80, 18);
        let mut buf = Buffer::empty(area);
        MessageList::render_ref(&vm, area, &mut buf, &theme);
        insta::assert_snapshot!("grok_assistant_streaming", buffer_to_string(&buf));
    }
}

// ============================================================================
// 7. Tool Call Tests
// ============================================================================

mod tool_call {
    use super::*;

    #[test]
    fn snapshot_tool_call_running() {
        let block = ToolCallBlock {
            tool_name: "List".to_string(),
            args: "`.`".to_string(),
            status: ToolStatus::Running,
            elapsed_secs: 1.8,
            total_secs: 0.0,
            bytes_in: 0,
            spinner_frame: 2,
        };
        let theme = ThemeWrapper::default();
        let area = Rect::new(0, 2, 80, 10);
        let mut buf = Buffer::empty(area);
        render_tool_call_block(&block, area, &mut buf, &theme, 0, 2);
        insta::assert_snapshot!("grok_tool_call_running", buffer_to_string(&buf));
    }

    #[test]
    fn snapshot_tool_call_complete() {
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
        let area = Rect::new(0, 2, 80, 10);
        let mut buf = Buffer::empty(area);
        render_tool_call_block(&block, area, &mut buf, &theme, 0, 2);
        insta::assert_snapshot!("grok_tool_call_complete", buffer_to_string(&buf));
    }

    #[test]
    fn snapshot_tool_call_error() {
        let block = ToolCallBlock {
            tool_name: "bash".to_string(),
            args: r#"{"command": "rm file"}"#.to_string(),
            status: ToolStatus::Error,
            elapsed_secs: 0.0,
            total_secs: 0.5,
            bytes_in: 0,
            spinner_frame: 0,
        };
        let theme = ThemeWrapper::default();
        let area = Rect::new(0, 2, 80, 10);
        let mut buf = Buffer::empty(area);
        render_tool_call_block(&block, area, &mut buf, &theme, 0, 2);
        insta::assert_snapshot!("grok_tool_call_error", buffer_to_string(&buf));
    }

    #[test]
    fn snapshot_message_list_with_tool_call() {
        let vm = MessageListViewModel {
            messages: vec![
                MessageItem::User {
                    text: "what time is it?".to_string(),
                    model: None,
                    timestamp: Some("11:28 PM".to_string()),
                },
                MessageItem::Assistant {
                    text: "Sat May 30 09:30:16 -05 2026".to_string(),
                    model: Some("gpt-4o".to_string()),
                    timestamp: Some("11:28 PM".to_string()),
                    expanded: true,
                    thought_duration: Some(0.5),
                    turn_duration: Some(4.0),
                },
                MessageItem::ToolCall {
                    name: "date".to_string(),
                    args: "{}".to_string(),
                    result: Some("Sat May 30 09:30:16 -05 2026".to_string()),
                    is_error: false,
                },
            ],
            scroll_offset: 0,
            agent_running: false,
            animation: AnimationState::default(),
            wrap_cache: WrapCache::new(),
        };
        let theme = ThemeWrapper::default();
        let area = Rect::new(0, 2, 80, 18);
        let mut buf = Buffer::empty(area);
        MessageList::render_ref(&vm, area, &mut buf, &theme);
        insta::assert_snapshot!("grok_message_list_with_tool_call", buffer_to_string(&buf));
    }
}

// ============================================================================
// 8. Status Bar Tests
// ============================================================================

mod status_bar {
    use super::*;

    #[test]
    fn snapshot_status_bar_idle() {
        let vm = StatusBarViewModel {
            mode: TuiMode::Chat,
            current_model: Some("gpt-4o".to_string()),
            session_token_usage: TokenUsage {
                total_tokens: 5000,
                estimated_cost: 0.0234,
                ..Default::default()
            },
            agent_running: false,
            thinking_duration_secs: None,
            message_count: 5,
            max_messages: 50,
        };
        let colors = make_test_colors();
        let area = Rect::new(0, 23, 80, 1);
        let mut buf = Buffer::empty(area);
        render_status_bar(&vm, area, &mut buf, &colors);
        insta::assert_snapshot!("grok_status_bar_idle", buffer_to_string(&buf));
    }

    #[test]
    fn snapshot_status_bar_running() {
        let vm = StatusBarViewModel {
            mode: TuiMode::Chat,
            current_model: Some("gpt-4o".to_string()),
            session_token_usage: TokenUsage {
                total_tokens: 5000,
                estimated_cost: 0.0234,
                ..Default::default()
            },
            agent_running: true,
            thinking_duration_secs: Some(2.5),
            message_count: 5,
            max_messages: 50,
        };
        let colors = make_test_colors();
        let area = Rect::new(0, 23, 80, 1);
        let mut buf = Buffer::empty(area);
        render_status_bar(&vm, area, &mut buf, &colors);
        insta::assert_snapshot!("grok_status_bar_running", buffer_to_string(&buf));
    }

    #[test]
    fn snapshot_status_bar_plan_mode() {
        let vm = StatusBarViewModel {
            mode: TuiMode::Plan,
            current_model: Some("gpt-4o".to_string()),
            session_token_usage: TokenUsage {
                total_tokens: 5000,
                estimated_cost: 0.0234,
                ..Default::default()
            },
            agent_running: false,
            thinking_duration_secs: None,
            message_count: 5,
            max_messages: 50,
        };
        let colors = make_test_colors();
        let area = Rect::new(0, 23, 80, 1);
        let mut buf = Buffer::empty(area);
        render_status_bar(&vm, area, &mut buf, &colors);
        insta::assert_snapshot!("grok_status_bar_plan_mode", buffer_to_string(&buf));
    }
}

// ============================================================================
// 9. Full Conversation Snapshots
// ============================================================================

mod full_conversation {
    use super::*;

    #[test]
    fn snapshot_grok_style_conversation() {
        // Grok-style conversation from GROK.md
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
        let area = Rect::new(0, 2, 80, 24);
        let mut buf = Buffer::empty(area);
        MessageList::render_ref(&vm, area, &mut buf, &theme);
        insta::assert_snapshot!("grok_style_conversation", buffer_to_string(&buf));
    }

    #[test]
    fn snapshot_grok_empty_state() {
        let vm = MessageListViewModel {
            messages: vec![],
            scroll_offset: 0,
            agent_running: false,
            animation: AnimationState::default(),
            wrap_cache: WrapCache::new(),
        };
        let theme = ThemeWrapper::default();
        let area = Rect::new(0, 2, 80, 18);
        let mut buf = Buffer::empty(area);
        MessageList::render_ref(&vm, area, &mut buf, &theme);
        insta::assert_snapshot!("grok_empty_state", buffer_to_string(&buf));
    }
}

// ============================================================================
// 10. Permission Modal Tests (Grok-style)
// ============================================================================

mod permission_modal {
    use super::*;

    #[test]
    fn snapshot_permission_modal_grok_style() {
        let modal = PermissionModal::new(
            "bash",
            r#"{"command": "ls -la"}"#,
            "Lists files in the current directory",
        );
        let theme = ThemeWrapper::default();
        let area = Rect::new(0, 0, 60, 18);
        let mut buf = Buffer::empty(area);
        modal.render_ref(area, &mut buf, &theme);
        insta::assert_snapshot!("grok_permission_modal", buffer_to_string(&buf));
    }

    #[test]
    fn snapshot_permission_modal_dangerous() {
        let modal = PermissionModal::new(
            "bash",
            r#"rm -rf /"#,
            "⚠ This command will DELETE ALL FILES on your system!",
        );
        let theme = ThemeWrapper::default();
        let area = Rect::new(0, 0, 60, 18);
        let mut buf = Buffer::empty(area);
        modal.render_ref(area, &mut buf, &theme);
        insta::assert_snapshot!("grok_permission_modal_dangerous", buffer_to_string(&buf));
    }
}

// ============================================================================
// 11. Diff Viewer Tests
// ============================================================================

mod diff_viewer {
    use super::*;

    #[test]
    fn snapshot_diff_viewer_grok_style() {
        let diff = DiffViewer::new(
            "docs/install.md".to_string(),
            "Run the CLI and follow the prompts.".to_string(),
            "Install the CLI with curl -fsSL x.ai/cli/install.sh | bash\nRun `grok-build -p` to use the CLI in headless ACP-compatible mode.\nSign in once, then configure models and API keys in `config.toml`.".to_string(),
        );
        let theme = ThemeWrapper::default();
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        diff.render_ref(area, &mut buf, &theme);
        insta::assert_snapshot!("grok_diff_viewer", buffer_to_string(&buf));
    }
}
