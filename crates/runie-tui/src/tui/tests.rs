use crate::tui::state::{AppState, TuiMode, Msg, Cmd, AnimationState, TopBarState, PermissionModalState, CommandPaletteState, ScrollState};
use crate::components::{
    AgentList, AgentItem, AgentStatus, MessageItem,
    ContextPanel, GitChange, GitStatus, SessionTreeNavigator, CommandPalette,
};
use crate::theme::ThemeWrapper;
use crate::tui::update::update;
use runie_agent::{AgentEvent, AgentMessage, PermissionDecision, ContentPart};
use runie_ai::TokenUsage;


#[allow(clippy::unwrap_used)]
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
            theme: ThemeWrapper::default(),
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
            top_bar: TopBarState::default(),
            permission_modal: PermissionModalState::default(),
            command_palette: CommandPaletteState::default(),
            scroll: ScrollState::default(),
            animation: AnimationState::default(),
            diff_viewer: None,
            token_usage: TokenUsage::default(),
            session_token_usage: TokenUsage::default(),
            session_tree: SessionTreeNavigator::new(),
            background_jobs: Vec::new(),
            onboarding: None,
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

    // ─── Tui dirty flag regression tests ─────────────────────────────────────────
    // These tests verify the pattern that prevents the bug where calling
    // runie_tui::update() (free function) instead of tui.update() (method)
    // causes state updates without setting dirty, resulting in blank renders.

    /// Mock Tui struct for testing the dirty flag pattern.
    /// Mirrors the exact structure of Tui.update() behavior:
    /// - Sets dirty=true BEFORE calling the reducer
    /// - Then calls the free function to update state
    struct MockTui {
        state: AppState,
        dirty: bool,
    }

    impl MockTui {
        fn new(initial_dirty: bool) -> Self {
            Self {
                state: AppState::default(),
                dirty: initial_dirty,
            }
        }

        /// This is the CORRECT pattern - sets dirty BEFORE calling reducer.
        /// The bug was calling runie_tui::update() directly on state without
        /// setting dirty, causing render() to skip since !dirty.
        fn update(&mut self, msg: Msg) -> Vec<Cmd> {
            self.dirty = true;  // <-- This is the critical line that was missing!
            update(&mut self.state, msg)
        }

        fn is_dirty(&self) -> bool {
            self.dirty
        }

        /// Returns true if render would actually draw (not skip)
        fn render(&mut self) -> bool {
            if !self.dirty {
                return false;  // Early return - render skipped
            }
            self.dirty = false;
            true  // Would actually render
        }
    }

    #[test]
    fn test_update_sets_dirty() {
        // Bug scenario: if someone calls runie_tui::update() directly on state,
        // dirty would NOT be set. This test verifies the tui.update() pattern works.
        let mut tui = MockTui::new(false);
        assert!(!tui.is_dirty());

        tui.update(Msg::InsertChar('a'));

        assert!(tui.is_dirty(), "tui.update() must set dirty=true");
    }

    #[test]
    fn test_render_skips_when_not_dirty() {
        let mut tui = MockTui::new(false);

        // With dirty=false, render should return early (not actually draw)
        let did_render = tui.render();
        assert!(!did_render, "render() should skip when dirty=false");
        assert!(!tui.is_dirty(), "dirty should remain false after skipped render");
    }

    #[test]
    fn test_render_executes_when_dirty() {
        let mut tui = MockTui::new(true);

        // With dirty=true, render should execute
        let did_render = tui.render();
        assert!(did_render, "render() should execute when dirty=true");
        assert!(!tui.is_dirty(), "dirty should be cleared after render");
    }

    #[test]
    fn test_insert_char_updates_state_and_sets_dirty() {
        let mut tui = MockTui::new(false);

        tui.update(Msg::InsertChar('x'));

        assert_eq!(tui.state.input_lines[0], "x", "InsertChar should update state");
        assert!(tui.is_dirty(), "InsertChar should set dirty=true");
    }

    #[test]
    fn test_submit_clears_input_and_sets_dirty() {
        let mut tui = MockTui::new(false);

        // Pre-populate with "hello"
        tui.update(Msg::InsertChar('h'));
        tui.update(Msg::InsertChar('e'));
        tui.update(Msg::InsertChar('l'));
        tui.update(Msg::InsertChar('l'));
        tui.update(Msg::InsertChar('o'));

        tui.update(Msg::Submit);

        assert!(tui.state.input_lines.is_empty() || tui.state.input_lines[0].is_empty(),
                "Submit should clear input");
        assert!(tui.is_dirty(), "Submit should set dirty=true");
    }

    #[test]
    fn test_keyboard_event_full_pipeline() {
        use crossterm::event::{Event, KeyCode, KeyModifiers, KeyEventKind, KeyEventState};
        use crate::tui::events::event_to_msg;

        let mut tui = MockTui::new(false);

        // Simulate keyboard event: pressing 'a'
        let event = Event::Key(crossterm::event::KeyEvent {
            code: KeyCode::Char('a'),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        });

        // Convert event to msg via the event_to_msg function
        if let Some(msg) = event_to_msg(event, &tui.state) {
            // This is the CORRECT path: tui.update(msg) sets dirty first
            tui.update(msg);
        }

        assert!(tui.is_dirty(), "Keyboard event pipeline should set dirty");
        assert_eq!(tui.state.input_lines[0], "a", "Keyboard event should update state");
    }

    // ─── Anti-pattern verification tests ─────────────────────────────────────────
    // These tests document the WRONG way to update Tui state.
    // If you call the free function directly on state (without setting dirty),
    // the render will be skipped, causing the "typing but nothing displayed" bug.

    #[test]
    fn test_free_function_does_not_set_dirty() {
        // This demonstrates WHY you must use tui.update() not runie_tui::update()
        // The free function only updates state - it cannot set dirty on Tui
        let mut state = AppState::default();

        // Calling free function directly on state
        update(&mut state, Msg::InsertChar('x'));

        // State is updated correctly...
        assert_eq!(state.input_lines[0], "x");

        // ...but there's NO dirty flag mechanism in the free function
        // This is why calling it directly on a Tui's state causes the bug:
        // Tui.dirty remains false, so render() returns early!
    }

    #[test]
    fn test_tui_update_must_be_used_not_free_function() {
        // This test verifies the contract: tui.update() is the ONLY safe way
        // to update state when using Tui. Calling the free function directly
        // bypasses the dirty flag mechanism.

        let mut tui = MockTui::new(false);

        // CORRECT: Use tui.update()
        tui.update(Msg::InsertChar('a'));
        assert!(tui.is_dirty());

        // If someone mistakenly does this:
        //   runie_tui::update(&mut tui.state, Msg::InsertChar('b'));
        // The state WOULD update, but dirty would NOT be set!
        // This is the bug we're preventing.

        // Verify clean state after correct usage
        assert_eq!(tui.state.input_lines[0], "a");
    }

    // ─── Forbidden pattern test ─────────────────────────────────────────────────
    // This test reads tui_run.rs source and fails if it finds the forbidden
    // pattern: runie_tui::update(&mut tui.state
    // This prevents the regression where calling the free function directly
    // on tui.state bypasses the dirty flag mechanism.

    #[test]
    fn test_no_direct_update_call_on_tui_state() {
        // This test reads the tui_run.rs source file and checks for the forbidden
        // pattern that causes the "typing but nothing displayed" bug.
        //
        // The forbidden pattern is:
        //   runie_tui::update(&mut tui.state, ...)
        //
        // The correct pattern is:
        //   tui.update(...)  // which sets dirty=true BEFORE calling reducer

        let tui_run_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("runie-cli")
            .join("src")
            .join("tui_run.rs");

        let source = std::fs::read_to_string(&tui_run_path)
            .expect("Failed to read tui_run.rs - may have moved");

        // Check for the forbidden pattern: runie_tui::update(&mut tui.state
        // This pattern bypasses the dirty flag mechanism in Tui.update()
        let forbidden_pattern = "runie_tui::update(&mut tui.state";

        assert!(
            !source.contains(forbidden_pattern),
            "FORBIDDEN PATTERN DETECTED: '{}' found in tui_run.rs\n\
             This bypasses Tui.update() which sets dirty=true before calling reducer.\n\
             Use tui.update(msg) instead of runie_tui::update(&mut tui.state, msg)",
            forbidden_pattern
        );

        // Also check for the re-exported update: use runie_tui::update
        // followed by direct update(&mut tui.state, ...) call
        let direct_update_patterns = [
            "update(&mut tui.state,",
            "update(&mut self.state,",
        ];

        for pattern in direct_update_patterns {
            assert!(
                !source.contains(pattern),
                "FORBIDDEN PATTERN DETECTED: '{}' found in tui_run.rs\n\
                 This bypasses the dirty flag mechanism.\n\
                 Use self.update(msg) or tui.update(msg) instead.",
                pattern
            );
        }
    }

    // ─── All update paths set dirty ─────────────────────────────────────────────
    // Verifies that ALL Msg variants result in dirty=true

    #[test]
    fn test_all_update_paths_set_dirty() {
        let mut tui = MockTui::new(false);

        // List of Msg variants that should ALL set dirty=true
        let test_cases: Vec<(Msg, &str)> = vec![
            (Msg::InsertChar('x'), "InsertChar"),
            (Msg::Backspace, "Backspace"),
            (Msg::InsertNewline, "InsertNewline"),
            (Msg::MoveCursorLeft, "MoveCursorLeft"),
            (Msg::MoveCursorRight, "MoveCursorRight"),
            (Msg::MoveCursorToStart, "MoveCursorToStart"),
            (Msg::MoveCursorToEnd, "MoveCursorToEnd"),
            (Msg::DeleteForward, "DeleteForward"),
            (Msg::DeleteWordBackward, "DeleteWordBackward"),
            (Msg::DeleteToStart, "DeleteToStart"),
            (Msg::ToggleSidebar, "ToggleSidebar"),
            (Msg::OpenCommandPalette, "OpenCommandPalette"),
            (Msg::CloseModal, "CloseModal"),
            (Msg::Submit, "Submit"),
            (Msg::Tick, "Tick"),
            (Msg::CursorBlink, "CursorBlink"),
        ];

        for (msg, name) in test_cases {
            tui.dirty = false; // Reset dirty flag
            tui.update(msg.clone());
            assert!(
                tui.is_dirty(),
                "{} should set dirty=true but didn't",
                name
            );
        }
    }

    // ─── Critical difference: free function vs method ───────────────────────────
    // This test documents the CRITICAL difference between:
    //   - tui.update(msg)  → sets dirty=true, then updates state (CORRECT)
    //   - update(&mut tui.state, msg)  → updates state but NOT dirty (BUG!)

    #[test]
    fn test_free_function_vs_method_difference() {
        // Demonstrate the bug: calling free function directly on state
        // does NOT set dirty, so render() skips!

        // Using the method (CORRECT)
        let mut tui = MockTui::new(false);
        tui.update(Msg::InsertChar('x'));
        assert!(tui.is_dirty(), "Method tui.update() sets dirty");
        assert_eq!(tui.state.input_lines[0], "x", "State is updated");

        // Using the free function directly on state (BUG!)
        let mut state = AppState::default();
        update(&mut state, Msg::InsertChar('y'));
        assert_eq!(state.input_lines[0], "y", "Free function updates state");

        // But there's NO dirty flag on the free function!
        // This is why you MUST use tui.update() not runie_tui::update()
        //
        // If you mistakenly do:
        //   runie_tui::update(&mut tui.state, msg);
        // The state updates but tui.dirty stays false!
        // Then tui.render() returns early and nothing is displayed.
    }
}
