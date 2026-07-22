//! Skills priority and scope tests.
//!
//! Tests the 6-scope priority system and skill disable/enable functionality.

use runie_core::skills::{Skill, SkillScope};
use std::collections::HashSet;

#[test]
fn skill_scope_ordering() {
    // Local < Repo < User < Config < Server < Bundled
    assert!(SkillScope::Local < SkillScope::Repo);
    assert!(SkillScope::Repo < SkillScope::User);
    assert!(SkillScope::User < SkillScope::Config);
    assert!(SkillScope::Config < SkillScope::Server);
    assert!(SkillScope::Server < SkillScope::Bundled);
}

#[test]
fn skill_scope_display() {
    assert_eq!(SkillScope::Local.to_string(), "Local");
    assert_eq!(SkillScope::Repo.to_string(), "Repo");
    assert_eq!(SkillScope::User.to_string(), "User");
    assert_eq!(SkillScope::Config.to_string(), "Config");
    assert_eq!(SkillScope::Server.to_string(), "Server");
    assert_eq!(SkillScope::Bundled.to_string(), "Bundled");
}

#[test]
fn skill_qualified_name_bare() {
    let skill = Skill {
        name: "my-skill".into(),
        description: "Test skill".into(),
        context: "".into(),
        user_invocable: false,
        file_path: camino::Utf8PathBuf::from("/path/to/SKILL.md"),
        scope: SkillScope::User,
        enabled: true,
        plugin_name: None,
        ignore_paths: vec![],
    };
    assert_eq!(skill.qualified_name(), "my-skill");
}

#[test]
fn skill_qualified_name_with_plugin() {
    let skill = Skill {
        name: "hello".into(),
        description: "Plugin skill".into(),
        context: "".into(),
        user_invocable: false,
        file_path: camino::Utf8PathBuf::from("/path/to/SKILL.md"),
        scope: SkillScope::Repo,
        enabled: true,
        plugin_name: Some("my-plugin".into()),
        ignore_paths: vec![],
    };
    assert_eq!(skill.qualified_name(), "my-plugin/hello");
}

#[test]
fn skill_summary_includes_scope() {
    let skill = Skill {
        name: "test-skill".into(),
        description: "A test skill".into(),
        context: "".into(),
        user_invocable: false,
        file_path: camino::Utf8PathBuf::from("/path/to/SKILL.md"),
        scope: SkillScope::Local,
        enabled: true,
        plugin_name: None,
        ignore_paths: vec![],
    };
    let summary = skill.summary();
    assert!(summary.contains("test-skill"));
    assert!(summary.contains("Local"));
    assert!(summary.contains("A test skill"));
}

#[test]
fn skill_summary_includes_invocable_tag() {
    let skill = Skill {
        name: "invocable-skill".into(),
        description: "User can invoke".into(),
        context: "".into(),
        user_invocable: true,
        file_path: camino::Utf8PathBuf::from("/path/to/SKILL.md"),
        scope: SkillScope::User,
        enabled: true,
        plugin_name: None,
        ignore_paths: vec![],
    };
    let summary = skill.summary();
    assert!(summary.contains("invocable"));
}

#[test]
fn deduplicate_skills_keeps_highest_priority() {
    let skill_local = Skill {
        name: "duplicate".into(),
        description: "Local version".into(),
        context: "".into(),
        user_invocable: false,
        file_path: camino::Utf8PathBuf::from("/local/SKILL.md"),
        scope: SkillScope::Local,
        enabled: true,
        plugin_name: None,
        ignore_paths: vec![],
    };
    let skill_user = Skill {
        name: "duplicate".into(),
        description: "User version".into(),
        context: "".into(),
        user_invocable: false,
        file_path: camino::Utf8PathBuf::from("/user/SKILL.md"),
        scope: SkillScope::User,
        enabled: true,
        plugin_name: None,
        ignore_paths: vec![],
    };
    let skill_bundled = Skill {
        name: "duplicate".into(),
        description: "Bundled version".into(),
        context: "".into(),
        user_invocable: false,
        file_path: camino::Utf8PathBuf::from("/bundled/SKILL.md"),
        scope: SkillScope::Bundled,
        enabled: true,
        plugin_name: None,
        ignore_paths: vec![],
    };

    // Test priority: Local > Repo > User > Config > Server > Bundled
    let mut skills = vec![skill_bundled.clone(), skill_user.clone(), skill_local.clone()];
    skills.sort_by_key(|s| s.scope);
    let mut seen = HashSet::new();
    skills.retain(|s| seen.insert(s.name.clone()));

    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].scope, SkillScope::Local);
    assert_eq!(skills[0].description, "Local version");
}

#[test]
fn skills_config_default() {
    use runie_core::config::SkillsConfig;

    let config = SkillsConfig::default();
    assert!(config.paths.is_empty());
    assert!(config.ignore.is_empty());
    assert!(config.disabled.is_empty());
}

#[test]
fn skills_config_with_values() {
    use runie_core::config::SkillsConfig;

    let config = SkillsConfig {
        paths: vec!["~/custom/skills".into()],
        ignore: vec!["/tmp/skills".into()],
        disabled: vec!["broken-skill".into()],
    };

    assert_eq!(config.paths.len(), 1);
    assert_eq!(config.ignore.len(), 1);
    assert_eq!(config.disabled.len(), 1);
    assert!(config.disabled.contains(&"broken-skill".into()));
}
