//! `/agents` slash command — manage agent profiles in `~/.runie/agents/*.toml`.

use crate::agent_profiles::{self, AgentProfile, ProfileError};
use crate::commands::{CommandCategory, CommandRegistry, CommandResult};
use crate::dialog::{ItemAction, Panel, PanelStack};
use crate::event::{DialogEvent, Event, SystemEvent};
use crate::model::AppState;

use super::spec::{CommandKind, CommandSpec};

fn profiles_dir() -> std::path::PathBuf {
    agent_profiles::profiles_dir()
}

static AGENTS_COMMANDS: &[CommandSpec] = &[CommandSpec {
    name: "agents",
    desc: "Manage agent profiles (CRUD)",
    aliases: &[],
    category: CommandCategory::System,
    sub: false,
    kind: CommandKind::Handler(handle_agents),
}];

pub fn register(registry: &mut CommandRegistry) {
    super::spec::register_commands(registry, AGENTS_COMMANDS);
}

/// Open the agent manager panel.
pub fn handle_agents(_state: &mut AppState, _args: &str) -> CommandResult {
    CommandResult::Event(Event::Dialog(DialogEvent::OpenAgentsManager))
}

/// Build the root panel showing all profiles.
pub fn build_root_panel() -> PanelStack {
    let dir = profiles_dir();
    let profiles = agent_profiles::load_profiles_from_dir(&dir).unwrap_or_default();

    let mut panel = Panel::new("agents_root", "Agent Profiles")
        .item("+ New profile", ItemAction::Push("agents_edit_new".into()));

    if profiles.is_empty() {
        panel = panel.item(
            "(no profiles found)",
            ItemAction::Emit(Event::System(SystemEvent::SystemMessage {
                content: format!("No profiles in {}", dir.display()),
            })),
        );
    } else {
        for p in &profiles {
            let label = if p.description.is_empty() {
                p.name.clone()
            } else {
                format!("{}  —  {}", p.name, p.description)
            };
            panel = panel.item(label, ItemAction::Push(format!("agents_view:{}", p.name)));
        }
    }

    panel = panel.item("Close", ItemAction::Close);
    PanelStack::new(panel)
}

/// Build a panel for viewing a specific profile.
pub fn build_view_panel(name: &str) -> PanelStack {
    let dir = profiles_dir();
    let path = dir.join(format!("{}.toml", name));
    let profile = agent_profiles::load_profile_from_file(&path).ok();

    let mut panel = Panel::new(
        format!("agents_view:{}", name),
        format!("Profile: {}", name),
    );

    panel = if let Some(p) = profile {
        build_profile_view_items(panel, name, &p)
    } else {
        build_profile_missing_items(panel, &path)
    };

    PanelStack::new(panel)
}

fn build_profile_view_items(panel: Panel, name: &str, p: &AgentProfile) -> Panel {
    let tools_str = p.tools.join(", ");
    let allowed = join_optional(&p.allowlist_tools);
    let denied = join_optional(&p.denylist_tools);
    let max = p.max_turns.map(|n| n.to_string()).unwrap_or_default();

    panel
        .item(format!("Name: {}", p.name), ItemAction::Close)
        .item(format!("Description: {}", p.description), ItemAction::Close)
        .item(
            format!("System prompt: {}", truncate(&p.system_prompt, 60)),
            ItemAction::Close,
        )
        .item(format!("Tools: {}", tools_str), ItemAction::Close)
        .item(format!("Allowlist: {}", allowed), ItemAction::Close)
        .item(format!("Denylist: {}", denied), ItemAction::Close)
        .item(format!("Max turns: {}", max), ItemAction::Close)
        .item(
            "─ Edit ─",
            ItemAction::Push(format!("agents_edit:{}", name)),
        )
        .item(
            "─ Delete ─",
            ItemAction::Push(format!("agents_delete:{}", name)),
        )
        .item("Back", ItemAction::Pop)
        .item("Close", ItemAction::Close)
}

fn build_profile_missing_items(panel: Panel, path: &std::path::Path) -> Panel {
    panel
        .item(
            format!("(could not load {})", path.display()),
            ItemAction::Close,
        )
        .item("Back", ItemAction::Pop)
}

fn join_optional(list: &Option<Vec<String>>) -> String {
    list.as_ref().map(|v| v.join(", ")).unwrap_or_default()
}

/// Build the edit panel for a profile (or new profile).
pub fn build_edit_panel(name: &str) -> PanelStack {
    let dir = profiles_dir();
    let path = dir.join(format!("{}.toml", name));
    let profile =
        agent_profiles::load_profile_from_file(&path).unwrap_or_else(|_| AgentProfile::new(name, ""));

    let title = edit_panel_title(name, &path);
    let panel = Panel::new(format!("agents_edit:{}", name), title);
    let panel = add_edit_field_items(panel, name, &profile);
    PanelStack::new(add_edit_actions(panel, name))
}

fn edit_panel_title(name: &str, path: &std::path::Path) -> String {
    if path.exists() {
        format!("Edit: {}", name)
    } else {
        format!("New profile: {}", name)
    }
}

fn add_edit_field_items(panel: Panel, name: &str, profile: &AgentProfile) -> Panel {
    let tools_csv = profile.tools.join(",");
    let allowed_csv = join_optional_csv(&profile.allowlist_tools);
    let denied_csv = join_optional_csv(&profile.denylist_tools);
    let max_str = profile.max_turns.map(|n| n.to_string()).unwrap_or_default();

    panel
        .item(
            format!("Name: {}", profile.name),
            edit_field_event(name, "name", &profile.name),
        )
        .item(
            format!("Description: {}", profile.description),
            edit_field_event(name, "description", &profile.description),
        )
        .item(
            format!("System prompt: {}", truncate(&profile.system_prompt, 40)),
            edit_field_event(name, "system_prompt", &profile.system_prompt),
        )
        .item(
            format!("Tools (csv): {}", tools_csv),
            edit_field_event(name, "tools", &tools_csv),
        )
        .item(
            format!("Allowlist (csv): {}", allowed_csv),
            edit_field_event(name, "allowlist_tools", &allowed_csv),
        )
        .item(
            format!("Denylist (csv): {}", denied_csv),
            edit_field_event(name, "denylist_tools", &denied_csv),
        )
        .item(
            format!("Max turns: {}", max_str),
            edit_field_event(name, "max_turns", &max_str),
        )
}

fn join_optional_csv(list: &Option<Vec<String>>) -> String {
    list.as_ref().map(|v| v.join(",")).unwrap_or_default()
}

fn edit_field_event(name: &str, field: &str, value: &str) -> ItemAction {
    ItemAction::Emit(Event::Dialog(DialogEvent::AgentsManagerSetField {
        name: name.to_string(),
        field: field.into(),
        value: value.to_string(),
    }))
}

fn add_edit_actions(panel: Panel, name: &str) -> Panel {
    panel
        .item(
            "─ Save ─",
            ItemAction::Emit(Event::Dialog(DialogEvent::AgentsManagerSave {
                name: name.to_string(),
            })),
        )
        .item("Back", ItemAction::Pop)
        .item("Close", ItemAction::Close)
}

/// Build the confirm-delete panel.
pub fn build_delete_panel(name: &str) -> PanelStack {
    let panel = Panel::new(format!("agents_delete:{}", name), format!("Delete {}?", name))
        .item(
            format!("Yes, delete {}", name),
            ItemAction::Emit(Event::Dialog(DialogEvent::AgentsManagerDelete {
                name: name.to_string(),
            })),
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
    use crate::tests::ENV_LOCK;

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
            CommandResult::Event(Event::Dialog(DialogEvent::OpenAgentsManager)) => {}
            other => panic!("expected OpenAgentsManager event, got {:?}", other),
        }
    }

    #[test]
    fn build_root_panel_has_new_profile_item() {
        let panel_stack = build_root_panel();
        let panel = panel_stack.current().unwrap();
        assert!(panel.items.len() >= 2);
        let has_new = panel.items.iter().any(|it| match it {
            crate::dialog::PanelItem::Action { label, .. } => label.contains("New profile"),
            _ => false,
        });
        assert!(has_new, "expected 'New profile' item");
    }

    #[test]
    fn build_root_panel_handles_empty_dir() {
        let panel_stack = build_root_panel();
        let panel = panel_stack.current().unwrap();
        assert!(!panel.items.is_empty());
    }

    #[test]
    fn build_root_panel_handles_nonexistent_dir() {
        let panel_stack = build_root_panel();
        let panel = panel_stack.current().unwrap();
        assert!(!panel.items.is_empty());
    }

    #[test]
    fn build_view_panel_loads_existing_profile() {
        let dir = tempfile::tempdir().unwrap();
        let name = "testagent";
        let path = dir.path().join(format!("{}.toml", name));
        std::fs::write(
            &path,
            r#"
            name = "testagent"
            description = "Test description"
            system_prompt = "You are a test."
            tools = ["read", "write"]
        "#,
        )
        .unwrap();

        let profile = agent_profiles::load_profile_from_file(&path).unwrap();
        assert_eq!(profile.name, "testagent");
        assert_eq!(profile.tools, vec!["read", "write"]);
    }

    #[test]
    fn build_edit_panel_handles_new_profile() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("HOME", "/tmp/_nonexistent_");
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
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("HOME", dir.path());

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
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("todelete.toml");
        std::fs::write(
            &path,
            "name = \"todelete\"\ndescription = \"x\"\nsystem_prompt = \"x\"\ntools = []\n",
        )
        .unwrap();
        assert!(path.exists());

        let loaded = agent_profiles::load_profile_from_file(&path).unwrap();
        assert_eq!(loaded.name, "todelete");
    }

    #[test]
    fn delete_nonexistent_profile_is_ok() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("HOME", dir.path());
        let result = delete_profile("doesnotexist");
        assert!(result.is_ok());
    }

    #[test]
    fn round_trip_profile() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("HOME", dir.path());

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
        assert_eq!(t.chars().count(), 11);
    }

    #[test]
    fn truncate_short_string_unchanged() {
        assert_eq!(truncate("hello", 10), "hello");
    }
}
