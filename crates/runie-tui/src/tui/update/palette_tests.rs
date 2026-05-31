//! Tests for palette update functions - handle_direct_command and handle_close_modal.

use crate::tui::state::{AppState, TuiMode, Msg};
use crate::components::{MessageItem, CommandPalette};
use crate::components::command_palette::PaletteCommand;
use crate::tui::update::palette::{handle_direct_command, handle_close_modal};
use crate::tui::update::update;

fn make_state() -> AppState {
    AppState {
        mode: TuiMode::Chat,
        running: true,
        ..Default::default()
    }
}

fn make_palette() -> CommandPalette {
    CommandPalette::new()
}

mod handle_direct_command_tests {
    use super::*;

    #[test]
    fn test_new_session_clears_messages() {
        let mut state = make_state();
        state.messages.push(MessageItem::User { text: "old".to_string(), model: None, timestamp: None });

        let cmds = handle_direct_command(&mut state, PaletteCommand::NewSession);

        // Messages cleared and system message added
        assert_eq!(state.messages.len(), 1);
        assert!(matches!(&state.messages[0], MessageItem::System { text } if text.contains("New session")));
        assert!(cmds.is_empty());
        assert_eq!(state.mode, TuiMode::Chat);
    }

    #[test]
    fn test_clear_chat_clears_messages() {
        let mut state = make_state();
        state.messages.push(MessageItem::User { text: "hello".to_string(), model: None, timestamp: None });
        state.messages.push(MessageItem::Assistant { text: "hi".to_string(), model: None, timestamp: None });

        let cmds = handle_direct_command(&mut state, PaletteCommand::ClearChat);

        // Old messages cleared - no system message added for clear
        assert_eq!(state.messages.len(), 0);
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_quit_sets_running_false() {
        let mut state = make_state();
        assert!(state.running);

        let cmds = handle_direct_command(&mut state, PaletteCommand::Quit);

        assert!(!state.running);
        // Quit produces UiCmd::Quit which becomes Cmd::Interrupt
        assert!(!cmds.is_empty());
        // Quit does not add a goodbye message
        assert!(!state.messages.iter().any(|m| matches!(m, MessageItem::System { text } if text.contains("Goodbye"))));
    }

    #[test]
    fn test_switch_model_opens_model_picker() {
        let mut state = make_state();

        let cmds = handle_direct_command(&mut state, PaletteCommand::SwitchModel);

        // No commands issued - just state change
        assert!(cmds.is_empty());
        // Should switch to Overlay mode for model picker
        assert_eq!(state.mode, TuiMode::Overlay);
        // Model picker should be populated with grouped models
        assert!(state.model_picker.is_some());
        let picker = state.model_picker.as_ref().unwrap();
        // Should have 3 providers: Anthropic, OpenAI, Google
        assert_eq!(picker.providers.len(), 3);
        // First provider should be Anthropic
        assert_eq!(picker.providers[0].provider_name, "Anthropic");
        // Selected should be first model
        assert_eq!(picker.selected, (0, 0));
    }

    #[test]
    fn test_cancel_does_nothing() {
        let mut state = make_state();
        state.messages.push(MessageItem::User { text: "hello".to_string(), model: None, timestamp: None });

        let cmds = handle_direct_command(&mut state, PaletteCommand::Cancel);

        assert!(cmds.is_empty());
        assert_eq!(state.messages.len(), 1); // Message preserved
    }
}

mod handle_close_modal_tests {
    use super::*;

    #[test]
    fn test_close_modal_resets_mode_to_chat() {
        let mut state = make_state();
        state.mode = TuiMode::CommandPalette;
        state.command_palette.open = true;

        handle_close_modal(&mut state);

        assert_eq!(state.mode, TuiMode::Chat);
    }

    #[test]
    fn test_close_modal_closes_palette() {
        let mut state = make_state();
        state.mode = TuiMode::CommandPalette;
        state.command_palette.open = true;

        handle_close_modal(&mut state);

        assert!(!state.command_palette.open);
    }

    #[test]
    fn test_close_modal_clears_filter() {
        let mut state = make_state();
        state.command_palette.filter = "test filter".to_string();

        handle_close_modal(&mut state);

        assert!(state.command_palette.filter.is_empty());
    }

    #[test]
    fn test_close_modal_resets_selection() {
        let mut state = make_state();
        state.command_palette.selected = 5;

        handle_close_modal(&mut state);

        assert_eq!(state.command_palette.selected, 0);
    }

    #[test]
    fn test_close_modal_clears_permission_modal() {
        let mut state = make_state();
        state.permission_modal.tool = Some("bash".to_string());
        state.permission_modal.tool_call_id = Some("tool_123".to_string());

        handle_close_modal(&mut state);

        assert!(state.permission_modal.tool.is_none());
        assert!(state.permission_modal.tool_call_id.is_none());
    }

    #[test]
    fn test_close_modal_clears_diff_viewer() {
        use crate::components::DiffViewer;
        let mut state = make_state();
        state.diff_viewer = Some(DiffViewer::new("test.txt".to_string(), "old".to_string(), "new".to_string()));

        handle_close_modal(&mut state);

        assert!(state.diff_viewer.is_none());
    }

    #[test]
    fn test_close_modal_hides_session_tree() {
        use crate::components::SessionTreeNavigator;
        let mut state = make_state();
        state.session_tree = SessionTreeNavigator::new();
        state.session_tree.show();

        handle_close_modal(&mut state);

        assert!(!state.session_tree.visible);
    }
}

mod palette_integration_tests {
    use super::*;

    #[test]
    fn test_full_flow_new_session() {
        let mut state = make_state();
        state.messages.push(MessageItem::User { text: "old message".to_string(), model: None, timestamp: None });
        state.mode = TuiMode::CommandPalette;
        state.command_palette.open = true;

        let cmds = update(&mut state, &mut make_palette(), Msg::DirectCommand(PaletteCommand::NewSession));

        assert!(state.messages.is_empty() || state.messages.iter().any(|m| matches!(m, MessageItem::System { .. })));
        assert_eq!(state.mode, TuiMode::Chat);
        assert!(cmds.is_empty());
    }

}

mod command_execution_tests {
    use super::*;
    use crate::tui::update::ui::UiCmd;

    // =====================================================================
    // 1. NewSession - clears messages, adds "New session started", returns to Chat
    // =====================================================================

    #[test]
    fn test_new_session_clears_all_messages() {
        let mut state = make_state();
        state.messages.push(MessageItem::User { text: "user msg".to_string(), model: None, timestamp: None });
        state.messages.push(MessageItem::Assistant { text: "assistant msg".to_string(), model: None, timestamp: None });
        state.messages.push(MessageItem::System { text: "system msg".to_string() });

        let cmds = handle_direct_command(&mut state, PaletteCommand::NewSession);

        // Old messages cleared, but "New session started" system message added
        assert_eq!(state.messages.len(), 1);
        assert!(matches!(&state.messages[0], MessageItem::System { text } if text.contains("New session")));
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_new_session_adds_system_message() {
        let mut state = make_state();

        let cmds = handle_direct_command(&mut state, PaletteCommand::NewSession);

        assert_eq!(state.messages.len(), 1);
        assert!(matches!(&state.messages[0], MessageItem::System { text } if text.contains("New session started")));
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_new_session_returns_to_chat_mode() {
        let mut state = make_state();
        state.mode = TuiMode::CommandPalette;
        state.command_palette.open = true;

        let cmds = handle_direct_command(&mut state, PaletteCommand::NewSession);

        assert_eq!(state.mode, TuiMode::Chat);
        assert!(!state.command_palette.open); // Palette is closed by run_slash
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_new_session_resets_scroll_offset() {
        let mut state = make_state();
        state.scroll.feed_offset = 100;

        let cmds = handle_direct_command(&mut state, PaletteCommand::NewSession);

        assert_eq!(state.scroll.feed_offset, 0);
        assert!(cmds.is_empty());
    }

    // =====================================================================
    // 2. ClearChat - clears all messages, returns to Chat
    // =====================================================================

    #[test]
    fn test_clear_chat_removes_all_messages() {
        let mut state = make_state();
        state.messages.push(MessageItem::User { text: "user".to_string(), model: None, timestamp: None });
        state.messages.push(MessageItem::Assistant { text: "assistant".to_string(), model: None, timestamp: None });
        state.messages.push(MessageItem::System { text: "system".to_string() });

        let cmds = handle_direct_command(&mut state, PaletteCommand::ClearChat);

        assert!(state.messages.is_empty());
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_clear_chat_does_not_add_system_message() {
        let mut state = make_state();

        let cmds = handle_direct_command(&mut state, PaletteCommand::ClearChat);

        // ClearChat should NOT add a system message
        assert!(state.messages.is_empty());
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_clear_chat_returns_to_chat_mode() {
        let mut state = make_state();
        state.mode = TuiMode::CommandPalette;

        let cmds = handle_direct_command(&mut state, PaletteCommand::ClearChat);

        assert_eq!(state.mode, TuiMode::Chat);
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_clear_chat_resets_scroll_offset() {
        let mut state = make_state();
        state.scroll.feed_offset = 50;

        let cmds = handle_direct_command(&mut state, PaletteCommand::ClearChat);

        assert_eq!(state.scroll.feed_offset, 0);
        assert!(cmds.is_empty());
    }

    // =====================================================================
    // 3. SwitchModel - opens model picker overlay (mode = Overlay)
    // =====================================================================

    #[test]
    fn test_switch_model_sets_overlay_mode() {
        let mut state = make_state();

        let cmds = handle_direct_command(&mut state, PaletteCommand::SwitchModel);

        assert_eq!(state.mode, TuiMode::Overlay);
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_switch_model_creates_model_picker() {
        let mut state = make_state();

        let cmds = handle_direct_command(&mut state, PaletteCommand::SwitchModel);

        assert!(state.model_picker.is_some());
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_switch_model_has_correct_providers() {
        let mut state = make_state();

        let cmds = handle_direct_command(&mut state, PaletteCommand::SwitchModel);

        let picker = state.model_picker.as_ref().unwrap();
        assert_eq!(picker.providers.len(), 3);
        assert_eq!(picker.providers[0].provider_name, "Anthropic");
        assert_eq!(picker.providers[1].provider_name, "OpenAI");
        assert_eq!(picker.providers[2].provider_name, "Google");
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_switch_model_initializes_selection() {
        let mut state = make_state();

        let cmds = handle_direct_command(&mut state, PaletteCommand::SwitchModel);

        let picker = state.model_picker.as_ref().unwrap();
        assert_eq!(picker.selected, (0, 0)); // First provider, first model
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_switch_model_does_not_clear_messages() {
        let mut state = make_state();
        state.messages.push(MessageItem::User { text: "hello".to_string(), model: None, timestamp: None });

        let cmds = handle_direct_command(&mut state, PaletteCommand::SwitchModel);

        assert_eq!(state.messages.len(), 1);
        assert!(cmds.is_empty());
    }

    // =====================================================================
    // 4. ForkSession - adds fork system message, returns to Chat mode
    // =====================================================================

    #[test]
    fn test_fork_session_adds_fork_message() {
        let mut state = make_state();

        let cmds = handle_direct_command(&mut state, PaletteCommand::ForkSession);

        assert_eq!(state.messages.len(), 1);
        assert!(matches!(&state.messages[0], MessageItem::System { text } if text.contains("Fork created")));
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_fork_session_preserves_existing_messages() {
        let mut state = make_state();
        state.messages.push(MessageItem::User { text: "original".to_string(), model: None, timestamp: None });

        let cmds = handle_direct_command(&mut state, PaletteCommand::ForkSession);

        // Fork adds message but doesn't clear existing
        assert_eq!(state.messages.len(), 2);
        assert!(matches!(&state.messages[1], MessageItem::System { text } if text.contains("Fork created")));
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_fork_session_returns_to_chat_mode() {
        let mut state = make_state();
        state.mode = TuiMode::CommandPalette;

        let cmds = handle_direct_command(&mut state, PaletteCommand::ForkSession);

        assert_eq!(state.mode, TuiMode::Chat);
        assert!(cmds.is_empty());
    }

    // =====================================================================
    // 5. SessionTree - opens session tree (mode changes)
    // =====================================================================

    #[test]
    fn test_session_tree_toggles_visible() {
        let mut state = make_state();
        assert!(!state.session_tree.visible);

        let cmds = handle_direct_command(&mut state, PaletteCommand::SessionTree);

        assert!(state.session_tree.visible);
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_session_tree_sets_session_tree_mode() {
        let mut state = make_state();

        let cmds = handle_direct_command(&mut state, PaletteCommand::SessionTree);

        assert_eq!(state.mode, TuiMode::SessionTree);
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_session_tree_toggles_off_when_called_again() {
        let mut state = make_state();

        // First call - opens
        let cmds1 = handle_direct_command(&mut state, PaletteCommand::SessionTree);
        assert!(state.session_tree.visible);
        assert_eq!(state.mode, TuiMode::SessionTree);

        // Second call - closes
        let cmds2 = handle_direct_command(&mut state, PaletteCommand::SessionTree);
        assert!(!state.session_tree.visible);
        assert_eq!(state.mode, TuiMode::Chat);

        assert!(cmds1.is_empty());
        assert!(cmds2.is_empty());
    }

    #[test]
    fn test_session_tree_does_not_clear_messages() {
        let mut state = make_state();
        state.messages.push(MessageItem::User { text: "hello".to_string(), model: None, timestamp: None });

        let cmds = handle_direct_command(&mut state, PaletteCommand::SessionTree);

        assert_eq!(state.messages.len(), 1);
        assert!(cmds.is_empty());
    }

    // =====================================================================
    // 6. Onboard - opens onboarding (mode = Onboarding)
    // =====================================================================

    #[test]
    fn test_onboard_sets_onboarding_mode() {
        let mut state = make_state();

        let cmds = handle_direct_command(&mut state, PaletteCommand::Onboard);

        assert_eq!(state.mode, TuiMode::Onboarding);
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_onboard_creates_onboarding_state() {
        let mut state = make_state();

        let cmds = handle_direct_command(&mut state, PaletteCommand::Onboard);

        assert!(state.onboarding.is_some());
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_onboard_does_not_clear_messages() {
        let mut state = make_state();
        state.messages.push(MessageItem::User { text: "hello".to_string(), model: None, timestamp: None });

        let cmds = handle_direct_command(&mut state, PaletteCommand::Onboard);

        assert_eq!(state.messages.len(), 1);
        assert!(cmds.is_empty());
    }

    // =====================================================================
    // 7. CopyLast - copies last assistant message
    // =====================================================================

    #[test]
    fn test_copy_last_with_assistant_message_adds_confirmation() {
        let mut state = make_state();
        state.messages.push(MessageItem::User { text: "hello".to_string(), model: None, timestamp: None });
        state.messages.push(MessageItem::Assistant { text: "assistant response".to_string(), model: None, timestamp: None });

        let cmds = handle_direct_command(&mut state, PaletteCommand::CopyLast);

        // Should add system confirmation message
        assert_eq!(state.messages.len(), 3);
        assert!(matches!(&state.messages[2], MessageItem::System { text } if text.contains("Copied")));
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_copy_last_without_assistant_message_adds_error_message() {
        let mut state = make_state();
        state.messages.push(MessageItem::User { text: "hello".to_string(), model: None, timestamp: None });

        let cmds = handle_direct_command(&mut state, PaletteCommand::CopyLast);

        // Should add system error message
        assert_eq!(state.messages.len(), 2);
        assert!(matches!(&state.messages[1], MessageItem::System { text } if text.contains("No assistant response")));
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_copy_last_with_empty_messages_adds_error_message() {
        let mut state = make_state();

        let cmds = handle_direct_command(&mut state, PaletteCommand::CopyLast);

        assert_eq!(state.messages.len(), 1);
        assert!(matches!(&state.messages[0], MessageItem::System { text } if text.contains("No assistant response")));
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_copy_last_copies_last_assistant_not_first() {
        let mut state = make_state();
        state.messages.push(MessageItem::Assistant { text: "first response".to_string(), model: None, timestamp: None });
        state.messages.push(MessageItem::User { text: "question".to_string(), model: None, timestamp: None });
        state.messages.push(MessageItem::Assistant { text: "last response".to_string(), model: None, timestamp: None });

        let cmds = handle_direct_command(&mut state, PaletteCommand::CopyLast);

        // Should find last assistant (which is "last response")
        assert!(matches!(&state.messages[3], MessageItem::System { text } if text.contains("Copied")));
        assert!(cmds.is_empty());
    }

    // =====================================================================
    // 8. ShowCost - shows cost info
    // =====================================================================

    #[test]
    fn test_show_cost_adds_cost_message() {
        let mut state = make_state();

        let cmds = handle_direct_command(&mut state, PaletteCommand::ShowCost);

        assert_eq!(state.messages.len(), 1);
        assert!(matches!(&state.messages[0], MessageItem::System { text } if text.contains("Session usage")));
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_show_cost_includes_token_count() {
        let mut state = make_state();
        state.session_token_usage.total_tokens = 12345;

        let cmds = handle_direct_command(&mut state, PaletteCommand::ShowCost);

        assert!(matches!(&state.messages[0], MessageItem::System { text } if text.contains("12345")));
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_show_cost_includes_cost_estimate() {
        let mut state = make_state();
        state.session_token_usage.estimated_cost = 0.0250;

        let cmds = handle_direct_command(&mut state, PaletteCommand::ShowCost);

        assert!(matches!(&state.messages[0], MessageItem::System { text } if text.contains("$0.025")));
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_show_cost_does_not_clear_messages() {
        let mut state = make_state();
        state.messages.push(MessageItem::User { text: "hello".to_string(), model: None, timestamp: None });

        let cmds = handle_direct_command(&mut state, PaletteCommand::ShowCost);

        assert_eq!(state.messages.len(), 2);
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_show_cost_returns_to_chat_mode() {
        let mut state = make_state();
        state.mode = TuiMode::CommandPalette;

        let cmds = handle_direct_command(&mut state, PaletteCommand::ShowCost);

        assert_eq!(state.mode, TuiMode::Chat);
        assert!(cmds.is_empty());
    }

    // =====================================================================
    // 9. Help - adds help system message
    // =====================================================================

    #[test]
    fn test_help_adds_help_message() {
        let mut state = make_state();

        let cmds = handle_direct_command(&mut state, PaletteCommand::Help);

        assert_eq!(state.messages.len(), 1);
        assert!(matches!(&state.messages[0], MessageItem::System { text } if text.contains("Available commands")));
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_help_includes_slash_commands() {
        let mut state = make_state();

        let cmds = handle_direct_command(&mut state, PaletteCommand::Help);

        let text = match &state.messages[0] {
            MessageItem::System { text } => text.clone(),
            _ => String::new(),
        };
        // Help should contain info about slash commands
        assert!(text.contains("/new") || text.contains("/clear") || text.contains("/quit"));
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_help_does_not_clear_messages() {
        let mut state = make_state();
        state.messages.push(MessageItem::User { text: "hello".to_string(), model: None, timestamp: None });

        let cmds = handle_direct_command(&mut state, PaletteCommand::Help);

        assert_eq!(state.messages.len(), 2);
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_help_returns_to_chat_mode() {
        let mut state = make_state();
        state.mode = TuiMode::CommandPalette;

        let cmds = handle_direct_command(&mut state, PaletteCommand::Help);

        assert_eq!(state.mode, TuiMode::Chat);
        assert!(cmds.is_empty());
    }

    // =====================================================================
    // 10. Quit - sets running = false, returns UiCmd::Quit
    // =====================================================================

    #[test]
    fn test_quit_sets_running_to_false() {
        let mut state = make_state();
        assert!(state.running);

        let cmds = handle_direct_command(&mut state, PaletteCommand::Quit);

        assert!(!state.running);
    }

    #[test]
    fn test_quit_returns_ui_cmd_quit() {
        let mut state = make_state();

        let cmds = handle_direct_command(&mut state, PaletteCommand::Quit);

        assert!(!cmds.is_empty());
        // Should return UiCmd::Quit
        use crate::tui::update::ui::UiCmd;
        assert!(cmds.iter().any(|c| matches!(c, UiCmd::Quit)));
    }

    #[test]
    fn test_quit_does_not_clear_messages() {
        let mut state = make_state();
        state.messages.push(MessageItem::User { text: "hello".to_string(), model: None, timestamp: None });

        let cmds = handle_direct_command(&mut state, PaletteCommand::Quit);

        // Quit should not modify messages
        assert_eq!(state.messages.len(), 1);
        assert!(cmds.iter().any(|c| matches!(c, UiCmd::Quit)));
    }

    #[test]
    fn test_quit_does_not_add_goodbye_message() {
        let mut state = make_state();

        let cmds = handle_direct_command(&mut state, PaletteCommand::Quit);

        // Quit should NOT add a goodbye message (that's a different behavior)
        assert!(!state.messages.iter().any(|m| matches!(m, MessageItem::System { text } if text.contains("Goodbye"))));
        assert!(cmds.iter().any(|c| matches!(c, UiCmd::Quit)));
    }

    #[test]
    fn test_quit_does_not_change_mode() {
        let mut state = make_state();
        state.mode = TuiMode::CommandPalette;

        let cmds = handle_direct_command(&mut state, PaletteCommand::Quit);

        // Quit should not change mode
        assert_eq!(state.mode, TuiMode::CommandPalette);
        assert!(!state.running);
        assert!(cmds.iter().any(|c| matches!(c, UiCmd::Quit)));
    }

    // =====================================================================
    // Cancel (control case) - does nothing
    // =====================================================================

    #[test]
    fn test_cancel_preserves_messages() {
        let mut state = make_state();
        state.messages.push(MessageItem::User { text: "hello".to_string(), model: None, timestamp: None });

        let cmds = handle_direct_command(&mut state, PaletteCommand::Cancel);

        assert_eq!(state.messages.len(), 1);
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_cancel_does_not_change_mode() {
        let mut state = make_state();
        state.mode = TuiMode::CommandPalette;

        let cmds = handle_direct_command(&mut state, PaletteCommand::Cancel);

        assert_eq!(state.mode, TuiMode::CommandPalette);
        assert!(cmds.is_empty());
    }
}
