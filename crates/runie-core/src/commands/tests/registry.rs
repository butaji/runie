use crate::model::AppState;

#[test]
fn registry_get_by_name() {
    let state = AppState::default();
    let cmd = state.registry.get("model");
    assert!(cmd.is_some());
    assert_eq!(cmd.unwrap().name, "model");
}

#[test]
fn registry_get_by_alias() {
    let state = AppState::default();
    let cmd = state.registry.get("m");
    assert!(cmd.is_some());
    assert_eq!(cmd.unwrap().name, "model");
}

#[test]
fn registry_get_provider_alias() {
    let state = AppState::default();
    let provider = state.registry.get("provider");
    let providers = state.registry.get("providers");
    assert!(provider.is_some());
    assert_eq!(provider.unwrap().name, "provider");
    assert_eq!(providers.unwrap().name, "provider");
}

#[test]
fn registry_contains_auto_command() {
    let state = AppState::default();
    let cmd = state
        .registry
        .get("auto")
        .expect("/auto command should be registered");
    assert_eq!(cmd.name, "auto");
    // Auto-approve is a Safety command, like /readonly.
    assert_eq!(cmd.category, crate::commands::CommandCategory::Safety);
}

#[test]
fn registry_does_not_include_clone() {
    let state = AppState::default();
    assert!(
        state.registry.get("clone").is_none(),
        "/clone should be removed"
    );
}

#[test]
fn registry_does_not_include_changelog() {
    let state = AppState::default();
    assert!(
        state.registry.get("changelog").is_none(),
        "/changelog should be removed"
    );
}

#[test]
fn registry_list_returns_all() {
    let state = AppState::default();
    let defs = state.registry.list();
    assert!(
        defs.len() >= 18,
        "registry should have 18+ commands, got {}",
        defs.len()
    );
}

#[test]
fn registry_list_groups_by_category() {
    let state = AppState::default();
    let groups = state.registry.list_by_category();
    assert!(!groups.is_empty());
    let total: usize = groups.iter().map(|g| g.1.len()).sum();
    assert!(total >= 18);
}
