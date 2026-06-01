//! Session management tests.
//!
//! Tests slash commands and session tree functionality.

use crate::components::{MessageItem, SessionTreeNavigator};
use crate::tui::state::{AppState, TuiMode};
use crate::tui::update::slash::handle_slash;
use runie_core::slash_command::{SlashCommand, parse_slash_command};
use runie_core::session::Session;
use runie_core::Message;

// ─── Helper Functions ─────────────────────────────────────────────────────────

fn make_state() -> AppState {
    AppState {
        mode: TuiMode::Chat,
        running: true,
        current_model: Some("gpt-4o".to_string()),
        scroll: crate::tui::state::ScrollState::default(),
        ..Default::default()
    }
}

fn with_user_and_assistant(state: &mut AppState) {
    state.messages.push(MessageItem::User {
        text: "Hello".to_string(),
        model: None,
        timestamp: None,
    });
    state.messages.push(MessageItem::Assistant {
        text: "Hi there".to_string(),
        model: None,
        timestamp: None,
    });
}

// ─── /new Command ─────────────────────────────────────────────────────────────

mod new_command {
    use super::*;

    #[test]
    fn test_new_clears_messages() {
        let mut state = make_state();
        with_user_and_assistant(&mut state);

        handle_slash(&mut state, SlashCommand::New);

        assert!(state.messages.is_empty() || state.messages.len() == 1);
        // /new adds a system message
    }

    #[test]
    fn test_new_adds_system_message() {
        let mut state = make_state();

        handle_slash(&mut state, SlashCommand::New);

        assert_eq!(state.messages.len(), 1);
        assert!(matches!(&state.messages[0], MessageItem::System { text } if text.contains("New session")));
    }

    #[test]
    fn test_new_resets_scroll() {
        let mut state = make_state();
        state.scroll.feed_offset = 100;
        state.scroll.user_scrolled_up = true;

        handle_slash(&mut state, SlashCommand::New);

        assert_eq!(state.scroll.feed_offset, 0);
        assert!(!state.scroll.user_scrolled_up);
    }

    #[test]
    fn test_new_alias_n() {
        let parsed = parse_slash_command("/n");
        assert!(matches!(parsed, Some(SlashCommand::New)));
    }
}

// ─── /clear Command ───────────────────────────────────────────────────────────

mod clear_command {
    use super::*;

    #[test]
    fn test_clear_removes_all_messages() {
        let mut state = make_state();
        with_user_and_assistant(&mut state);

        handle_slash(&mut state, SlashCommand::Clear);

        assert!(state.messages.is_empty());
    }

    #[test]
    fn test_clear_does_not_add_system_message() {
        let mut state = make_state();

        handle_slash(&mut state, SlashCommand::Clear);

        assert!(state.messages.is_empty());
    }

    #[test]
    fn test_clear_resets_scroll() {
        let mut state = make_state();
        state.scroll.feed_offset = 50;

        handle_slash(&mut state, SlashCommand::Clear);

        assert_eq!(state.scroll.feed_offset, 0);
    }

    #[test]
    fn test_clear_alias_c() {
        let parsed = parse_slash_command("/c");
        assert!(matches!(parsed, Some(SlashCommand::Clear)));
    }
}

// ─── /fork Command ────────────────────────────────────────────────────────────

mod fork_command {
    use super::*;

    #[test]
    fn test_fork_adds_system_message() {
        let mut state = make_state();
        state.messages.clear();

        handle_slash(&mut state, SlashCommand::Fork);

        assert_eq!(state.messages.len(), 1);
        assert!(matches!(&state.messages[0], MessageItem::System { text } if text.contains("Fork")));
    }

    #[test]
    fn test_fork_preserves_existing_messages() {
        let mut state = make_state();
        with_user_and_assistant(&mut state);

        handle_slash(&mut state, SlashCommand::Fork);

        // Original messages preserved + fork message
        assert!(state.messages.len() >= 2);
    }

    #[test]
    fn test_fork_alias_f() {
        let parsed = parse_slash_command("/f");
        assert!(matches!(parsed, Some(SlashCommand::Fork)));
    }
}

// ─── /tree Command ───────────────────────────────────────────────────────────

mod tree_command {
    use super::*;

    #[test]
    fn test_tree_toggles_visibility_on() {
        let mut state = make_state();
        state.session_tree.visible = false;
        state.mode = TuiMode::Chat;

        handle_slash(&mut state, SlashCommand::Tree);

        assert!(state.session_tree.visible);
        assert_eq!(state.mode, TuiMode::SessionTree);
    }

    #[test]
    fn test_tree_toggles_visibility_off() {
        let mut state = make_state();
        state.session_tree.visible = true;
        state.mode = TuiMode::SessionTree;

        handle_slash(&mut state, SlashCommand::Tree);

        assert!(!state.session_tree.visible);
        assert_eq!(state.mode, TuiMode::Chat);
    }

    #[test]
    fn test_tree_alias_t() {
        let parsed = parse_slash_command("/t");
        assert!(matches!(parsed, Some(SlashCommand::Tree)));
    }
}

// ─── Session Tree Navigator Tests ─────────────────────────────────────────────

mod session_tree_navigator {
    use super::*;

    fn make_tree_navigator() -> SessionTreeNavigator {
        SessionTreeNavigator::new()
    }

    fn make_test_session() -> Session {
        let mut session = Session::new("test".to_string());
        let id1 = session.add_message(None, Message::User { content: "Hello".to_string(), attachments: vec![] });
        let _id2 = session.add_message(Some(id1.clone()), Message::Assistant { content: "Hi there".to_string(), tool_calls: vec![], thinking: None });
        session
    }

    #[test]
    fn test_tree_move_up() {
        let mut nav = make_tree_navigator();
        let session = make_test_session();
        nav.load_session(&session);

        nav.move_down(); // Move to index 1
        assert_eq!(nav.selected, 1);

        nav.move_up(); // Move back to index 0
        assert_eq!(nav.selected, 0);
    }

    #[test]
    fn test_tree_move_down() {
        let mut nav = make_tree_navigator();
        let session = make_test_session();
        nav.load_session(&session);

        nav.move_down();
        assert_eq!(nav.selected, 1);

        // Can't go past end
        nav.move_down();
        assert_eq!(nav.selected, 1);
    }

    #[test]
    fn test_tree_up_boundary() {
        let mut nav = make_tree_navigator();
        let session = make_test_session();
        nav.load_session(&session);

        assert_eq!(nav.selected, 0);
        nav.move_up();
        assert_eq!(nav.selected, 0, "Should not go below 0");
    }

    #[test]
    fn test_tree_down_boundary() {
        let mut nav = make_tree_navigator();
        let session = make_test_session();
        nav.load_session(&session);

        nav.move_down();
        nav.move_down(); // Try to go past end
        assert_eq!(nav.selected, 1);
    }

    #[test]
    fn test_tree_toggle() {
        let mut nav = make_tree_navigator();
        assert!(!nav.visible);

        nav.toggle();
        assert!(nav.visible);

        nav.toggle();
        assert!(!nav.visible);
    }

    #[test]
    fn test_tree_with_zero_messages() {
        let mut nav = make_tree_navigator();
        let session = Session::new("empty".to_string());
        nav.load_session(&session);

        assert_eq!(nav.entries.len(), 0);

        nav.move_down();
        assert_eq!(nav.selected, 0);

        nav.move_up();
        assert_eq!(nav.selected, 0);
    }

    #[test]
    fn test_tree_with_100_messages() {
        let mut nav = make_tree_navigator();
        let mut session = Session::new("large".to_string());

        // Add 100 messages
        for i in 0..100 {
            let parent = if i > 0 { Some(session.messages[i - 1].id.clone()) } else { None };
            session.add_message(parent, Message::User { content: format!("Message {}", i), attachments: vec![] });
        }

        nav.load_session(&session);
        assert_eq!(nav.entries.len(), 100);

        // Navigate to end
        for _ in 0..150 {
            nav.move_down();
        }
        assert_eq!(nav.selected, 99);

        // Navigate to start
        for _ in 0..150 {
            nav.move_up();
        }
        assert_eq!(nav.selected, 0);
    }

    #[test]
    fn test_tree_get_selected_id() {
        let mut nav = make_tree_navigator();
        let session = make_test_session();
        nav.load_session(&session);

        let first_id = nav.get_selected_id();
        assert!(first_id.is_some());

        nav.move_down();
        let second_id = nav.get_selected_id();
        assert!(second_id.is_some());
        assert_ne!(first_id, second_id);
    }

    #[test]
    fn test_tree_with_empty_session() {
        let mut nav = make_tree_navigator();
        let session = Session::new("empty".to_string());
        nav.load_session(&session);

        assert_eq!(nav.entries.len(), 0);
        assert!(nav.get_selected_id().is_none());
    }
}

// ─── /copy Command ───────────────────────────────────────────────────────────

mod copy_command {
    use super::*;

    #[test]
    fn test_copy_with_no_assistant_shows_error() {
        let mut state = make_state();
        state.messages.clear();
        state.messages.push(MessageItem::User {
            text: "Hello".to_string(),
            model: None,
            timestamp: None,
        });

        handle_slash(&mut state, SlashCommand::Copy);

        // Should add error message
        let has_error = state.messages.iter().any(|m| matches!(
            m, MessageItem::System { text } if text.contains("No assistant")
        ));
        assert!(has_error);
    }

    #[test]
    fn test_copy_with_assistant_copies() {
        let mut state = make_state();
        state.messages.clear();
        state.messages.push(MessageItem::User {
            text: "Hello".to_string(),
            model: None,
            timestamp: None,
        });
        state.messages.push(MessageItem::Assistant {
            text: "Hi there".to_string(),
            model: None,
            timestamp: None,
        });

        handle_slash(&mut state, SlashCommand::Copy);

        // Should add confirmation message
        let has_copied = state.messages.iter().any(|m| matches!(
            m, MessageItem::System { text } if text.contains("Copied")
        ));
        assert!(has_copied);
    }

    #[test]
    fn test_copy_finds_last_assistant() {
        let mut state = make_state();
        state.messages.clear();
        state.messages.push(MessageItem::Assistant {
            text: "First response".to_string(),
            model: None,
            timestamp: None,
        });
        state.messages.push(MessageItem::User {
            text: "Question".to_string(),
            model: None,
            timestamp: None,
        });
        state.messages.push(MessageItem::Assistant {
            text: "Last response".to_string(),
            model: None,
            timestamp: None,
        });

        handle_slash(&mut state, SlashCommand::Copy);

        // Should copy the last assistant
        let has_copied = state.messages.iter().any(|m| matches!(
            m, MessageItem::System { text } if text.contains("Copied")
        ));
        assert!(has_copied);
    }
}

// ─── /cost Command ────────────────────────────────────────────────────────────

mod cost_command {
    use super::*;

    #[test]
    fn test_cost_shows_token_usage() {
        let mut state = make_state();
        state.messages.clear();
        state.session_token_usage.prompt_tokens = 100;
        state.session_token_usage.completion_tokens = 50;
        state.session_token_usage.total_tokens = 150;
        state.session_token_usage.estimated_cost = 0.0075;

        handle_slash(&mut state, SlashCommand::Cost);

        assert_eq!(state.messages.len(), 1);
        assert!(matches!(&state.messages[0], MessageItem::System { text } if text.contains("150")));
    }

    #[test]
    fn test_cost_message_format() {
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
}

// ─── /help Command ────────────────────────────────────────────────────────────

mod help_command {
    use super::*;

    #[test]
    fn test_help_shows_all_commands() {
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
    fn test_help_alias_h() {
        let parsed = parse_slash_command("/h");
        assert!(matches!(parsed, Some(SlashCommand::Help)));
    }

    #[test]
    fn test_help_alias_question() {
        let parsed = parse_slash_command("/?");
        assert!(matches!(parsed, Some(SlashCommand::Help)));
    }
}

// ─── Unknown Command ──────────────────────────────────────────────────────────

mod unknown_command {
    use super::*;

    #[test]
    fn test_unknown_shows_error() {
        let mut state = make_state();
        state.messages.clear();

        handle_slash(&mut state, SlashCommand::Unknown("/badcmd".to_string()));

        assert_eq!(state.messages.len(), 1);
        assert!(matches!(&state.messages[0], MessageItem::System { text } if text.contains("Unknown command")));
    }

    #[test]
    fn test_unknown_suggests_help() {
        let mut state = make_state();
        state.messages.clear();

        handle_slash(&mut state, SlashCommand::Unknown("/foo".to_string()));

        let text = match &state.messages[0] {
            MessageItem::System { text } => text.clone(),
            _ => String::new(),
        };
        assert!(text.contains("/help"));
    }

    #[test]
    fn test_unknown_includes_command_name() {
        let mut state = make_state();
        state.messages.clear();

        handle_slash(&mut state, SlashCommand::Unknown("/xyz".to_string()));

        let text = match &state.messages[0] {
            MessageItem::System { text } => text.clone(),
            _ => String::new(),
        };
        assert!(text.contains("/xyz"));
    }
}

// ─── All Slash Command Aliases ───────────────────────────────────────────────

mod all_aliases {
    use super::*;

    #[test]
    fn test_new_aliases() {
        assert!(matches!(parse_slash_command("/new"), Some(SlashCommand::New)));
        assert!(matches!(parse_slash_command("/n"), Some(SlashCommand::New)));
    }

    #[test]
    fn test_clear_aliases() {
        assert!(matches!(parse_slash_command("/clear"), Some(SlashCommand::Clear)));
        assert!(matches!(parse_slash_command("/c"), Some(SlashCommand::Clear)));
    }

    #[test]
    fn test_tree_aliases() {
        assert!(matches!(parse_slash_command("/tree"), Some(SlashCommand::Tree)));
        assert!(matches!(parse_slash_command("/t"), Some(SlashCommand::Tree)));
    }

    #[test]
    fn test_fork_aliases() {
        assert!(matches!(parse_slash_command("/fork"), Some(SlashCommand::Fork)));
        assert!(matches!(parse_slash_command("/f"), Some(SlashCommand::Fork)));
    }

    #[test]
    fn test_onboard_aliases() {
        assert!(matches!(parse_slash_command("/onboard"), Some(SlashCommand::Onboard)));
        assert!(matches!(parse_slash_command("/o"), Some(SlashCommand::Onboard)));
    }

    #[test]
    fn test_quit_aliases() {
        assert!(matches!(parse_slash_command("/quit"), Some(SlashCommand::Quit)));
        assert!(matches!(parse_slash_command("/q"), Some(SlashCommand::Quit)));
        assert!(matches!(parse_slash_command("/exit"), Some(SlashCommand::Quit)));
    }

    #[test]
    fn test_model_aliases() {
        assert!(matches!(parse_slash_command("/model gpt-4o"), Some(SlashCommand::Model(m)) if m == "gpt-4o"));
        assert!(matches!(parse_slash_command("/m gpt-4o"), Some(SlashCommand::Model(m)) if m == "gpt-4o"));
    }

    #[test]
    fn test_copy_no_alias() {
        assert!(matches!(parse_slash_command("/copy"), Some(SlashCommand::Copy)));
        // No short alias for copy.  The parser does surface Unknown for any
        // /-prefixed input that doesn't match a known command, so "/co"
        // falls through to Unknown rather than being None — the invariant
        // we care about is that the *Copy* variant is not produced.
        assert!(!matches!(parse_slash_command("/co"), Some(SlashCommand::Copy)));
    }

    #[test]
    fn test_cost_no_alias() {
        assert!(matches!(parse_slash_command("/cost"), Some(SlashCommand::Cost)));
        // No short alias for cost — same rationale as test_copy_no_alias.
        assert!(!matches!(parse_slash_command("/cos"), Some(SlashCommand::Cost)));
    }
}

// ─── Command Sequence Tests ───────────────────────────────────────────────────

mod command_sequences {
    use super::*;

    #[test]
    fn test_new_then_model() {
        let mut state = make_state();
        state.messages.push(MessageItem::User {
            text: "Hello".to_string(),
            model: None,
            timestamp: None,
        });

        handle_slash(&mut state, SlashCommand::New);
        assert_eq!(state.messages.len(), 1);

        handle_slash(&mut state, SlashCommand::Model("claude-3-opus".to_string()));
        assert_eq!(state.messages.len(), 2);
        assert_eq!(state.current_model, Some("claude-3-opus".to_string()));
    }

    #[test]
    fn test_fork_then_clear() {
        let mut state = make_state();
        with_user_and_assistant(&mut state);

        handle_slash(&mut state, SlashCommand::Fork);
        assert!(state.messages.len() >= 2);

        handle_slash(&mut state, SlashCommand::Clear);
        assert!(state.messages.is_empty());
    }

    #[test]
    fn test_tree_toggle_multiple_times() {
        let mut state = make_state();
        assert!(!state.session_tree.visible);

        handle_slash(&mut state, SlashCommand::Tree);
        assert!(state.session_tree.visible);

        handle_slash(&mut state, SlashCommand::Tree);
        assert!(!state.session_tree.visible);

        handle_slash(&mut state, SlashCommand::Tree);
        assert!(state.session_tree.visible);
    }

    #[test]
    fn test_help_then_cost() {
        let mut state = make_state();
        state.messages.clear();
        state.session_token_usage.total_tokens = 100;

        handle_slash(&mut state, SlashCommand::Help);
        handle_slash(&mut state, SlashCommand::Cost);

        assert_eq!(state.messages.len(), 2);
    }
}

// ─── Session Tree Navigation ─────────────────────────────────────────────────

mod tree_navigation {
    use super::*;

    fn create_large_session(count: usize) -> SessionTreeNavigator {
        let mut nav = SessionTreeNavigator::new();
        let mut session = Session::new("large".to_string());

        for i in 0..count {
            let parent = if i > 0 { Some(session.messages[i - 1].id.clone()) } else { None };
            session.add_message(parent, Message::User { content: format!("Message {}", i), attachments: vec![] });
        }

        nav.load_session(&session);
        nav
    }

    #[test]
    fn test_tree_navigation_100_entries() {
        let mut nav = create_large_session(100);

        // Navigate to middle
        for _ in 0..50 {
            nav.move_down();
        }
        assert_eq!(nav.selected, 50);

        // Navigate to end
        for _ in 0..60 {
            nav.move_down();
        }
        assert_eq!(nav.selected, 99);

        // Navigate back to start
        for _ in 0..110 {
            nav.move_up();
        }
        assert_eq!(nav.selected, 0);
    }

    #[test]
    fn test_tree_scroll_offset_tracks_selection() {
        let mut nav = create_large_session(100);

        // Move far down
        for _ in 0..50 {
            nav.move_down();
        }

        assert!(nav.scroll_offset <= nav.selected);
    }

    #[test]
    fn test_tree_toggle_resets_selection() {
        let mut nav = create_large_session(20);

        for _ in 0..10 {
            nav.move_down();
        }
        assert_eq!(nav.selected, 10);

        nav.toggle();
        assert!(nav.visible);

        nav.toggle();
        assert!(!nav.visible);
        // Selection should reset when toggled on again
        nav.toggle();
        assert_eq!(nav.selected, 0);
    }
}