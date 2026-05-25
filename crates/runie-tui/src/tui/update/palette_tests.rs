//! Tests for palette update functions - handle_direct_command and handle_close_modal.

use crate::tui::state::{AppState, TuiMode, Msg, Cmd};
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

        // Old messages cleared and system message added
        assert_eq!(state.messages.len(), 1);
        assert!(matches!(&state.messages[0], MessageItem::System { text } if text.contains("Chat cleared")));
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_quit_sets_running_false() {
        let mut state = make_state();
        assert!(state.running);

        let cmds = handle_direct_command(&mut state, PaletteCommand::Quit);

        assert!(!state.running);
        assert!(cmds.is_empty());
        // Should have goodbye message
        assert!(state.messages.iter().any(|m| matches!(m, MessageItem::System { text } if text.contains("Goodbye"))));
    }

    #[test]
    fn test_load_session_pushes_cmd_with_name() {
        let mut state = make_state();

        let cmds = handle_direct_command(&mut state, PaletteCommand::LoadSession { name: "my_session".to_string() });

        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0], Cmd::LoadSession { name: "my_session".to_string() });
        // Should have system message
        assert!(state.messages.iter().any(|m| matches!(m, MessageItem::System { text } if text.contains("Loading session"))));
    }

    #[test]
    fn test_save_session_pushes_cmd_with_name() {
        let mut state = make_state();

        let cmds = handle_direct_command(&mut state, PaletteCommand::SaveSession { name: "save_me".to_string() });

        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0], Cmd::SaveSession { name: Some("save_me".to_string()) });
        assert!(state.messages.iter().any(|m| matches!(m, MessageItem::System { text } if text.contains("Saving session"))));
    }

    #[test]
    fn test_switch_model_opens_model_picker() {
        let mut state = make_state();

        let cmds = handle_direct_command(&mut state, PaletteCommand::SwitchModel);

        // No commands issued - just state change
        assert!(cmds.is_empty());
        // Should switch to Overlay mode for model picker
        assert_eq!(state.mode, TuiMode::Overlay);
        // Model picker should be populated
        assert!(!state.model_picker_items.is_empty());
        assert_eq!(state.model_picker_title, "Select Model");
        assert_eq!(state.model_picker_selected, 0);
    }

    #[test]
    fn test_read_file_pushes_cmd() {
        let mut state = make_state();

        let cmds = handle_direct_command(&mut state, PaletteCommand::ReadFile { path: "/tmp/test.txt".to_string() });

        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0], Cmd::ReadFile { path: "/tmp/test.txt".to_string() });
        assert!(state.messages.iter().any(|m| matches!(m, MessageItem::System { text } if text.contains("Reading file"))));
    }

    #[test]
    fn test_edit_file_pushes_cmd() {
        let mut state = make_state();

        let cmds = handle_direct_command(&mut state, PaletteCommand::EditFile { path: "/tmp/test.txt".to_string() });

        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0], Cmd::EditFile { path: "/tmp/test.txt".to_string() });
        assert!(state.messages.iter().any(|m| matches!(m, MessageItem::System { text } if text.contains("Editing file"))));
    }

    #[test]
    fn test_write_file_pushes_cmd() {
        let mut state = make_state();

        let cmds = handle_direct_command(&mut state, PaletteCommand::WriteFile { path: "/tmp/new.txt".to_string() });

        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0], Cmd::WriteFile { path: "/tmp/new.txt".to_string() });
        assert!(state.messages.iter().any(|m| matches!(m, MessageItem::System { text } if text.contains("Writing file"))));
    }

    #[test]
    fn test_delete_file_pushes_cmd() {
        let mut state = make_state();

        let cmds = handle_direct_command(&mut state, PaletteCommand::DeleteFile { path: "/tmp/old.txt".to_string() });

        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0], Cmd::DeleteFile { path: "/tmp/old.txt".to_string() });
        assert!(state.messages.iter().any(|m| matches!(m, MessageItem::System { text } if text.contains("Deleting file"))));
    }

    #[test]
    fn test_compact_context_pushes_cmd() {
        let mut state = make_state();

        let cmds = handle_direct_command(&mut state, PaletteCommand::CompactContext);

        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0], Cmd::CompactContext);
        assert!(state.messages.iter().any(|m| matches!(m, MessageItem::System { text } if text.contains("Compacting context"))));
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

    #[test]
    fn test_full_flow_read_file() {
        let mut state = make_state();
        state.mode = TuiMode::CommandPalette;
        state.command_palette.open = true;

        let cmds = update(&mut state, &mut make_palette(), Msg::DirectCommand(PaletteCommand::ReadFile { path: "/tmp/test.txt".to_string() }));

        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0], Cmd::ReadFile { path: "/tmp/test.txt".to_string() });
    }

    #[test]
    fn test_full_flow_save_session() {
        let mut state = make_state();
        state.mode = TuiMode::CommandPalette;
        state.command_palette.open = true;

        let cmds = update(&mut state, &mut make_palette(), Msg::DirectCommand(PaletteCommand::SaveSession { name: "my_work".to_string() }));

        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0], Cmd::SaveSession { name: Some("my_work".to_string()) });
    }
}
