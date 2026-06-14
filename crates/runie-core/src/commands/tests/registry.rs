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
fn registry_get_providers_alias() {
    let state = AppState::default();
    let providers = state.registry.get("providers");
    let provider = state.registry.get("provider");
    assert!(providers.is_some());
    assert_eq!(providers.unwrap().name, "providers");
    assert_eq!(provider.unwrap().name, "providers");
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
        defs.len() >= 22,
        "registry should have 22+ commands, got {}",
        defs.len()
    );
}

#[test]
fn registry_list_groups_by_category() {
    let state = AppState::default();
    let groups = state.registry.list_by_category();
    assert!(!groups.is_empty());
    let total: usize = groups.iter().map(|g| g.1.len()).sum();
    assert!(total >= 22);
}
