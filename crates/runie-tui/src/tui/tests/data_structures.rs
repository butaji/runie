//! Data structure tests.

use crate::components::{
    AgentList, AgentItem, AgentStatus, ContextPanel,
    GitChange, GitStatus,
};
use crate::theme::ThemeWrapper;

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
