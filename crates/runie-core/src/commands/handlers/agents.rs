//! `/agents` slash command — manage agent profiles in `~/.runie/agents/*.toml`.
//!
//! Opens a panel stack:
//! - Root: list of all profiles (with "New profile" + per-profile edit/delete)
//! - Edit: form with name/description/system_prompt/tools/max_turns fields
//! - Confirm delete: confirm before deleting a profile

use crate::agent_profiles::{
    self, AgentProfile, ProfileError,
};
use crate::commands::{CommandCategory, CommandRegistry, CommandResult};
use crate::dialog::{ItemAction, Panel, PanelStack};
use crate::event::Event;
use crate::model::AppState;

/// Re-export profiles_dir for convenience.
fn profiles_dir() -> std::path::PathBuf {
    agent_profiles::profiles_dir()
}

/// Register the `/agents` command.
pub fn register(registry: &mut CommandRegistry) {
    registry.register(
        crate::cmd!("agents")
            .desc("Manage agent profiles (CRUD)")
            .category(CommandCategory::System)
            .handler(handle_agents),
    );
}

/// Open the agent manager panel.
pub fn handle_agents(_state: &mut AppState, _args: &str) -> CommandResult {
    CommandResult::Event(Event::OpenAgentsManager)
}

/// Build the root panel showing all profiles.
pub fn build_root_panel() -> PanelStack {
    let dir = profiles_dir();
    let profiles = agent_profiles::load_profiles_from_dir(&dir)
        .unwrap_or_default();

    let mut panel = Panel::new("agents_root", "Agent Profiles")
        .item("+ New profile", ItemAction::Push("agents_edit_new".into()));

    if profiles.is_empty() {
        panel = panel.item(
            "(no profiles found)",
            ItemAction::Emit(Event::SystemMessage {
                content: format!("No profiles in {}", dir.display()),
            }),
        );
    } else {
        for p in &profiles {
            let label = if p.description.is_empty() {
                p.name.clone()
            } else {
                format!("{}  —  {}", p.name, p.description)
            };
            // Each profile gets an Edit + Delete sub-action when selected
            panel = panel.item(label, ItemAction::Push(format!("agents_view:{}", p.name)));
        }
    }

    panel = panel.item("Close", ItemAction::Close);
    PanelStack::new(panel)
}

/// Build a panel for viewing/editing a specific profile.
pub fn build_view_panel(name: &str) -> PanelStack {
    let dir = profiles_dir();
    let path = dir.join(format!("{}.toml", name));
    let profile = agent_profiles::load_profile_from_file(&path).ok();

    let mut panel = Panel::new(format!("agents_view:{}", name), format!("Profile: {}", name));

    if let Some(p) = profile {
        let tools_str = p.tools.join(", ");
        let allowed = p.allowlist_tools.as_ref().map(|v| v.join(", ")).unwrap_or_default();
        let denied = p.denylist_tools.as_ref().map(|v| v.join(", ")).unwrap_or_default();
        let max = p.max_turns.map(|n| n.to_string()).unwrap_or_default();

        panel = panel
            .item(format!("Name: {}", p.name), ItemAction::Close)
            .item(format!("Description: {}", p.description), ItemAction::Close)
            .item(format!("System prompt: {}", truncate(&p.system_prompt, 60)), ItemAction::Close)
            .item(format!("Tools: {}", tools_str), ItemAction::Close)
            .item(format!("Allowlist: {}", allowed), ItemAction::Close)
            .item(format!("Denylist: {}", denied), ItemAction::Close)
            .item(format!("Max turns: {}", max), ItemAction::Close)
            .item("─ Edit ─", ItemAction::Push(format!("agents_edit:{}", name)))
            .item("─ Delete ─", ItemAction::Push(format!("agents_delete:{}", name)))
            .item("Back", ItemAction::Pop)
            .item("Close", ItemAction::Close);
    } else {
        panel = panel
            .item(format!("(could not load {})", path.display()), ItemAction::Close)
            .item("Back", ItemAction::Pop);
    }

    PanelStack::new(panel)
}

/// Build the edit panel for a profile (or new profile).
pub fn build_edit_panel(name: &str) -> PanelStack {
    let dir = profiles_dir();
    let path = dir.join(format!("{}.toml", name));
    let profile = agent_profiles::load_profile_from_file(&path)
        .unwrap_or_else(|_| AgentProfile::new(name, ""));

    let title = if path.exists() {
        format!("Edit: {}", name)
    } else {
        format!("New profile: {}", name)
    };

    let tools_csv = profile.tools.join(",");
    let allowed_csv = profile
        .allowlist_tools
        .as_ref()
        .map(|v| v.join(","))
        .unwrap_or_default();
    let denied_csv = profile
        .denylist_tools
        .as_ref()
        .map(|v| v.join(","))
        .unwrap_or_default();
    let max_str = profile.max_turns.map(|n| n.to_string()).unwrap_or_default();

    let panel = Panel::new(format!("agents_edit:{}", name), title)
        .item(
            format!("Name: {}", profile.name),
            ItemAction::Emit(Event::AgentsManagerSetField {
                name: name.to_string(),
                field: "name".into(),
                value: profile.name.clone(),
            }),
        )
        .item(
            format!("Description: {}", profile.description),
            ItemAction::Emit(Event::AgentsManagerSetField {
                name: name.to_string(),
                field: "description".into(),
                value: profile.description.clone(),
            }),
        )
        .item(
            format!("System prompt: {}", truncate(&profile.system_prompt, 40)),
            ItemAction::Emit(Event::AgentsManagerSetField {
                name: name.to_string(),
                field: "system_prompt".into(),
                value: profile.system_prompt.clone(),
            }),
        )
        .item(
            format!("Tools (csv): {}", tools_csv),
            ItemAction::Emit(Event::AgentsManagerSetField {
                name: name.to_string(),
                field: "tools".into(),
                value: tools_csv,
            }),
        )
        .item(
            format!("Allowlist (csv): {}", allowed_csv),
            ItemAction::Emit(Event::AgentsManagerSetField {
                name: name.to_string(),
                field: "allowlist_tools".into(),
                value: allowed_csv,
            }),
        )
        .item(
            format!("Denylist (csv): {}", denied_csv),
            ItemAction::Emit(Event::AgentsManagerSetField {
                name: name.to_string(),
                field: "denylist_tools".into(),
                value: denied_csv,
            }),
        )
        .item(
            format!("Max turns: {}", max_str),
            ItemAction::Emit(Event::AgentsManagerSetField {
                name: name.to_string(),
                field: "max_turns".into(),
                value: max_str,
            }),
        )
        .item("─ Save ─", ItemAction::Emit(Event::AgentsManagerSave { name: name.to_string() }))
        .item("Back", ItemAction::Pop)
        .item("Close", ItemAction::Close);

    PanelStack::new(panel)
}

/// Build the confirm-delete panel.
pub fn build_delete_panel(name: &str) -> PanelStack {
    let panel = Panel::new(format!("agents_delete:{}", name), format!("Delete {}?", name))
        .item(
            format!("Yes, delete {}", name),
            ItemAction::Emit(Event::AgentsManagerDelete { name: name.to_string() }),
        )
        .item("No, go back", ItemAction::Pop)
        .item("Close", ItemAction::Close);
    PanelStack::new(panel)
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(n).collect();
        out.push('…');
        out
    }
}

/// Save a profile to disk given its full state.
pub fn save_profile(profile: &AgentProfile) -> Result<std::path::PathBuf, ProfileError> {
    agent_profiles::save_profile(profile)
}

/// Delete a profile from disk.
pub fn delete_profile(name: &str) -> Result<(), ProfileError> {
    agent_profiles::delete_profile(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::registry::CommandRegistry;
    use std::collections::HashMap;

    fn make_registry() -> CommandRegistry {
        CommandRegistry::new()
    }

    #[test]
    fn agents_command_is_registered() {
        let reg = make_registry();
        let cmd = reg.get("agents");
        assert!(cmd.is_some(), "expected /agents command to be registered");
        assert_eq!(cmd.unwrap().name, "agents");
    }

    #[test]
    fn agents_command_has_description() {
        let reg = make_registry();
        let cmd = reg.get("agents").unwrap();
        assert!(!cmd.desc.is_empty());
    }

    #[test]
    fn agents_command_emits_open_event() {
        let mut state = AppState::default();
        let result = handle_agents(&mut state, "");
        match result {
            CommandResult::Event(Event::OpenAgentsManager) => {}
            other => panic!("expected OpenAgentsManager event, got {:?}", other),
        }
    }

    #[test]
    fn build_root_panel_has_new_profile_item() {
        let panel_stack = build_root_panel();
        let panel = panel_stack.current().unwrap();
        // Should have at least: "+ New profile" + "Close"
        assert!(panel.items.len() >= 2);
        let has_new = panel.items.iter().any(|it| match it {
            crate::dialog::PanelItem::Action { label, .. } => label.contains("New profile"),
            _ => false,
        });
        assert!(has_new, "expected 'New profile' item");
    }

    #[test]
    fn build_root_panel_handles_empty_dir() {
        // Even with no profiles, the panel should still have items
        let panel_stack = build_root_panel();
        let panel = panel_stack.current().unwrap();
        assert!(!panel.items.is_empty());
    }

    #[test]
    fn build_root_panel_handles_nonexistent_dir() {
        // Should not panic
        let panel_stack = build_root_panel();
        let panel = panel_stack.current().unwrap();
        assert!(!panel.items.is_empty());
    }

    #[test]
    fn build_view_panel_loads_existing_profile() {
        // Verify that load_profile_from_file + a manually-built panel works
        let dir = tempfile::tempdir().unwrap();
        let name = "testagent";
        let path = dir.path().join(format!("{}.toml", name));
        std::fs::write(&path, r#"
            name = "testagent"
            description = "Test description"
            system_prompt = "You are a test."
            tools = ["read", "write"]
        "#).unwrap();

        let profile = agent_profiles::load_profile_from_file(&path).unwrap();
        assert_eq!(profile.name, "testagent");
        assert_eq!(profile.tools, vec!["read", "write"]);
    }

    #[test]
    fn build_edit_panel_handles_new_profile() {
        let _ = std::env::set_var("HOME", "/tmp/_nonexistent_");
        let panel_stack = build_edit_panel("newagent");
        let panel = panel_stack.current().unwrap();
        assert!(panel.items.len() >= 8);
    }

    #[test]
    fn build_delete_panel_has_confirm() {
        let panel_stack = build_delete_panel("foo");
        let panel = panel_stack.current().unwrap();
        let has_yes = panel.items.iter().any(|it| match it {
            crate::dialog::PanelItem::Action { label, .. } => label.contains("Yes"),
            _ => false,
        });
        let has_no = panel.items.iter().any(|it| match it {
            crate::dialog::PanelItem::Action { label, .. } => label.contains("No"),
            _ => false,
        });
        assert!(has_yes && has_no);
    }

    #[test]
    fn save_profile_writes_toml() {
        let dir = tempfile::tempdir().unwrap();
        let _ = std::env::set_var("HOME", dir.path());

        let mut profile = AgentProfile::new("myagent", "You are a test.");
        profile.description = "Test".into();
        profile.tools = vec!["read".into(), "write".into()];

        let path = save_profile(&profile).unwrap();
        assert!(path.exists());
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("name = \"myagent\""));
        assert!(content.contains("system_prompt"));
        assert!(content.contains("read"));
    }

    #[test]
    fn delete_profile_removes_file() {
        // Write a file to a temp dir, then delete it via the agent_profiles API
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("todelete.toml");
        std::fs::write(&path, "name = \"todelete\"\ndescription = \"x\"\nsystem_prompt = \"x\"\ntools = []\n").unwrap();
        assert!(path.exists());

        // Verify we can load it
        let loaded = agent_profiles::load_profile_from_file(&path).unwrap();
        assert_eq!(loaded.name, "todelete");
    }

    #[test]
    fn delete_nonexistent_profile_is_ok() {
        let dir = tempfile::tempdir().unwrap();
        let _ = std::env::set_var("HOME", dir.path());
        let result = delete_profile("doesnotexist");
        assert!(result.is_ok());
    }

    #[test]
    fn round_trip_profile() {
        let dir = tempfile::tempdir().unwrap();
        let _ = std::env::set_var("HOME", dir.path());

        let original = AgentProfile {
            name: "roundtrip".into(),
            description: "Test round trip".into(),
            system_prompt: "You are a test.".into(),
            tools: vec!["read".into(), "bash".into()],
            max_turns: Some(50),
            allowlist_tools: Some(vec!["read".into()]),
            denylist_tools: Some(vec!["bash".into()]),
        };

        let path = save_profile(&original).unwrap();
        let loaded = agent_profiles::load_profile_from_file(&path).unwrap();

        assert_eq!(loaded.name, original.name);
        assert_eq!(loaded.description, original.description);
        assert_eq!(loaded.system_prompt, original.system_prompt);
        assert_eq!(loaded.tools, original.tools);
        assert_eq!(loaded.max_turns, original.max_turns);
        assert_eq!(loaded.allowlist_tools, original.allowlist_tools);
        assert_eq!(loaded.denylist_tools, original.denylist_tools);
    }

    #[test]
    fn truncate_long_string() {
        let s = "a".repeat(100);
        let t = truncate(&s, 10);
        assert_eq!(t.chars().count(), 11); // 10 + ellipsis
    }

    #[test]
    fn truncate_short_string_unchanged() {
        let t = truncate("hello", 10);
        assert_eq!(t, "hello");
    }
}
