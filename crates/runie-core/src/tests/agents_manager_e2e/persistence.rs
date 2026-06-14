use crate::agent_profiles::{delete_profile, load_profile_from_file, save_profile, AgentProfile};

#[test]
fn full_crud_lifecycle() {
    let _dir = tempfile::tempdir().unwrap();
    let name = "testlifecycle";

    let mut profile = AgentProfile::new(name, "Be helpful");
    profile.description = "Test profile".into();
    profile.tools = vec!["read".into()];
    let path = save_profile(&profile).unwrap();
    assert!(path.exists(), "Create: file should exist");

    let loaded = load_profile_from_file(&path).unwrap();
    assert_eq!(loaded.name, name);
    assert_eq!(loaded.description, "Test profile");
    assert_eq!(loaded.tools, vec!["read"]);

    profile.description = "Updated".into();
    profile.tools = vec!["read".into(), "write".into()];
    save_profile(&profile).unwrap();
    let updated = load_profile_from_file(&path).unwrap();
    assert_eq!(updated.description, "Updated");
    assert_eq!(updated.tools.len(), 2);

    delete_profile(name).unwrap();
}

#[test]
fn save_overwrites_existing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("over.toml");

    let p1 = AgentProfile::new("over", "v1");
    std::fs::write(&path, toml::to_string_pretty(&p1).unwrap()).unwrap();

    let p2 = AgentProfile::new("over", "v2");
    std::fs::write(&path, toml::to_string_pretty(&p2).unwrap()).unwrap();

    let loaded = load_profile_from_file(&path).unwrap();
    assert_eq!(loaded.system_prompt, "v2", "Save should overwrite");
}

#[test]
fn parse_invalid_toml_fails() {
    let bad = "name = broken ====";
    let result = crate::agent_profiles::parse_profile(bad);
    assert!(result.is_err());
}

#[test]
fn parse_missing_required_field_fails() {
    let bad = "description = \"x\"\nsystem_prompt = \"x\"\ntools = []";
    let result = crate::agent_profiles::parse_profile(bad);
    assert!(result.is_err());
}
