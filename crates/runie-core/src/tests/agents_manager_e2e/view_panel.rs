use crate::agent_profiles::{parse_profile, AgentProfile};
use crate::commands::handlers::agents::build_view_panel;
use crate::dialog::{ItemAction, PanelItem};

#[test]
fn view_panel_for_loaded_profile_has_all_fields() {
    let profile = AgentProfile {
        name: "foo".into(),
        description: "Foo description".into(),
        system_prompt: "You are foo".into(),
        tools: vec!["read".into(), "write".into()],
        max_turns: Some(10),
        allowlist_tools: Some(vec!["read".into()]),
        denylist_tools: None,
    };
    let toml_str = toml::to_string_pretty(&profile).unwrap();
    assert!(toml_str.contains("name = \"foo\""));
    assert!(toml_str.contains("description = \"Foo description\""));
    assert!(toml_str.contains("system_prompt"));
    assert!(toml_str.contains("read"));
    assert!(toml_str.contains("max_turns = 10"));
}

#[test]
fn view_panel_handles_missing_profile() {
    let stack = build_view_panel("does_not_exist");
    let panel = stack.current().unwrap();
    let has_back = panel.items.iter().any(|it| {
        if let PanelItem::Action { action, .. } = it {
            matches!(action, ItemAction::Pop)
        } else {
            false
        }
    });
    assert!(has_back, "Missing profile panel should have Back action");
}

#[test]
fn view_panel_with_empty_fields_works() {
    let profile = AgentProfile::new("minimal", "");
    let toml_str = toml::to_string_pretty(&profile).unwrap();
    let parsed = parse_profile(&toml_str).unwrap();
    assert_eq!(parsed.name, "minimal");
    assert_eq!(parsed.tools.len(), 0);
}
