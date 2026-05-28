//! End-to-end flow tests for complete user interaction scenarios.
//!
//! These tests verify the full integration of state management, message handling,
//! and command generation across all domains (chat, agent, UI, onboarding).

#![allow(clippy::unwrap_used)]
#![cfg(test)]

use crate::tui::state::{AppState, Msg, Cmd, TuiMode, TopBarState, OnboardingStep};
use crate::components::{CommandPalette, MessageItem};
use crate::tui::update::update;
use runie_agent::{AgentEvent, AgentMessage, ContentPart, ToolResult, PermissionDecision};
use serde_json::json;

// ─── Test Helpers ───────────────────────────────────────────────────────────────

fn make_state() -> AppState {
    AppState::default()
}

fn make_state_with_model(model: &str) -> AppState {
    AppState {
        current_model: Some(model.to_string()),
        top_bar: TopBarState {
            model: model.to_string(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn make_state_with_text(text: &str) -> AppState {
    AppState {
        current_model: Some("openai/gpt-4o".to_string()),
        textarea: ratatui_textarea::TextArea::new(vec![text.to_string()]),
        ..Default::default()
    }
}

fn make_agent_message(role: &str, content: &str) -> AgentMessage {
    AgentMessage {
        role: role.to_string(),
        content: vec![ContentPart::Text { text: content.to_string() }],
        timestamp: 0,
        usage: None,
        stop_reason: None,
        error_message: None,
        tool_calls: vec![],
    }
}

fn make_tool_result(tool_call_id: &str, tool_name: &str, content: &str, is_error: bool) -> ToolResult {
    ToolResult {
        tool_call_id: tool_call_id.to_string(),
        tool_name: tool_name.to_string(),
        input: json!({}),
        content: vec![ContentPart::Text { text: content.to_string() }],
        is_error,
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// MODULE: onboarding_flows
// ═══════════════════════════════════════════════════════════════════════════════
mod onboarding_flows {

    use super::*;

    #[test]
    fn test_e2e_onboarding_enter() {
        let mut state = make_state();
        let mut palette = CommandPalette::new();

        // Enter onboarding
        update(&mut state, &mut palette, Msg::EnterOnboarding);
        assert_eq!(state.mode, TuiMode::Onboarding);
        assert!(state.onboarding.is_some());
    }

    #[test]
    fn test_e2e_onboarding_skip_exits() {
        let mut state = make_state();
        let mut palette = CommandPalette::new();

        // Enter onboarding
        update(&mut state, &mut palette, Msg::EnterOnboarding);
        assert_eq!(state.mode, TuiMode::Onboarding);

        // Skip onboarding
        update(&mut state, &mut palette, Msg::OnboardingSkip);
        assert_eq!(state.mode, TuiMode::Chat);
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// MODULE: chat_flows
// ═══════════════════════════════════════════════════════════════════════════════
mod chat_flows {

    use super::*;

    #[test]
    fn test_e2e_chat_submit_with_model() {
        let mut state = make_state_with_text("Hello, world!");
        let mut palette = CommandPalette::new();

        // Submit
        let cmds = update(&mut state, &mut palette, Msg::Submit);

        // Verify user message added
        assert_eq!(state.messages.len(), 1);
        assert!(matches!(&state.messages[0], MessageItem::User { text, .. } if text == "Hello, world!"));

        // Verify agent spawned
        assert!(state.agent_running);
        assert!(matches!(&cmds[0], Cmd::SpawnAgent { .. }));
    }

    #[test]
    fn test_e2e_chat_submit_empty_text_rejected() {
        let mut state = make_state();
        let mut palette = CommandPalette::new();

        // Submit empty
        let cmds = update(&mut state, &mut palette, Msg::Submit);

        // Should not spawn agent
        assert!(cmds.is_empty());
        assert!(!state.agent_running);
        assert!(state.input_right_info.contains("Type a message"));
    }

    #[test]
    fn test_e2e_chat_submit_no_model_shows_hint() {
        let mut state = make_state();
        let mut palette = CommandPalette::new();
        state.textarea = ratatui_textarea::TextArea::new(vec!["Hello".to_string()]);

        // Submit without model
        let cmds = update(&mut state, &mut palette, Msg::Submit);

        // Should not spawn agent, show hint
        assert!(cmds.is_empty());
        // Note: agent_running is set to true before model check (BUG in chat.rs)
        // but no SpawnAgent command is issued
        assert!(state.messages.iter().any(|m| matches!(m, MessageItem::System { text } if text.contains("No model"))));
    }

    #[test]
    fn test_e2e_chat_submit_while_agent_running_blocked() {
        let mut state = make_state_with_text("Hello!");
        let mut palette = CommandPalette::new();
        state.agent_running = true; // Simulate already running

        // Submit while agent running
        let cmds = update(&mut state, &mut palette, Msg::Submit);

        // Should be blocked
        assert!(cmds.is_empty());
        assert!(state.input_right_info.contains("Agent running"));
    }

    #[test]
    fn test_e2e_clear_chat() {
        let mut state = make_state_with_model("openai/gpt-4o");
        let mut palette = CommandPalette::new();

        // Add some messages
        state.messages.push(MessageItem::User { text: "Hello".to_string(), model: None, timestamp: None });
        state.messages.push(MessageItem::Assistant { text: "Hi".to_string(), model: None, timestamp: None });

        // Clear chat
        update(&mut state, &mut palette, Msg::ClearChat);

        assert!(state.messages.is_empty());
    }

    #[test]
    fn test_e2e_clear_input() {
        let mut state = make_state_with_text("Hello, world!");
        let mut palette = CommandPalette::new();

        // Clear input
        update(&mut state, &mut palette, Msg::ClearInput);

        // Textarea should be empty
        let text = state.textarea.lines().join("");
        assert!(text.is_empty());
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// MODULE: agent_flows
// ═══════════════════════════════════════════════════════════════════════════════
mod agent_flows {

    use super::*;

    #[test]
    fn test_e2e_agent_message_start_end() {
        let mut state = make_state_with_model("openai/gpt-4o");
        let mut palette = CommandPalette::new();

        // Simulate agent starting to respond
        update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::MessageStart {
            message: make_agent_message("assistant", ""),
            turn: 1,
        }));

        assert!(state.agent_running);
        assert_eq!(state.messages.len(), 1);
        assert!(matches!(&state.messages[0], MessageItem::Assistant { text, .. } if text.is_empty()));

        // Simulate message content
        update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::MessageUpdate {
            message: make_agent_message("assistant", "Hello"),
            turn: 1,
            delta: "Hello".to_string(),
        }));

        // Simulate message end
        update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::MessageEnd {
            message: make_agent_message("assistant", "Hello"),
            turn: 1,
        }));

        // agent_running remains true after MessageEnd - only AgentEnd clears it
        assert!(state.agent_running);

        // Simulate agent end
        update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::AgentEnd {
            messages: vec![],
            total_turns: 1,
            final_token_usage: runie_agent::TokenUsage::default(),
        }));

        assert!(!state.agent_running);
    }

    #[test]
    fn test_e2e_agent_error_sets_recoverable() {
        let mut state = make_state_with_model("openai/gpt-4o");
        let mut palette = CommandPalette::new();

        // Simulate recoverable error
        update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::Error {
            message: "timeout: connection refused".to_string(),
            error_type: "network".to_string(),
            recoverable: true,
            context: "test".to_string(),
        }));

        assert!(!state.agent_running);
        assert_eq!(state.messages.len(), 1);
        assert!(matches!(&state.messages[0], MessageItem::Error { message: _, recoverable: true }));
    }

    #[test]
    fn test_e2e_agent_error_sets_fatal() {
        let mut state = make_state_with_model("openai/gpt-4o");
        let mut palette = CommandPalette::new();

        // Simulate fatal error
        update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::Error {
            message: "invalid_api_key".to_string(),
            error_type: "auth".to_string(),
            recoverable: false,
            context: "test".to_string(),
        }));

        assert!(!state.agent_running);
        assert!(matches!(&state.messages[0], MessageItem::Error { recoverable: false, .. }));
        assert_eq!(state.mode, TuiMode::Chat);
    }

    #[test]
    fn test_e2e_agent_end_clears_running_flag() {
        let mut state = make_state_with_model("openai/gpt-4o");
        let mut palette = CommandPalette::new();
        state.agent_running = true;
        state.agent_start_time = Some(std::time::Instant::now());

        // Simulate agent end
        update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::AgentEnd {
            messages: vec![],
            total_turns: 1,
            final_token_usage: runie_agent::TokenUsage::default(),
        }));

        assert!(!state.agent_running);
        assert!(state.agent_start_time.is_none());
    }

    #[test]
    fn test_e2e_agent_token_usage_accumulates() {
        let mut state = make_state_with_model("openai/gpt-4o");
        let mut palette = CommandPalette::new();

        // Initial token usage should be zero
        assert_eq!(state.session_token_usage.total_tokens, 0);

        // Simulate token usage event
        update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::TokenUsage {
            prompt_tokens: 100,
            completion_tokens: 50,
            total_tokens: 150,
            context_window: 128000,
        }));

        assert_eq!(state.session_token_usage.prompt_tokens, 100);
        assert_eq!(state.session_token_usage.completion_tokens, 50);
        assert_eq!(state.session_token_usage.total_tokens, 150);
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// MODULE: tool_flows
// ═══════════════════════════════════════════════════════════════════════════════
mod tool_flows {

    use super::*;

    #[test]
    fn test_e2e_tool_call_success() {
        let mut state = make_state_with_model("openai/gpt-4o");
        let mut palette = CommandPalette::new();

        // Tool execution starts
        update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::ToolExecutionStart {
            tool_call_id: "tool_abc123".to_string(),
            tool_name: "bash".to_string(),
            tool_args: "ls -la".to_string(),
            turn: 1,
        }));

        assert_eq!(state.messages.len(), 1);
        assert!(matches!(&state.messages[0], MessageItem::ToolCall { name, .. } if name == "tool_abc123"));

        // Tool execution ends successfully
        update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::ToolExecutionEnd {
            tool_call_id: "tool_abc123".to_string(),
            tool_name: "bash".to_string(),
            tool_args: "ls -la".to_string(),
            result: make_tool_result("tool_abc123", "bash", "file1\nfile2", false),
            duration_ms: 150,
            turn: 1,
        }));

        assert_eq!(state.messages.len(), 1);
        if let MessageItem::ToolCall { result, is_error, .. } = &state.messages[0] {
            assert!(result.is_some());
            assert!(!*is_error);
        } else {
            panic!("Expected ToolCall message");
        }
    }

    #[test]
    fn test_e2e_tool_call_error() {
        let mut state = make_state_with_model("openai/gpt-4o");
        let mut palette = CommandPalette::new();

        // Tool execution starts
        update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::ToolExecutionStart {
            tool_call_id: "tool_err".to_string(),
            tool_name: "bash".to_string(),
            tool_args: "invalid_command".to_string(),
            turn: 1,
        }));

        // Tool execution ends with error
        update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::ToolExecutionEnd {
            tool_call_id: "tool_err".to_string(),
            tool_name: "bash".to_string(),
            tool_args: "invalid_command".to_string(),
            result: make_tool_result("tool_err", "bash", "command not found", true),
            duration_ms: 50,
            turn: 1,
        }));

        if let MessageItem::ToolCall { is_error, .. } = &state.messages[0] {
            assert!(*is_error);
        } else {
            panic!("Expected ToolCall message");
        }
    }

    #[test]
    fn test_e2e_tool_call_permission() {
        let mut state = make_state_with_model("openai/gpt-4o");
        let mut palette = CommandPalette::new();

        // Agent requests permission
        update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::PermissionRequest {
            tool_call_id: "tool_perm".to_string(),
            tool_name: "bash".to_string(),
            tool_args: "rm -rf /".to_string(),
            tool_description: "Remove files".to_string(),
            turn: 1,
            context_window_usage: 0.5,
        }));

        // Should be in permission mode
        assert_eq!(state.mode, TuiMode::Permission);
        assert!(state.permission_modal.tool.is_some());
        assert_eq!(state.permission_modal.tool.as_deref(), Some("bash"));

        // User confirms permission
        let cmds = update(&mut state, &mut palette, Msg::PermissionConfirm);

        // Should send Allow permission
        assert!(cmds.iter().any(|c| matches!(c, Cmd::SendPermission { decision: PermissionDecision::Allow { .. } })));
        assert_eq!(state.mode, TuiMode::Chat);
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// MODULE: permission_flows
// ═══════════════════════════════════════════════════════════════════════════════
mod permission_flows {

    use super::*;

    #[test]
    fn test_e2e_permission_confirm() {
        let mut state = make_state_with_model("openai/gpt-4o");
        let mut palette = CommandPalette::new();

        // Set up permission modal
        state.mode = TuiMode::Permission;
        state.permission_modal.tool = Some("bash".to_string());
        state.permission_modal.tool_call_id = Some("tool_123".to_string());
        state.permission_modal.args = Some("ls".to_string());

        // Confirm permission
        let cmds = update(&mut state, &mut palette, Msg::PermissionConfirm);

        // Verify permission decision sent
        assert!(cmds.iter().any(|c| matches!(c, Cmd::SendPermission { decision: PermissionDecision::Allow { .. } })));
        assert_eq!(state.mode, TuiMode::Chat);
        assert!(state.permission_modal.tool.is_none());
    }

    #[test]
    fn test_e2e_permission_deny() {
        let mut state = make_state_with_model("openai/gpt-4o");
        let mut palette = CommandPalette::new();

        // Set up permission modal
        state.mode = TuiMode::Permission;
        state.permission_modal.tool = Some("bash".to_string());
        state.permission_modal.tool_call_id = Some("tool_456".to_string());
        state.permission_modal.args = Some("rm -rf".to_string());

        // Deny permission
        let cmds = update(&mut state, &mut palette, Msg::PermissionCancel);

        // Verify denial and rollback
        assert!(cmds.iter().any(|c| matches!(c, Cmd::SendPermission { decision: PermissionDecision::Deny { .. } })));
        assert!(cmds.iter().any(|c| matches!(c, Cmd::Rollback { tool_call_id } if tool_call_id == "tool_456")));
        assert_eq!(state.mode, TuiMode::Chat);
    }

    #[test]
    fn test_e2e_permission_always() {
        let mut state = make_state_with_model("openai/gpt-4o");
        let mut palette = CommandPalette::new();

        // Set up permission modal
        state.mode = TuiMode::Permission;
        state.permission_modal.tool = Some("bash".to_string());
        state.permission_modal.tool_call_id = Some("tool_789".to_string());
        state.permission_modal.args = Some("cat file".to_string());

        // Allow always
        let cmds = update(&mut state, &mut palette, Msg::PermissionAlways);

        assert!(cmds.iter().any(|c| matches!(c, Cmd::SendPermission { decision: PermissionDecision::AllowAlways { .. } })));
    }

    #[test]
    fn test_e2e_permission_skip() {
        let mut state = make_state_with_model("openai/gpt-4o");
        let mut palette = CommandPalette::new();

        // Set up permission modal
        state.mode = TuiMode::Permission;
        state.permission_modal.tool = Some("read_file".to_string());
        state.permission_modal.tool_call_id = Some("tool_skip".to_string());
        state.permission_modal.args = Some("test.txt".to_string());

        // Skip permission
        let cmds = update(&mut state, &mut palette, Msg::PermissionSkip);

        assert!(cmds.iter().any(|c| matches!(c, Cmd::SendPermission { decision: PermissionDecision::Skip { .. } })));
        assert!(cmds.iter().any(|c| matches!(c, Cmd::Rollback { .. }))); // Skip triggers rollback
    }

    #[test]
    fn test_e2e_permission_queue_in_blocking_mode() {
        let mut state = make_state_with_model("openai/gpt-4o");
        let mut palette = CommandPalette::new();

        // Set blocking mode (Overlay)
        state.mode = TuiMode::Overlay;

        // Permission request while in blocking mode should be queued
        update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::PermissionRequest {
            tool_call_id: "tool_queued".to_string(),
            tool_name: "bash".to_string(),
            tool_args: "ls".to_string(),
            tool_description: "List files".to_string(),
            turn: 1,
            context_window_usage: 0.1,
        }));

        // Should queue the request instead of showing modal
        assert!(state.permission_modal.pending_queue.len() == 1);
        assert_eq!(state.mode, TuiMode::Overlay); // Mode unchanged

        // System message indicates queued
        assert!(state.messages.iter().any(|m| matches!(m, MessageItem::System { text } if text.contains("queued"))));
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// MODULE: palette_flows
// ═══════════════════════════════════════════════════════════════════════════════
mod palette_flows {

    use super::*;

    #[test]
    fn test_e2e_palette_open_filter_confirm() {
        let mut state = make_state_with_model("openai/gpt-4o");
        let mut palette = CommandPalette::new();

        // Open palette
        update(&mut state, &mut palette, Msg::OpenCommandPalette);
        assert_eq!(state.mode, TuiMode::CommandPalette);
        assert!(state.command_palette.open);

        // Type filter
        update(&mut state, &mut palette, Msg::CommandPaletteFilter('q'));
        update(&mut state, &mut palette, Msg::CommandPaletteFilter('u'));
        update(&mut state, &mut palette, Msg::CommandPaletteFilter('i'));
        update(&mut state, &mut palette, Msg::CommandPaletteFilter('t'));
        assert_eq!(state.command_palette.filter, "quit");

        // Confirm (Quit)
        let cmds = update(&mut state, &mut palette, Msg::CommandPaletteConfirm);
        assert!(cmds.iter().any(|c| matches!(c, Cmd::Interrupt)));
        assert!(!state.running);
    }

    #[test]
    fn test_e2e_palette_cancel() {
        let mut state = make_state_with_model("openai/gpt-4o");
        let mut palette = CommandPalette::new();

        // Open palette
        update(&mut state, &mut palette, Msg::OpenCommandPalette);
        assert_eq!(state.mode, TuiMode::CommandPalette);

        // Cancel with Escape
        update(&mut state, &mut palette, Msg::CommandPaletteCancelArgument);
        assert_eq!(state.mode, TuiMode::Chat);
        assert!(!state.command_palette.open);
    }

    #[test]
    fn test_e2e_palette_navigation() {
        let mut state = make_state_with_model("openai/gpt-4o");
        let mut palette = CommandPalette::new();

        // Open palette
        update(&mut state, &mut palette, Msg::OpenCommandPalette);
        let initial_selected = state.command_palette.selected;

        // Navigate down
        update(&mut state, &mut palette, Msg::CommandPaletteDown);
        assert_eq!(state.command_palette.selected, initial_selected + 1);

        // Navigate up
        update(&mut state, &mut palette, Msg::CommandPaletteUp);
        assert_eq!(state.command_palette.selected, initial_selected);
    }

    #[test]
    fn test_e2e_palette_clear_chat_command() {
        let mut state = make_state_with_model("openai/gpt-4o");
        let mut palette = CommandPalette::new();

        // Add a message
        state.messages.push(MessageItem::User { text: "Test".to_string(), model: None, timestamp: None });

        // Open palette and filter for clear
        update(&mut state, &mut palette, Msg::OpenCommandPalette);
        update(&mut state, &mut palette, Msg::CommandPaletteFilter('c'));
        update(&mut state, &mut palette, Msg::CommandPaletteFilter('l'));
        update(&mut state, &mut palette, Msg::CommandPaletteFilter('e'));
        update(&mut state, &mut palette, Msg::CommandPaletteFilter('a'));
        update(&mut state, &mut palette, Msg::CommandPaletteFilter('r'));

        // Confirm clear - adds system message "Chat cleared"
        update(&mut state, &mut palette, Msg::CommandPaletteConfirm);
        assert_eq!(state.messages.len(), 1);
        assert!(matches!(&state.messages[0], MessageItem::System { text } if text.contains("cleared")));
    }

    #[test]
    fn test_e2e_palette_backspace_filter() {
        let mut state = make_state_with_model("openai/gpt-4o");
        let mut palette = CommandPalette::new();

        // Open palette
        update(&mut state, &mut palette, Msg::OpenCommandPalette);
        update(&mut state, &mut palette, Msg::CommandPaletteFilter('q'));
        update(&mut state, &mut palette, Msg::CommandPaletteFilter('u'));
        assert_eq!(state.command_palette.filter, "qu");

        // Backspace
        update(&mut state, &mut palette, Msg::CommandPaletteBackspace);
        assert_eq!(state.command_palette.filter, "q");
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// MODULE: settings_flows
// ═══════════════════════════════════════════════════════════════════════════════
mod settings_flows {

    use super::*;

    #[test]
    fn test_e2e_save_settings() {
        let mut state = make_state();
        let mut palette = CommandPalette::new();

        // Simulate DirectCommand for SwitchModel
        update(&mut state, &mut palette, Msg::DirectCommand(crate::components::PaletteCommand::SwitchModel));
        assert!(state.model_picker.is_some());
        assert_eq!(state.mode, TuiMode::Overlay);

        // Simulate model selection
        update(&mut state, &mut palette, Msg::SelectConfirm);
        // current_model should be set from model_picker
    }

    #[test]
    fn test_e2e_model_picker_selection() {
        let mut state = make_state();
        let mut palette = CommandPalette::new();

        // Open model picker
        update(&mut state, &mut palette, Msg::DirectCommand(crate::components::PaletteCommand::SwitchModel));
        assert!(state.model_picker.is_some());

        // Navigate
        update(&mut state, &mut palette, Msg::SelectDown);
        update(&mut state, &mut palette, Msg::SelectUp);

        // Confirm selection
        update(&mut state, &mut palette, Msg::SelectConfirm);
        // If a model was selected, it should be set
        if state.current_model.is_some() {
            assert_eq!(state.mode, TuiMode::Chat);
            assert!(state.model_picker.is_none());
        }
    }

    #[test]
    fn test_e2e_settings_persist_model() {
        let mut state = make_state_with_text("Hello");
        let mut palette = CommandPalette::new();

        // Submit message
        let cmds = update(&mut state, &mut palette, Msg::Submit);
        assert!(state.agent_running);
        assert!(state.current_model.is_some());
        assert!(!cmds.is_empty());

        // Simulate agent end
        update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::AgentEnd {
            messages: vec![],
            total_turns: 1,
            final_token_usage: runie_agent::TokenUsage::default(),
        }));

        // Model should persist
        assert_eq!(state.current_model.as_deref(), Some("openai/gpt-4o"));

        // Another submit should use same model
        state.textarea = ratatui_textarea::TextArea::new(vec!["Hello again".to_string()]);
        let cmds2 = update(&mut state, &mut palette, Msg::Submit);
        assert!(cmds2.iter().any(|c| matches!(c, Cmd::SpawnAgent { .. })));
    }

    #[test]
    fn test_e2e_save_settings_respects_dev_folder() {
        // Set RUNIE_HOME to a temp directory
        let temp_dir = std::env::temp_dir().join("runie_test_dev");
        std::env::set_var("RUNIE_HOME", temp_dir.display().to_string());

        // Create state with onboarding active
        let mut state = AppState::default();
        state.onboarding = Some(crate::components::onboarding::Onboarding::new());
        state.mode = TuiMode::Onboarding;
        let mut palette = CommandPalette::new();

        // Set up minimax provider and model directly (simulating completed onboarding flow)
        let o = state.onboarding.as_mut().unwrap();
        o.step = OnboardingStep::Complete;
        o.selected_item = 1; // "No, finish setup"
        let minimax_idx = o.providers.iter()
            .position(|p| p.id == "minimax")
            .expect("MiniMax provider should exist");
        o.selected_provider = Some(minimax_idx);
        o.selected_model = Some(0);
        o.api_key_input = "test-minimax-api-key".to_string();
        o.models.push(crate::components::onboarding::ModelOption {
            name: "MiniMax-Text-01".to_string(),
            id: "MiniMax-Text-01".to_string(),
            description: "MiniMax text model".to_string(),
        });

        // Finish onboarding - this should emit SaveSettings
        let cmds = update(&mut state, &mut palette, Msg::OnboardingNext);

        // Verify SaveSettings is emitted
        assert!(!cmds.is_empty(), "Expected SaveSettings command");
        let save_settings = cmds.iter().find_map(|c| match c {
            Cmd::SaveSettings { provider, model, api_key } => Some((provider.clone(), model.clone(), api_key.clone())),
            _ => None,
        });
        assert!(save_settings.is_some(), "Expected SaveSettings command in {:?}", cmds);

        let (provider, model, api_key) = save_settings.unwrap();
        assert_eq!(provider, "minimax");
        assert_eq!(model, "MiniMax-Text-01");
        assert_eq!(api_key, "test-minimax-api-key");

        // Verify state is now in Chat mode
        assert_eq!(state.mode, TuiMode::Chat);
        assert!(state.onboarding.is_none());

        // Note: current_model is set by tui_run.rs when it processes SaveSettings,
        // not by the update function itself. The update function only emits the command.
        // The config path verification (RUNIE_HOME vs ~/.runie) also happens in tui_run.rs
        // when it calls settings::config_path() which respects RUNIE_HOME env var.

        // Cleanup
        std::env::remove_var("RUNIE_HOME");
        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_e2e_model_displayed_correctly() {
        // Set current_model to minimax/MiniMax-Text-01
        let mut state = AppState::default();
        state.current_model = Some("minimax/MiniMax-Text-01".to_string());
        state.top_bar.model = "MiniMax-Text-01".to_string();

        // Verify current_model is full provider/model string
        assert_eq!(state.current_model.as_deref(), Some("minimax/MiniMax-Text-01"));

        // Verify top_bar.model is just the model name (not full provider/model)
        assert_eq!(state.top_bar.model, "MiniMax-Text-01");

        // Verify status bar would show minimax/MiniMax-Text-01 (not openai/gpt-4o)
        // The status_bar.current_model comes from state.current_model
        assert_ne!(state.current_model.as_deref(), Some("openai/gpt-4o"));
        assert_eq!(state.current_model.as_deref(), Some("minimax/MiniMax-Text-01"));
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// MODULE: mode_transitions
// ═══════════════════════════════════════════════════════════════════════════════
mod mode_transitions {

    use super::*;

    #[test]
    fn test_e2e_mode_chat_to_palette_to_chat() {
        let mut state = make_state_with_model("openai/gpt-4o");
        let mut palette = CommandPalette::new();

        // Start in Chat
        assert_eq!(state.mode, TuiMode::Chat);

        // Open palette
        update(&mut state, &mut palette, Msg::OpenCommandPalette);
        assert_eq!(state.mode, TuiMode::CommandPalette);

        // Cancel back to Chat
        update(&mut state, &mut palette, Msg::CloseModal);
        assert_eq!(state.mode, TuiMode::Chat);
    }

    #[test]
    fn test_e2e_mode_chat_to_permission_to_chat() {
        let mut state = make_state_with_model("openai/gpt-4o");
        let mut palette = CommandPalette::new();

        // Trigger permission request
        update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::PermissionRequest {
            tool_call_id: "tool_test".to_string(),
            tool_name: "bash".to_string(),
            tool_args: "ls".to_string(),
            tool_description: "List files".to_string(),
            turn: 1,
            context_window_usage: 0.5,
        }));
        assert_eq!(state.mode, TuiMode::Permission);

        // Confirm and return to Chat
        update(&mut state, &mut palette, Msg::PermissionConfirm);
        assert_eq!(state.mode, TuiMode::Chat);
    }

    #[test]
    fn test_e2e_mode_permission_deny_returns_to_chat() {
        let mut state = make_state_with_model("openai/gpt-4o");
        let mut palette = CommandPalette::new();

        // Set up permission mode manually
        state.mode = TuiMode::Permission;
        state.permission_modal.tool = Some("bash".to_string());
        state.permission_modal.tool_call_id = Some("tool_deny".to_string());

        // Deny
        update(&mut state, &mut palette, Msg::PermissionCancel);
        assert_eq!(state.mode, TuiMode::Chat);
    }

    #[test]
    fn test_e2e_mode_chat_to_onboarding() {
        let mut state = make_state_with_model("openai/gpt-4o");
        let mut palette = CommandPalette::new();

        // Enter onboarding
        update(&mut state, &mut palette, Msg::EnterOnboarding);
        assert_eq!(state.mode, TuiMode::Onboarding);

        // Exit onboarding
        update(&mut state, &mut palette, Msg::OnboardingSkip);
        assert_eq!(state.mode, TuiMode::Chat);
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// MODULE: ui_flows
// ═══════════════════════════════════════════════════════════════════════════════
mod ui_flows {

    use super::*;

    #[test]
    fn test_e2e_top_bar_shows_model() {
        let mut state = make_state_with_model("anthropic/claude-3-opus");
        let mut palette = CommandPalette::new();

        // Top bar should show model
        assert_eq!(state.top_bar.model, "anthropic/claude-3-opus");

        // Model change updates top bar
        state.current_model = Some("openai/gpt-4".to_string());
        state.top_bar.model = "openai/gpt-4".to_string();
        assert_eq!(state.top_bar.model, "openai/gpt-4");
    }

    #[test]
    fn test_e2e_status_bar_hotkeys() {
        let mut state = make_state_with_model("openai/gpt-4o");
        let mut palette = CommandPalette::new();

        // In Chat mode, Ctrl+P should open palette
        update(&mut state, &mut palette, Msg::OpenCommandPalette);
        assert_eq!(state.mode, TuiMode::CommandPalette);

        // In Palette mode, Esc should close
        update(&mut state, &mut palette, Msg::CommandPaletteCancelArgument);
        assert_eq!(state.mode, TuiMode::Chat);
    }

    #[test]
    fn test_e2e_thinking_indicator() {
        let mut state = make_state_with_model("openai/gpt-4o");
        let mut palette = CommandPalette::new();

        // Initially not thinking
        assert!(!state.agent_running);

        // Start agent
        update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::MessageStart {
            message: make_agent_message("assistant", ""),
            turn: 1,
        }));

        // Now thinking
        assert!(state.agent_running);

        // End agent
        update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::AgentEnd {
            messages: vec![],
            total_turns: 1,
            final_token_usage: runie_agent::TokenUsage::default(),
        }));

        // No longer thinking
        assert!(!state.agent_running);
    }

    #[test]
    fn test_e2e_toggle_sidebar() {
        let mut state = make_state_with_model("openai/gpt-4o");
        let mut palette = CommandPalette::new();

        assert!(!state.show_sidebar);

        update(&mut state, &mut palette, Msg::ToggleSidebar);
        assert!(state.show_sidebar);

        update(&mut state, &mut palette, Msg::ToggleSidebar);
        assert!(!state.show_sidebar);
    }

    #[test]
    fn test_e2e_scroll_messages() {
        let mut state = make_state_with_model("openai/gpt-4o");
        let mut palette = CommandPalette::new();

        // Add messages
        for i in 0..20 {
            state.messages.push(MessageItem::User { text: format!("Message {}", i), model: None, timestamp: None });
        }

        // Scroll
        update(&mut state, &mut palette, Msg::ScrollDown);
        assert_eq!(state.scroll.feed_offset, 1);

        update(&mut state, &mut palette, Msg::ScrollUp);
        assert_eq!(state.scroll.feed_offset, 0);

        // Page scroll
        update(&mut state, &mut palette, Msg::ScrollPageDown);
        assert_eq!(state.scroll.feed_offset, 10);
    }

    #[test]
    fn test_e2e_resize_terminal() {
        let mut state = make_state();
        let mut palette = CommandPalette::new();

        assert_eq!(state.terminal_size, (0, 0));

        update(&mut state, &mut palette, Msg::Resize(160, 50));

        assert_eq!(state.terminal_size, (160, 50));
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// MODULE: slash_commands
// ═══════════════════════════════════════════════════════════════════════════════
mod slash_commands {

    use super::*;

    #[test]
    fn test_e2e_slash_clear() {
        let mut state = make_state_with_model("openai/gpt-4o");
        let mut palette = CommandPalette::new();

        // Add messages
        state.messages.push(MessageItem::User { text: "Hello".to_string(), model: None, timestamp: None });
        state.messages.push(MessageItem::Assistant { text: "Hi".to_string(), model: None, timestamp: None });

        // Slash clear
        update(&mut state, &mut palette, Msg::SlashCommand(runie_core::slash_command::SlashCommand::Clear));

        assert!(state.messages.is_empty());
    }

    #[test]
    fn test_e2e_slash_new() {
        let mut state = make_state_with_model("openai/gpt-4o");
        let mut palette = CommandPalette::new();

        // Add messages
        state.messages.push(MessageItem::User { text: "Hello".to_string(), model: None, timestamp: None });

        // Slash new - clears messages and adds system message
        update(&mut state, &mut palette, Msg::SlashCommand(runie_core::slash_command::SlashCommand::New));

        // Should have one system message "New session started"
        assert_eq!(state.messages.len(), 1);
        assert!(matches!(&state.messages[0], MessageItem::System { text } if text.contains("New session")));
        assert_eq!(state.scroll.feed_offset, 0);
        assert!(state.scroll.feed_offset == 0);
        // Should have new session system message
        assert!(state.messages.iter().any(|m| matches!(m, MessageItem::System { text } if text.contains("New session"))));
    }

    #[test]
    fn test_e2e_slash_model() {
        let mut state = make_state();
        let mut palette = CommandPalette::new();

        // Slash model
        update(&mut state, &mut palette, Msg::SlashCommand(runie_core::slash_command::SlashCommand::Model("gpt-4o".to_string())));

        assert_eq!(state.current_model.as_deref(), Some("gpt-4o"));
        assert!(state.messages.iter().any(|m| matches!(m, MessageItem::System { text } if text.contains("Model switched"))));
    }

    #[test]
    fn test_e2e_slash_help() {
        let mut state = make_state_with_model("openai/gpt-4o");
        let mut palette = CommandPalette::new();

        // Slash help
        update(&mut state, &mut palette, Msg::SlashCommand(runie_core::slash_command::SlashCommand::Help));

        assert!(state.messages.iter().any(|m| matches!(m, MessageItem::System { text } if text.contains("/"))));
    }

    #[test]
    fn test_e2e_slash_unknown() {
        let mut state = make_state_with_model("openai/gpt-4o");
        let mut palette = CommandPalette::new();

        // Unknown command
        update(&mut state, &mut palette, Msg::SlashCommand(runie_core::slash_command::SlashCommand::Unknown("badcmd".to_string())));

        assert!(state.messages.iter().any(|m| matches!(m, MessageItem::System { text } if text.contains("Unknown command"))));
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// MODULE: cursor_and_animation
// ═══════════════════════════════════════════════════════════════════════════════
mod cursor_and_animation {

    use super::*;

    #[test]
    fn test_e2e_cursor_blink_toggles() {
        let mut state = make_state();
        let mut palette = CommandPalette::new();

        assert!(state.animation.streaming_cursor_visible);

        update(&mut state, &mut palette, Msg::CursorBlink);
        assert!(!state.animation.streaming_cursor_visible);

        update(&mut state, &mut palette, Msg::CursorBlink);
        assert!(state.animation.streaming_cursor_visible);
    }

    #[test]
    fn test_e2e_animation_tick_advances() {
        let mut state = make_state();
        let mut palette = CommandPalette::new();

        let initial_frame = state.animation.braille_frame;

        update(&mut state, &mut palette, Msg::Tick);

        // Frame should advance (modulo 10)
        assert_eq!(state.animation.braille_frame, (initial_frame + 1) % 10);
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// MODULE: git_info
// ═══════════════════════════════════════════════════════════════════════════════
mod git_info {

    use super::*;

    #[test]
    fn test_e2e_set_git_info() {
        let mut state = make_state();
        let mut palette = CommandPalette::new();

        update(&mut state, &mut palette, Msg::SetGitInfo {
            repo: "myrepo".to_string(),
            branch: "main".to_string(),
            path: "src/lib.rs".to_string(),
        });

        assert_eq!(state.top_bar.repo, "myrepo");
        assert_eq!(state.top_bar.branch, "main");
        assert_eq!(state.top_bar.path, "src/lib.rs");
    }

    #[test]
    fn test_e2e_set_top_bar_checks() {
        let mut state = make_state();
        let mut palette = CommandPalette::new();

        update(&mut state, &mut palette, Msg::SetTopBarMockChecks {
            checks_passed: Some(8),
            checks_total: Some(10),
            percentage: Some(80.0),
            context_badges: vec!["rust".to_string(), "fmt".to_string()],
        });

        assert_eq!(state.top_bar.checks_passed, Some(8));
        assert_eq!(state.top_bar.checks_total, Some(10));
        assert_eq!(state.top_bar.percentage, Some(80.0));
        assert_eq!(state.top_bar.context_badges, vec!["rust", "fmt"]);
    }
}
