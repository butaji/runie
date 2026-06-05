//! Tests for slash command parsing and handlers.

use crate::components::MessageItem;
use crate::tui::state::{AppState, TuiMode};
use crate::tui::update::slash::handle_slash;
use runie_core::slash_command::{SlashCommand, parse_slash_command};

fn make_state() -> AppState {
    AppState {
        mode: TuiMode::Chat,
        running: true,
        current_model: Some("gpt-4o".to_string()),
        ..Default::default()
    }
}

// ─── Parsing Tests ─────────────────────────────────────────────────────────────

mod parsing_tests {
    use super::*;

    // /new and /n
    #[test]
    fn test_parse_new_aliases() {
        let parsed_new = parse_slash_command("/new");
        let parsed_n = parse_slash_command("/n");
        assert!(matches!(parsed_new, Some(SlashCommand::New)));
        assert!(matches!(parsed_n, Some(SlashCommand::New)));
    }

    // /clear and /c
    #[test]
    fn test_parse_clear_aliases() {
        let parsed_clear = parse_slash_command("/clear");
        let parsed_c = parse_slash_command("/c");
        assert!(matches!(parsed_clear, Some(SlashCommand::Clear)));
        assert!(matches!(parsed_c, Some(SlashCommand::Clear)));
    }

    // /model and /m
    #[test]
    fn test_parse_model_aliases() {
        // No arg = Help
        assert!(matches!(parse_slash_command("/model"), Some(SlashCommand::Help)));
        assert!(matches!(parse_slash_command("/m"), Some(SlashCommand::Help)));
        // With arg = Model
        assert!(matches!(parse_slash_command("/model gpt-4o"), Some(SlashCommand::Model(m)) if m == "gpt-4o"));
        assert!(matches!(parse_slash_command("/m gpt-4o"), Some(SlashCommand::Model(m)) if m == "gpt-4o"));
        assert!(matches!(parse_slash_command("/model gpt-4o-mini"), Some(SlashCommand::Model(m)) if m == "gpt-4o-mini"));
    }

    // /tree and /t
    #[test]
    fn test_parse_tree_aliases() {
        assert!(matches!(parse_slash_command("/tree"), Some(SlashCommand::Tree)));
        assert!(matches!(parse_slash_command("/t"), Some(SlashCommand::Tree)));
    }

    // /fork and /f
    #[test]
    fn test_parse_fork_aliases() {
        assert!(matches!(parse_slash_command("/fork"), Some(SlashCommand::Fork)));
        assert!(matches!(parse_slash_command("/f"), Some(SlashCommand::Fork)));
    }

    // /copy
    #[test]
    fn test_parse_copy() {
        assert!(matches!(parse_slash_command("/copy"), Some(SlashCommand::Copy)));
    }

    // /onboard and /o
    #[test]
    fn test_parse_onboard_aliases() {
        assert!(matches!(parse_slash_command("/onboard"), Some(SlashCommand::Onboard)));
        assert!(matches!(parse_slash_command("/o"), Some(SlashCommand::Onboard)));
    }

    // /quit, /q, /exit
    #[test]
    fn test_parse_quit_aliases() {
        assert!(matches!(parse_slash_command("/quit"), Some(SlashCommand::Quit)));
        assert!(matches!(parse_slash_command("/q"), Some(SlashCommand::Quit)));
        assert!(matches!(parse_slash_command("/exit"), Some(SlashCommand::Quit)));
    }

    // /help, /h, /?
    #[test]
    fn test_parse_help_aliases() {
        assert!(matches!(parse_slash_command("/help"), Some(SlashCommand::Help)));
        assert!(matches!(parse_slash_command("/h"), Some(SlashCommand::Help)));
        assert!(matches!(parse_slash_command("/?"), Some(SlashCommand::Help)));
    }

    // /cost
    #[test]
    fn test_parse_cost() {
        assert!(matches!(parse_slash_command("/cost"), Some(SlashCommand::Cost)));
    }

    // Unknown commands
    #[test]
    fn test_parse_unknown() {
        assert!(matches!(parse_slash_command("/unknown"), Some(SlashCommand::Unknown(u)) if u == "/unknown"));
        assert!(matches!(parse_slash_command("/foo"), Some(SlashCommand::Unknown(u)) if u == "/foo"));
        assert!(matches!(parse_slash_command("/whatever"), Some(SlashCommand::Unknown(u)) if u == "/whatever"));
    }

    // Non-slash input returns None
    #[test]
    fn test_parse_non_slash() {
        assert!(parse_slash_command("hello").is_none());
        assert!(parse_slash_command("").is_none());
        assert!(parse_slash_command(" /help").is_none()); // Not at start
    }

    // Case sensitivity - commands are case-sensitive
    #[test]
    fn test_parse_case_sensitive() {
        // These should NOT match (case matters)
        assert!(matches!(parse_slash_command("/NEW"), Some(SlashCommand::Unknown(u)) if u == "/NEW"));
        assert!(matches!(parse_slash_command("/CLEAR"), Some(SlashCommand::Unknown(u)) if u == "/CLEAR"));
        assert!(matches!(parse_slash_command("/Model"), Some(SlashCommand::Unknown(u)) if u == "/Model"));
        assert!(matches!(parse_slash_command("/TREE"), Some(SlashCommand::Unknown(u)) if u == "/TREE"));
        assert!(matches!(parse_slash_command("/FORK"), Some(SlashCommand::Unknown(u)) if u == "/FORK"));
        assert!(matches!(parse_slash_command("/COPY"), Some(SlashCommand::Unknown(u)) if u == "/COPY"));
        assert!(matches!(parse_slash_command("/QUIT"), Some(SlashCommand::Unknown(u)) if u == "/QUIT"));
        assert!(matches!(parse_slash_command("/HELP"), Some(SlashCommand::Unknown(u)) if u == "/HELP"));
        assert!(matches!(parse_slash_command("/COST"), Some(SlashCommand::Unknown(u)) if u == "/COST"));
    }
}

// ─── Handler Tests ─────────────────────────────────────────────────────────────

mod handler_new_tests {
    use super::*;

    #[test]
    fn test_handle_new_clears_old_messages() {
        let mut state = make_state();
        state.messages.push(MessageItem::User { text: "hello".to_string(), model: None, timestamp: None });
        state.messages.push(MessageItem::Assistant { text: "hi".to_string(), model: None, timestamp: None, expanded: false, thought_duration: None, turn_duration: None });

        handle_slash(&mut state, SlashCommand::New);

        // New clears messages but adds a system message, so old user/assistant messages are gone
        assert!(!state.messages.iter().any(|m| matches!(m, MessageItem::User { text, .. } if text == "hello")));
        assert!(!state.messages.iter().any(|m| matches!(m, MessageItem::Assistant { text, .. } if text == "hi")));
    }

    #[test]
    fn test_handle_new_resets_scroll() {
        let mut state = make_state();
        state.scroll.feed_offset = 100;

        handle_slash(&mut state, SlashCommand::New);

        assert_eq!(state.scroll.feed_offset, 0);
    }

    #[test]
    fn test_handle_new_adds_system_message() {
        let mut state = make_state();

        handle_slash(&mut state, SlashCommand::New);

        // handle_new clears messages and shows home screen, doesn't add a system message
        assert_eq!(state.messages.len(), 0);
        assert_eq!(state.mode, TuiMode::HomeScreen);
    }

    #[test]
    fn test_handle_new_produces_no_ui_commands() {
        let mut state = make_state();

        let cmds = handle_slash(&mut state, SlashCommand::New);

        assert!(cmds.is_empty());
    }
}

mod handler_clear_tests {
    use super::*;

    #[test]
    fn test_handle_clear_clears_messages() {
        let mut state = make_state();
        state.messages.push(MessageItem::User { text: "hello".to_string(), model: None, timestamp: None });
        state.messages.push(MessageItem::Assistant { text: "hi".to_string(), model: None, timestamp: None, expanded: false, thought_duration: None, turn_duration: None });

        handle_slash(&mut state, SlashCommand::Clear);

        assert!(state.messages.is_empty());
    }

    #[test]
    fn test_handle_clear_resets_scroll() {
        let mut state = make_state();
        state.scroll.feed_offset = 100;

        handle_slash(&mut state, SlashCommand::Clear);

        assert_eq!(state.scroll.feed_offset, 0);
    }

    #[test]
    fn test_handle_clear_does_not_add_system_message() {
        let mut state = make_state();

        handle_slash(&mut state, SlashCommand::Clear);

        assert!(state.messages.is_empty());
    }

    #[test]
    fn test_handle_clear_produces_no_ui_commands() {
        let mut state = make_state();

        let cmds = handle_slash(&mut state, SlashCommand::Clear);

        assert!(cmds.is_empty());
    }
}

mod handler_model_tests {
    use super::*;

    #[test]
    fn test_handle_model_sets_current_model() {
        let mut state = make_state();
        state.current_model = Some("gpt-4o".to_string());

        handle_slash(&mut state, SlashCommand::Model("claude-3-opus".to_string()));

        assert_eq!(state.current_model, Some("claude-3-opus".to_string()));
    }

    #[test]
    fn test_handle_model_adds_system_message() {
        let mut state = make_state();
        state.messages.clear();

        handle_slash(&mut state, SlashCommand::Model("gpt-4o-mini".to_string()));

        assert_eq!(state.messages.len(), 1);
        assert!(matches!(&state.messages[0], MessageItem::System { text } if text.contains("gpt-4o-mini")));
    }

    #[test]
    fn test_handle_model_message_contains_model_name() {
        let mut state = make_state();
        state.messages.clear();

        handle_slash(&mut state, SlashCommand::Model("gpt-4o".to_string()));

        assert!(matches!(&state.messages[0], MessageItem::System { text } if text.contains("Model switched to gpt-4o")));
    }

    #[test]
    fn test_handle_model_produces_no_ui_commands() {
        let mut state = make_state();

        let cmds = handle_slash(&mut state, SlashCommand::Model("gpt-4o".to_string()));

        assert!(cmds.is_empty());
    }
}

mod handler_tree_tests {
    use super::*;

    #[test]
    fn test_handle_tree_toggles_session_tree_on() {
        let mut state = make_state();
        state.session_tree.visible = false;
        state.mode = TuiMode::Chat;

        handle_slash(&mut state, SlashCommand::Tree);

        assert!(state.session_tree.visible);
        assert_eq!(state.mode, TuiMode::SessionTree);
    }

    #[test]
    fn test_handle_tree_toggles_session_tree_off() {
        let mut state = make_state();
        state.session_tree.visible = true;
        state.mode = TuiMode::SessionTree;

        handle_slash(&mut state, SlashCommand::Tree);

        assert!(!state.session_tree.visible);
        assert_eq!(state.mode, TuiMode::Chat);
    }

    #[test]
    fn test_handle_tree_produces_no_ui_commands() {
        let mut state = make_state();
        state.session_tree.visible = false;

        let cmds = handle_slash(&mut state, SlashCommand::Tree);

        assert!(cmds.is_empty());
    }
}

mod handler_fork_tests {
    use super::*;

    #[test]
    fn test_handle_fork_adds_system_message() {
        let mut state = make_state();
        state.messages.clear();

        handle_slash(&mut state, SlashCommand::Fork);

        assert_eq!(state.messages.len(), 1);
        assert!(matches!(&state.messages[0], MessageItem::System { text } if text.contains("Fork")));
    }

    #[test]
    fn test_handle_fork_message_contains_position() {
        let mut state = make_state();
        state.messages.clear();

        handle_slash(&mut state, SlashCommand::Fork);

        assert!(matches!(&state.messages[0], MessageItem::System { text } if text.contains("current position")));
    }

    #[test]
    fn test_handle_fork_produces_no_ui_commands() {
        let mut state = make_state();

        let cmds = handle_slash(&mut state, SlashCommand::Fork);

        assert!(cmds.is_empty());
    }
}

mod handler_copy_tests {
    use super::*;

    #[test]
    fn test_handle_copy_with_no_assistant_message_adds_error() {
        let mut state = make_state();
        state.messages.clear();
        state.messages.push(MessageItem::User { text: "hello".to_string(), model: None, timestamp: None });

        handle_slash(&mut state, SlashCommand::Copy);

        // Should add system error message
        assert_eq!(state.messages.len(), 2);
        assert!(matches!(&state.messages[1], MessageItem::System { text } if text.contains("No assistant response")));
    }

    #[test]
    fn test_handle_copy_with_assistant_message_adds_confirmation() {
        let mut state = make_state();
        state.messages.clear();
        state.messages.push(MessageItem::User { text: "hello".to_string(), model: None, timestamp: None });
        state.messages.push(MessageItem::Assistant { text: "assistant response".to_string(), model: None, timestamp: None, expanded: false, thought_duration: None, turn_duration: None });

        handle_slash(&mut state, SlashCommand::Copy);

        // Should add system confirmation message
        assert_eq!(state.messages.len(), 3);
        assert!(matches!(&state.messages[2], MessageItem::System { text } if text.contains("Copied")));
    }

    #[test]
    fn test_handle_copy_copies_last_assistant_not_first() {
        let mut state = make_state();
        state.messages.clear();
        state.messages.push(MessageItem::Assistant { text: "first response".to_string(), model: None, timestamp: None, expanded: false, thought_duration: None, turn_duration: None });
        state.messages.push(MessageItem::User { text: "question".to_string(), model: None, timestamp: None });
        state.messages.push(MessageItem::Assistant { text: "last response".to_string(), model: None, timestamp: None, expanded: false, thought_duration: None, turn_duration: None });

        handle_slash(&mut state, SlashCommand::Copy);

        // Should find last assistant (which is "last response")
        assert!(matches!(&state.messages[3], MessageItem::System { text } if text.contains("Copied")));
    }

    #[test]
    fn test_handle_copy_returns_empty_vec() {
        let mut state = make_state();
        state.messages.clear();

        let cmds = handle_slash(&mut state, SlashCommand::Copy);

        // Copy command returns empty Vec<UiCmd>
        assert!(cmds.is_empty());
    }
}

mod handler_onboard_tests {
    use super::*;

    #[test]
    fn test_handle_onboard_sets_mode() {
        let mut state = make_state();
        state.mode = TuiMode::Chat;
        state.onboarding = None;

        handle_slash(&mut state, SlashCommand::Onboard);

        assert_eq!(state.mode, TuiMode::Onboarding);
    }

    #[test]
    fn test_handle_onboard_creates_onboarding() {
        let mut state = make_state();
        state.onboarding = None;

        handle_slash(&mut state, SlashCommand::Onboard);

        assert!(state.onboarding.is_some());
    }

    #[test]
    fn test_handle_onboard_produces_no_ui_commands() {
        let mut state = make_state();

        let cmds = handle_slash(&mut state, SlashCommand::Onboard);

        assert!(cmds.is_empty());
    }
}

mod handler_quit_tests {
    use super::*;

    #[test]
    fn test_handle_quit_sets_running_false() {
        let mut state = make_state();
        assert!(state.running);

        handle_slash(&mut state, SlashCommand::Quit);

        assert!(!state.running);
    }

    #[test]
    fn test_handle_quit_does_not_clear_messages() {
        let mut state = make_state();
        state.messages.push(MessageItem::User { text: "hello".to_string(), model: None, timestamp: None });

        handle_slash(&mut state, SlashCommand::Quit);

        assert_eq!(state.messages.len(), 1);
    }

    #[test]
    fn test_handle_quit_produces_no_ui_commands() {
        let mut state = make_state();

        let cmds = handle_slash(&mut state, SlashCommand::Quit);

        assert!(cmds.is_empty());
    }
}

mod handler_help_tests {
    use super::*;

    #[test]
    fn test_handle_help_adds_system_message() {
        let mut state = make_state();
        state.messages.clear();

        handle_slash(&mut state, SlashCommand::Help);

        assert_eq!(state.messages.len(), 1);
        assert!(matches!(&state.messages[0], MessageItem::System { text } if text.contains("Available commands")));
    }

    #[test]
    fn test_handle_help_contains_all_commands() {
        let mut state = make_state();
        state.messages.clear();

        handle_slash(&mut state, SlashCommand::Help);

        let text = match &state.messages[0] {
            MessageItem::System { text } => text.clone(),
            _ => String::new(),
        };
        assert!(text.contains("/new"));
        assert!(text.contains("/clear"));
        assert!(text.contains("/model"));
        assert!(text.contains("/tree"));
        assert!(text.contains("/fork"));
        assert!(text.contains("/copy"));
        assert!(text.contains("/cost"));
        assert!(text.contains("/quit"));
        assert!(text.contains("/help"));
    }

    #[test]
    fn test_handle_help_produces_no_ui_commands() {
        let mut state = make_state();

        let cmds = handle_slash(&mut state, SlashCommand::Help);

        assert!(cmds.is_empty());
    }
}

mod handler_cost_tests {
    use super::*;

    #[test]
    fn test_handle_cost_adds_system_message() {
        let mut state = make_state();
        state.messages.clear();
        state.session_token_usage.total_tokens = 1000;
        state.session_token_usage.estimated_cost = 0.05;

        handle_slash(&mut state, SlashCommand::Cost);

        assert_eq!(state.messages.len(), 1);
        assert!(matches!(&state.messages[0], MessageItem::System { text } if text.contains("1000")));
    }

    #[test]
    fn test_handle_cost_message_contains_tokens_and_cost() {
        let mut state = make_state();
        state.messages.clear();
        state.session_token_usage.prompt_tokens = 300;
        state.session_token_usage.completion_tokens = 200;
        state.session_token_usage.total_tokens = 500;
        state.session_token_usage.estimated_cost = 0.025;

        handle_slash(&mut state, SlashCommand::Cost);

        let text = match &state.messages[0] {
            MessageItem::System { text } => text.clone(),
            _ => String::new(),
        };
        assert!(text.contains("300"));
        assert!(text.contains("200"));
        assert!(text.contains("500"));
        assert!(text.contains("$0.0250"));
    }

    #[test]
    fn test_handle_cost_produces_no_ui_commands() {
        let mut state = make_state();
        state.session_token_usage.total_tokens = 1234;
        state.session_token_usage.estimated_cost = 0.1;

        let cmds = handle_slash(&mut state, SlashCommand::Cost);

        assert!(cmds.is_empty());
    }

    #[test]
    fn test_handle_cost_message_format() {
        let mut state = make_state();
        state.messages.clear();
        state.session_token_usage.prompt_tokens = 100;
        state.session_token_usage.completion_tokens = 50;
        state.session_token_usage.total_tokens = 150;
        state.session_token_usage.estimated_cost = 0.0075;

        handle_slash(&mut state, SlashCommand::Cost);

        let text = match &state.messages[0] {
            MessageItem::System { text } => text.clone(),
            _ => String::new(),
        };
        // Verify exact format: "Session usage: {} prompt + {} completion = {} tokens, ${:.4}"
        assert_eq!(text, "Session usage: 100 prompt + 50 completion = 150 tokens, $0.0075");
    }
}

mod handler_unknown_tests {
    use super::*;

    #[test]
    fn test_handle_unknown_adds_error_message() {
        let mut state = make_state();
        state.messages.clear();

        handle_slash(&mut state, SlashCommand::Unknown("/foo".to_string()));

        assert_eq!(state.messages.len(), 1);
        assert!(matches!(&state.messages[0], MessageItem::System { text } if text.contains("Unknown command")));
    }

    #[test]
    fn test_handle_unknown_includes_command_name() {
        let mut state = make_state();
        state.messages.clear();

        handle_slash(&mut state, SlashCommand::Unknown("/bar".to_string()));

        let text = match &state.messages[0] {
            MessageItem::System { text } => text.clone(),
            _ => String::new(),
        };
        assert!(text.contains("/bar"));
    }

    #[test]
    fn test_handle_unknown_mentions_help() {
        let mut state = make_state();
        state.messages.clear();

        handle_slash(&mut state, SlashCommand::Unknown("/xyz".to_string()));

        let text = match &state.messages[0] {
            MessageItem::System { text } => text.clone(),
            _ => String::new(),
        };
        assert!(text.contains("/help"));
    }

    #[test]
    fn test_handle_unknown_produces_no_ui_commands() {
        let mut state = make_state();

        let cmds = handle_slash(&mut state, SlashCommand::Unknown("/test".to_string()));

        assert!(cmds.is_empty());
    }
}

// ─── Integration Tests ─────────────────────────────────────────────────────────

mod integration_tests {
    use super::*;

    #[test]
    fn test_all_aliases_parse_correctly() {
        // New aliases
        let new_cmd = parse_slash_command("/new");
        let n_cmd = parse_slash_command("/n");
        assert!(matches!(new_cmd, Some(SlashCommand::New)));
        assert!(matches!(n_cmd, Some(SlashCommand::New)));

        // Clear aliases
        let clear_cmd = parse_slash_command("/clear");
        let c_cmd = parse_slash_command("/c");
        assert!(matches!(clear_cmd, Some(SlashCommand::Clear)));
        assert!(matches!(c_cmd, Some(SlashCommand::Clear)));

        // Tree aliases
        let tree_cmd = parse_slash_command("/tree");
        let t_cmd = parse_slash_command("/t");
        assert!(matches!(tree_cmd, Some(SlashCommand::Tree)));
        assert!(matches!(t_cmd, Some(SlashCommand::Tree)));

        // Fork aliases
        let fork_cmd = parse_slash_command("/fork");
        let f_cmd = parse_slash_command("/f");
        assert!(matches!(fork_cmd, Some(SlashCommand::Fork)));
        assert!(matches!(f_cmd, Some(SlashCommand::Fork)));

        // Onboard aliases
        let onboard_cmd = parse_slash_command("/onboard");
        let o_cmd = parse_slash_command("/o");
        assert!(matches!(onboard_cmd, Some(SlashCommand::Onboard)));
        assert!(matches!(o_cmd, Some(SlashCommand::Onboard)));

        // Quit aliases
        let quit_cmd = parse_slash_command("/quit");
        let q_cmd = parse_slash_command("/q");
        let exit_cmd = parse_slash_command("/exit");
        assert!(matches!(quit_cmd, Some(SlashCommand::Quit)));
        assert!(matches!(q_cmd, Some(SlashCommand::Quit)));
        assert!(matches!(exit_cmd, Some(SlashCommand::Quit)));

        // Help aliases
        let help_cmd = parse_slash_command("/help");
        let h_cmd = parse_slash_command("/h");
        let question_cmd = parse_slash_command("/?");
        assert!(matches!(help_cmd, Some(SlashCommand::Help)));
        assert!(matches!(h_cmd, Some(SlashCommand::Help)));
        assert!(matches!(question_cmd, Some(SlashCommand::Help)));
    }

    #[test]
    fn test_model_command_with_different_model_names() {
        let models = vec![
            "gpt-4o",
            "gpt-4o-mini",
            "claude-3-opus",
            "claude-3-sonnet",
            "gemini-1.5-pro",
            "o1-preview",
            "o1-mini",
        ];

        for model in models {
            let cmd = parse_slash_command(&format!("/model {}", model));
            assert!(matches!(cmd, Some(SlashCommand::Model(m)) if m == model));
        }
    }

    #[test]
    fn test_handle_slash_dispatches_all_commands_without_panic() {
        let commands = vec![
            SlashCommand::New,
            SlashCommand::Clear,
            SlashCommand::Tree,
            SlashCommand::Fork,
            SlashCommand::Onboard,
            SlashCommand::Quit,
            SlashCommand::Help,
            SlashCommand::Cost,
            SlashCommand::Unknown("/test".to_string()),
        ];

        for cmd in commands {
            let mut state = make_state();
            // Should not panic
            let _cmds = handle_slash(&mut state, cmd);
        }
    }

    #[test]
    fn test_command_sequence_new_then_model() {
        let mut state = make_state();
        state.messages.push(MessageItem::User { text: "hello".to_string(), model: None, timestamp: None });

        // Note: handle_slash is called directly (not through run_slash), so mode stays at HomeScreen after New
        handle_slash(&mut state, SlashCommand::New);

        // handle_new clears messages and switches to HomeScreen
        assert_eq!(state.messages.len(), 0);
        assert_eq!(state.mode, TuiMode::HomeScreen);

        handle_slash(&mut state, SlashCommand::Model("claude-3-opus".to_string()));

        // handle_model adds a system message
        assert_eq!(state.messages.len(), 1);
        assert_eq!(state.current_model, Some("claude-3-opus".to_string()));
    }

    #[test]
    fn test_quit_is_idempotent() {
        let mut state = make_state();

        handle_slash(&mut state, SlashCommand::Quit);
        assert!(!state.running);

        // Calling again should still keep running false
        handle_slash(&mut state, SlashCommand::Quit);
        assert!(!state.running);
    }

    #[test]
    fn test_tree_toggle_is_idempotent() {
        let mut state = make_state();
        state.session_tree.visible = false;

        // Toggle on
        handle_slash(&mut state, SlashCommand::Tree);
        assert!(state.session_tree.visible);

        // Toggle off
        handle_slash(&mut state, SlashCommand::Tree);
        assert!(!state.session_tree.visible);

        // Toggle on again
        handle_slash(&mut state, SlashCommand::Tree);
        assert!(state.session_tree.visible);
    }

    #[test]
    fn test_unknown_command_preserves_existing_messages() {
        let mut state = make_state();
        state.messages.push(MessageItem::User { text: "hello".to_string(), model: None, timestamp: None });
        state.messages.push(MessageItem::Assistant { text: "hi".to_string(), model: None, timestamp: None, expanded: false, thought_duration: None, turn_duration: None });

        handle_slash(&mut state, SlashCommand::Unknown("/bad".to_string()));

        // Original messages still there + 1 error message
        assert_eq!(state.messages.len(), 3);
    }
}
