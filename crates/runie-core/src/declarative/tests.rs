//! Tests for the declarative configuration loader.

#[cfg(test)]
mod loader_tests {
    use std::collections::HashMap;
    use std::path::Path;

    // Re-export types for tests
    use crate::declarative::types::{CommandDef, DeclarativeCommandYaml, SkillDef, Trigger};
    use crate::commands::CommandCategory;
    use crate::declarative::loader::{
        load_skills_from_dir, parse_command_yaml, parse_triggers,
    };
    use crate::resource_loader::{
        extract_frontmatter, parse_yaml_line, strip_quotes,
    };

    // ── Frontmatter parsing tests ───────────────────────────────────────────────

    #[test]
    fn frontmatter_parses_name_and_description() {
        let content = r#"---
name: my-skill
description: A test skill
---

# Content
"#;
        let fm = extract_frontmatter(content);
        assert_eq!(fm.get("name"), Some(&"my-skill".to_owned()));
        assert_eq!(fm.get("description"), Some(&"A test skill".to_owned()));
    }

    #[test]
    fn frontmatter_strips_quotes() {
        let content = r#"---
name: "quoted name"
description: 'single quoted'
---

Content
"#;
        let fm = extract_frontmatter(content);
        assert_eq!(fm.get("name"), Some(&"quoted name".to_owned()));
        assert_eq!(fm.get("description"), Some(&"single quoted".to_owned()));
    }

    #[test]
    fn frontmatter_handles_simple_context() {
        let content = r#"---
name: simple-context
context: This is a simple context value
---

Content
"#;
        let fm = extract_frontmatter(content);
        let ctx = fm.get("context").unwrap();
        assert!(ctx.contains("simple context"));
    }

    #[test]
    fn frontmatter_returns_none_without_delimiters() {
        let content = "# No frontmatter\n\nContent";
        assert!(extract_frontmatter(content).is_empty());
    }

    #[test]
    fn frontmatter_handles_empty_values() {
        let content = r#"---
name: test
empty: 
---

Content
"#;
        let fm = extract_frontmatter(content);
        assert_eq!(fm.get("name"), Some(&"test".to_owned()));
        // Empty values are parsed as "Null" by serde_yaml
        assert_eq!(fm.get("empty"), Some(&"Null".to_owned()));
    }

    #[test]
    fn frontmatter_ignores_comments() {
        let content = r#"---
name: test
# This is a comment
description: desc
---

Content
"#;
        let fm = extract_frontmatter(content);
        assert_eq!(fm.len(), 2);
        assert!(fm.contains_key("name"));
        assert!(fm.contains_key("description"));
    }

    // ── YAML line parsing tests ────────────────────────────────────────────────

    #[test]
    fn yaml_line_parses_key_value() {
        let (key, val) = parse_yaml_line("name: test").unwrap();
        assert_eq!(key, "name");
        assert_eq!(val, "test");
    }

    #[test]
    fn yaml_line_handles_leading_whitespace() {
        let (key, val) = parse_yaml_line("  name: test").unwrap();
        assert_eq!(key, "name");
    }

    #[test]
    fn yaml_line_ignores_empty_lines() {
        assert!(parse_yaml_line("").is_none());
        assert!(parse_yaml_line("   ").is_none());
    }

    #[test]
    fn yaml_line_ignores_comments() {
        assert!(parse_yaml_line("# comment").is_none());
        assert!(parse_yaml_line("  # indented comment").is_none());
    }

    #[test]
    fn yaml_line_handles_colons_in_values() {
        let (key, val) = parse_yaml_line("url: http://example.com").unwrap();
        assert_eq!(val, "http://example.com");
    }

    // ── Quote stripping tests ──────────────────────────────────────────────────

    #[test]
    fn strip_quotes_removes_double_quotes() {
        assert_eq!(strip_quotes("\"hello\""), "hello");
    }

    #[test]
    fn strip_quotes_removes_single_quotes() {
        assert_eq!(strip_quotes("'hello'"), "hello");
    }

    #[test]
    fn strip_quotes_preserves_unquoted() {
        assert_eq!(strip_quotes("hello"), "hello");
    }

    #[test]
    fn strip_quotes_handles_whitespace() {
        assert_eq!(strip_quotes("  \"hello\"  "), "hello");
    }

    // ── Trigger parsing tests ─────────────────────────────────────────────────

    #[test]
    fn triggers_parse_command() {
        let mut fm = HashMap::new();
        fm.insert("command".to_owned(), "/check-work".to_owned());
        let triggers = parse_triggers(&fm);
        assert_eq!(triggers, vec![Trigger::Command("/check-work".to_owned())]);
    }

    #[test]
    fn triggers_parse_command_list() {
        let mut fm = HashMap::new();
        fm.insert(
            "triggers".to_owned(),
            "- command: /check-work\n- command: /verify".to_owned(),
        );
        let triggers = parse_triggers(&fm);
        assert_eq!(triggers.len(), 2);
        assert_eq!(triggers[0], Trigger::Command("/check-work".to_owned()));
        assert_eq!(triggers[1], Trigger::Command("/verify".to_owned()));
    }

    #[test]
    fn triggers_parse_mixed_list() {
        let mut fm = HashMap::new();
        fm.insert(
            "triggers".to_owned(),
            "- command: /test\n- *.xlsx".to_owned(),
        );
        let triggers = parse_triggers(&fm);
        assert_eq!(triggers.len(), 2);
        assert!(matches!(triggers[0], Trigger::Command(_)));
        assert!(matches!(triggers[1], Trigger::FilePattern(_)));
    }

    #[test]
    fn triggers_parse_empty_when_no_triggers() {
        let fm = HashMap::new();
        let triggers = parse_triggers(&fm);
        assert!(triggers.is_empty());
    }

    // ── Command category parsing ───────────────────────────────────────────────

    #[test]
    fn command_category_parses_known_values() {
        assert_eq!(crate::declarative::types::parse_category("session"), Some(CommandCategory::Session));
        assert_eq!(crate::declarative::types::parse_category("Session"), Some(CommandCategory::Session));
        assert_eq!(crate::declarative::types::parse_category("MODEL"), Some(CommandCategory::Model));
        assert_eq!(crate::declarative::types::parse_category("system"), Some(CommandCategory::System));
    }

    #[test]
    fn command_category_defaults_to_system() {
        // Tool, Help, Unknown all map to System
        assert_eq!(crate::declarative::types::parse_category("unknown"), Some(CommandCategory::System));
        assert_eq!(crate::declarative::types::parse_category(""), Some(CommandCategory::System));
        assert_eq!(crate::declarative::types::parse_category("tool"), Some(CommandCategory::System));
        assert_eq!(crate::declarative::types::parse_category("help"), Some(CommandCategory::System));
    }

    #[test]
    fn command_category_returns_none_for_unknown() {
        assert_eq!(crate::declarative::types::parse_category("nonexistent"), None);
    }

    // ── Skill definition tests ────────────────────────────────────────────────

    #[test]
    fn skill_def_has_correct_fields() {
        let skill = SkillDef {
            name: "test-skill".to_owned(),
            description: "A test skill".to_owned(),
            context: Some("Context content".to_owned()),
            triggers: vec![Trigger::Command("/test".to_owned())],
            file_path: Path::new("/test/skill.md").to_path_buf(),
            user_invocable: true,
        };
        assert_eq!(skill.name, "test-skill");
        assert!(skill.user_invocable);
        assert_eq!(skill.triggers.len(), 1);
    }

    // ── Command definition tests ───────────────────────────────────────────────

    #[test]
    fn command_def_has_correct_fields() {
        let cmd = CommandDef {
            name: "bookmark".to_owned(),
            description: "Bookmark message".to_owned(),
            category: CommandCategory::Session,
            intent: "BookmarkMessage".to_owned(),
            shortcut: Some("Ctrl+b".to_owned()),
            has_subcommands: false,
            file_path: Path::new("/commands/bookmark.yaml").to_path_buf(),
        };
        assert_eq!(cmd.name, "bookmark");
        assert_eq!(cmd.category, CommandCategory::Session);
        assert!(cmd.shortcut.is_some());
    }

    // ── Layer 1: Integration tests ─────────────────────────────────────────────

    #[test]
    fn skill_md_parsing_integration() {
        let content = r#"---
name: check-work
description: Verify changes with a subagent.
context: This skill verifies code changes.
triggers:
  - command: /check-work
  - command: /verify
invocation: user can invoke this with /check-work
---

## Usage

`/check-work [focus area]`

## Steps

1. Spawn verifier
2. Read verdict
"#;
        let fm = extract_frontmatter(content);
        assert_eq!(fm.get("name"), Some(&"check-work".to_owned()));
        assert!(fm.get("context").unwrap().contains("verifies"));
    }

    // ── Layer 2: Declarative loader behavior ─────────────────────────────────

    #[test]
    fn loader_derives_name_from_path_stem() {
        use crate::declarative::loader::load_skills_from_dir;
        // Test that empty dir returns empty vec
        let temp_dir = tempfile::tempdir().unwrap();
        let skills = load_skills_from_dir(temp_dir.path());
        assert!(skills.is_empty());
    }

    #[test]
    fn loader_loads_command_yaml() {
        // Create a temp command YAML file
        let temp_dir = tempfile::tempdir().unwrap();
        let cmd_path = temp_dir.path().join("bookmark.yaml");
        std::fs::write(
            &cmd_path,
            r#"name: bookmark
description: Bookmark the current message
category: Session
intent: BookmarkMessage
shortcut: Ctrl+b
"#,
        )
        .unwrap();

        let cmd = parse_command_yaml(&cmd_path).unwrap();
        assert_eq!(cmd.name, "bookmark");
        assert_eq!(cmd.category, CommandCategory::Session);
        assert_eq!(cmd.intent, "BookmarkMessage");
        assert_eq!(cmd.shortcut, Some("Ctrl+b".to_owned()));
    }

    #[test]
    fn loader_handles_invalid_yaml_gracefully() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cmd_path = temp_dir.path().join("invalid.yaml");
        std::fs::write(&cmd_path, "invalid: yaml: content:").unwrap();

        let result = parse_command_yaml(&cmd_path);
        assert!(result.is_none());
    }

    #[test]
    fn declarative_command_yaml_deserializes() {
        let yaml = r#"
name: test-cmd
description: A test command
category: Model
intent: SetModel
shortcut: Ctrl+m
subcommands: false
"#;
        let cmd: DeclarativeCommandYaml = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(cmd.name, "test-cmd");
        assert_eq!(cmd.description, "A test command");
        assert_eq!(cmd.category, CommandCategory::Model);
        assert_eq!(cmd.intent, "SetModel");
        assert_eq!(cmd.shortcut, Some("Ctrl+m".to_owned()));
        assert!(!cmd.subcommands);
    }

    #[test]
    fn declarative_command_yaml_defaults_missing_fields() {
        let yaml = "name: minimal\n";
        let cmd: DeclarativeCommandYaml = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(cmd.name, "minimal");
        assert_eq!(cmd.description, "");
        assert_eq!(cmd.category, CommandCategory::System);
        assert_eq!(cmd.intent, "");
        assert!(cmd.shortcut.is_none());
        assert!(!cmd.subcommands);
        assert!(cmd.triggers.is_empty());
    }

    #[test]
    fn declarative_command_yaml_deserializes_triggers_as_list() {
        let yaml = r#"
name: trig-cmd
triggers:
  - /help
  - Ctrl+h
"#;
        let cmd: DeclarativeCommandYaml = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(cmd.triggers.len(), 2);
        assert_eq!(cmd.triggers[0], Trigger::Command("/help".to_owned()));
        assert_eq!(cmd.triggers[1], Trigger::Shortcut("Ctrl+h".to_owned()));
    }
}
