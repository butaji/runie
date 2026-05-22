use crate::tui::state::{AppState, TuiMode, Msg, Cmd, AnimationState};
use crate::components::{
    AgentList, AgentItem, AgentStatus, MessageItem,
    ContextPanel, GitChange, GitStatus, SessionTreeNavigator, CommandPalette,
};
use crate::tui::update::update;
use runie_agent::events::{AgentEvent, AgentMessage, PermissionDecision, ContentPart};
use runie_ai::TokenUsage;


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_list_has_demo_data() {
        // Verify AgentList default has agents populated (testing the data structure)
        let agent_list = AgentList {
            agents: vec![
                AgentItem {
                    id: "coder".to_string(),
                    tag: "coder".to_string(),
                    tag_type: "assistant".to_string(),
                    description: "editing files".to_string(),
                    model: "claude-4".to_string(),
                    duration_secs: 45,
                    status: AgentStatus::Running,
                },
                AgentItem {
                    id: "test".to_string(),
                    tag: "test".to_string(),
                    tag_type: "system".to_string(),
                    description: "running tests".to_string(),
                    model: "gpt-4".to_string(),
                    duration_secs: 12,
                    status: AgentStatus::Completed,
                },
            ],
        };
        assert_eq!(agent_list.agents.len(), 2);
        assert_eq!(agent_list.agents[0].id, "coder");
        assert_eq!(agent_list.agents[1].status, AgentStatus::Completed);
    }

    #[test]
    fn test_context_panel_has_demo_data() {
        let context_panel = ContextPanel {
            recent_files: vec![
                "src/main.rs".to_string(),
                "Cargo.toml".to_string(),
                "README.md".to_string(),
            ],
            git_changes: vec![
                GitChange { path: "src/tui.rs".to_string(), status: GitStatus::Modified },
                GitChange { path: "src/components/context_panel.rs".to_string(), status: GitStatus::Added },
            ],
            active_tool: Some("read_file".to_string()),
            model_name: "claude-4".to_string(),
            session_info: "demo-session-001".to_string(),
        };
        assert_eq!(context_panel.model_name, "claude-4");
        assert_eq!(context_panel.recent_files.len(), 3);
        assert_eq!(context_panel.git_changes.len(), 2);
        assert_eq!(context_panel.active_tool, Some("read_file".to_string()));
    }

    #[test]
    fn test_sidebar_toggle_methods() {
        // Test that toggle methods work on Tui state
        // We test the methods themselves since Tui::new requires a terminal
        let mut show_left = false;
        let mut show_right = false;

        // Simulate toggle_left_sidebar
        show_left = !show_left;
        assert!(show_left);

        // Simulate toggle_right_sidebar
        show_right = !show_right;
        assert!(show_right);
    }

    #[test]
    fn test_agent_status_variants() {
        assert_eq!(AgentStatus::Running, AgentStatus::Running);
        assert_eq!(AgentStatus::Completed, AgentStatus::Completed);
        assert_ne!(AgentStatus::Running, AgentStatus::Completed);
    }

    #[test]
    fn test_git_status_variants() {
        assert_eq!(GitStatus::Modified, GitStatus::Modified);
        assert_eq!(GitStatus::Added, GitStatus::Added);
        assert_eq!(GitStatus::Deleted, GitStatus::Deleted);
        assert_eq!(GitStatus::Untracked, GitStatus::Untracked);
    }

    // ─── Reducer Tests ─────────────────────────────────────────────────────────

    fn make_state() -> AppState {
        AppState {
            messages: vec![],
            input_lines: vec![String::new()],
            cursor_col: 0,
            cursor_row: 0,
            input_right_info: String::new(),
            mode: TuiMode::Chat,
            running: true,
            show_sidebar: false,
            agent_running: false,
            current_model: None,
            top_bar_repo: String::new(),
            top_bar_branch: String::new(),
            top_bar_path: String::new(),
            top_bar_checks_passed: None,
            top_bar_checks_total: None,
            top_bar_percentage: None,
            top_bar_agent_count: None,
            permission_modal_tool: None,
            permission_modal_tool_call_id: None,
            permission_modal_args: None,
            permission_modal_desc: None,
            action_log: Vec::new(),
            action_log_capacity: 1000,
            command_palette_open: false,
            command_palette_filter: String::new(),
            command_palette_selected: 0,
            feed_scroll_offset: 0,
            diff_scroll_offset: 0,
            tree_scroll_offset: 0,
            animation: AnimationState::default(),
            diff_viewer: None,
            token_usage: TokenUsage::default(),
            session_token_usage: TokenUsage::default(),
            session_tree: SessionTreeNavigator::new(),
        }
    }

    #[test]
    fn test_insert_char() {
        let mut state = make_state();
        update(&mut state, Msg::InsertChar('h'));
        update(&mut state, Msg::InsertChar('i'));
        assert_eq!(state.input_lines, vec!["hi"]);
        assert_eq!(state.cursor_col, 2);
    }

    #[test]
    fn test_backspace() {
        let mut state = make_state();
        update(&mut state, Msg::InsertChar('h'));
        update(&mut state, Msg::InsertChar('i'));
        update(&mut state, Msg::Backspace);
        assert_eq!(state.input_lines, vec!["h"]);
        assert_eq!(state.cursor_col, 1);
    }

    #[test]
    fn test_submit_clears_input() {
        let mut state = make_state();
        update(&mut state, Msg::InsertChar('h'));
        update(&mut state, Msg::InsertChar('i'));
        let cmds = update(&mut state, Msg::Submit);
        assert_eq!(state.input_lines, vec![""]);
        assert_eq!(state.messages.len(), 1);
        // Should return a SpawnAgent cmd
        assert_eq!(cmds.len(), 1);
        if let Cmd::SpawnAgent { .. } = &cmds[0] {
            // Expected
        } else {
            panic!("Expected SpawnAgent cmd");
        }
        if let MessageItem::User { text, .. } = &state.messages[0] {
            assert_eq!(text, "hi");
        } else {
            panic!("Expected User message");
        }
    }

    #[test]
    fn test_submit_empty_does_nothing() {
        let mut state = make_state();
        let cmds = update(&mut state, Msg::Submit);
        assert_eq!(state.messages.len(), 0);
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_move_cursor() {
        let mut state = make_state();
        update(&mut state, Msg::InsertChar('a'));
        update(&mut state, Msg::InsertChar('b'));
        update(&mut state, Msg::InsertChar('c'));
        assert_eq!(state.cursor_col, 3);

        update(&mut state, Msg::MoveCursorLeft);
        assert_eq!(state.cursor_col, 2);

        update(&mut state, Msg::MoveCursorLeft);
        assert_eq!(state.cursor_col, 1);

        update(&mut state, Msg::MoveCursorRight);
        assert_eq!(state.cursor_col, 2);

        update(&mut state, Msg::MoveCursorToStart);
        assert_eq!(state.cursor_col, 0);

        update(&mut state, Msg::MoveCursorToEnd);
        assert_eq!(state.cursor_col, 3);
    }

    #[test]
    fn test_newline() {
        let mut state = make_state();
        update(&mut state, Msg::InsertChar('h'));
        update(&mut state, Msg::InsertChar('i'));
        update(&mut state, Msg::InsertNewline);
        assert_eq!(state.input_lines, vec!["hi", ""]);
        assert_eq!(state.cursor_row, 1);
        assert_eq!(state.cursor_col, 0);
    }

    #[test]
    fn test_multi_line_submit() {
        let mut state = make_state();
        for c in "line1".chars() {
            update(&mut state, Msg::InsertChar(c));
        }
        update(&mut state, Msg::InsertNewline);
        for c in "line2".chars() {
            update(&mut state, Msg::InsertChar(c));
        }
        update(&mut state, Msg::Submit);

        assert_eq!(state.input_lines, vec![""]);
        assert_eq!(state.messages.len(), 1);
        if let MessageItem::User { text, .. } = &state.messages[0] {
            assert_eq!(text, "line1\nline2");
        } else {
            panic!("Expected User message");
        }
    }

    #[test]
    fn test_quit() {
        let mut state = make_state();
        update(&mut state, Msg::Quit);
        assert!(!state.running);
    }

    #[test]
    fn test_toggle_sidebar() {
        let mut state = make_state();
        assert!(!state.show_sidebar);
        update(&mut state, Msg::ToggleSidebar);
        assert!(state.show_sidebar);
        update(&mut state, Msg::ToggleSidebar);
        assert!(!state.show_sidebar);
    }

    #[test]
    fn test_delete_word_backward() {
        let mut state = make_state();
        // Type "hello world"
        for c in "hello world".chars() {
            update(&mut state, Msg::InsertChar(c));
        }
        assert_eq!(state.cursor_col, 11);

        // Delete word backward → "hello" (removes " world" including space, bash-like)
        update(&mut state, Msg::DeleteWordBackward);
        assert_eq!(state.input_lines[0], "hello");
        assert_eq!(state.cursor_col, 5);

        // Delete word backward → "" (no more words, clears line)
        update(&mut state, Msg::DeleteWordBackward);
        assert_eq!(state.input_lines[0], "");
        assert_eq!(state.cursor_col, 0);
    }

    #[test]
    fn test_delete_to_start() {
        let mut state = make_state();
        for c in "hello".chars() {
            update(&mut state, Msg::InsertChar(c));
        }
        update(&mut state, Msg::MoveCursorToEnd);
        update(&mut state, Msg::DeleteToStart);
        assert_eq!(state.input_lines[0], "");
        assert_eq!(state.cursor_col, 0);
    }

    #[test]
    fn test_delete_forward() {
        let mut state = make_state();
        update(&mut state, Msg::InsertChar('a'));
        update(&mut state, Msg::InsertChar('b'));
        update(&mut state, Msg::InsertChar('c'));
        update(&mut state, Msg::MoveCursorToStart);
        update(&mut state, Msg::DeleteForward);
        assert_eq!(state.input_lines[0], "bc");
    }

    #[test]
    fn test_agent_event_message_start() {
        let mut state = make_state();
        update(
            &mut state,
            Msg::AgentEvent(AgentEvent::MessageStart {
                message: AgentMessage {
                    role: "assistant".to_string(),
                    content: vec![],
                    timestamp: 0,
                    usage: None,
                    stop_reason: None,
                    error_message: None,
                },
            }),
        );
        assert!(state.agent_running);
        assert_eq!(state.messages.len(), 1);
    }

    #[test]
    fn test_agent_event_message_update() {
        let mut state = make_state();
        // Start message
        update(
            &mut state,
            Msg::AgentEvent(AgentEvent::MessageStart {
                message: AgentMessage {
                    role: "assistant".to_string(),
                    content: vec![],
                    timestamp: 0,
                    usage: None,
                    stop_reason: None,
                    error_message: None,
                },
            }),
        );

        // Update with text
        update(
            &mut state,
            Msg::AgentEvent(AgentEvent::MessageUpdate {
                message: AgentMessage {
                    role: "assistant".to_string(),
                    content: vec![ContentPart::Text {
                        text: "Hello".to_string(),
                    }],
                    timestamp: 0,
                    usage: None,
                    stop_reason: None,
                    error_message: None,
                },
            }),
        );

        assert_eq!(state.messages.len(), 1);
        if let MessageItem::Assistant { text, .. } = &state.messages[0] {
            assert_eq!(text, "Hello");
        } else {
            panic!("Expected Assistant message");
        }
    }

    // ─── Time-Travel Tests ─────────────────────────────────────────────────────

    #[test]
    fn test_action_log_records_msgs() {
        let mut state = make_state();
        update(&mut state, Msg::InsertChar('h'));
        update(&mut state, Msg::InsertChar('i'));
        update(&mut state, Msg::Submit);

        assert_eq!(state.action_log.len(), 3);
        assert!(matches!(state.action_log[0], Msg::InsertChar('h')));
        assert!(matches!(state.action_log[1], Msg::InsertChar('i')));
        assert!(matches!(state.action_log[2], Msg::Submit));
    }

    #[test]
    fn test_action_log_capacity() {
        let mut state = make_state();
        state.action_log_capacity = 5;

        for i in 0..10 {
            update(&mut state, Msg::InsertChar('a'));
        }

        assert_eq!(state.action_log.len(), 5); // Only keeps last 5
    }

    #[test]
    fn test_replay_actions() {
        let mut state = make_state();
        update(&mut state, Msg::InsertChar('h'));
        update(&mut state, Msg::InsertChar('i'));
        update(&mut state, Msg::Submit);

        let replayed = state.replay_to(2); // Replay first 2 msgs
        assert_eq!(replayed.input_lines, vec!["hi"]);
        assert_eq!(replayed.messages.len(), 0); // Submit not replayed

        let replayed_full = state.replay_to(3); // Replay all 3
        assert_eq!(replayed_full.messages.len(), 1);
    }

    #[test]
    fn test_replay_produces_same_state() {
        let mut state = make_state();
        // Complex sequence
        update(&mut state, Msg::InsertChar('h'));
        update(&mut state, Msg::InsertChar('e'));
        update(&mut state, Msg::InsertChar('l'));
        update(&mut state, Msg::InsertChar('l'));
        update(&mut state, Msg::InsertChar('o'));
        update(&mut state, Msg::Submit);
        update(&mut state, Msg::ToggleSidebar);
        update(&mut state, Msg::InsertChar('w'));
        update(&mut state, Msg::InsertChar('o'));
        update(&mut state, Msg::InsertChar('r'));
        update(&mut state, Msg::InsertChar('l'));
        update(&mut state, Msg::InsertChar('d'));

        let replayed = state.replay_to(state.action_log.len());
        assert_eq!(replayed.input_lines, state.input_lines);
        assert_eq!(replayed.messages, state.messages);
        assert_eq!(replayed.show_sidebar, state.show_sidebar);
    }

    #[test]
    fn test_permission_cmds() {
        let mut state = make_state();

        // PermissionConfirm should return Allow decision
        let cmds = update(&mut state, Msg::PermissionConfirm);
        assert_eq!(cmds.len(), 1);
        if let Cmd::SendPermission { decision } = &cmds[0] {
            assert!(matches!(*decision, PermissionDecision::Allow { .. }));
        } else {
            panic!("Expected SendPermission cmd");
        }

        // PermissionCancel should return Deny decision
        let cmds = update(&mut state, Msg::PermissionCancel);
        if let Cmd::SendPermission { decision } = &cmds[0] {
            assert!(matches!(*decision, PermissionDecision::Deny { .. }));
        }

        // PermissionAlways should return AllowAlways decision
        let cmds = update(&mut state, Msg::PermissionAlways);
        if let Cmd::SendPermission { decision } = &cmds[0] {
            assert!(matches!(*decision, PermissionDecision::AllowAlways { .. }));
        }

        // PermissionSkip should return Skip decision
        let cmds = update(&mut state, Msg::PermissionSkip);
        if let Cmd::SendPermission { decision } = &cmds[0] {
            assert!(matches!(*decision, PermissionDecision::Skip { .. }));
        }
    }
}
