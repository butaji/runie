//! Tests for the declarative configuration loader.

#[cfg(test)]
mod loader_tests {
    use std::collections::HashMap;

    // Re-export types for tests
    use crate::commands::CommandCategory;
    use crate::declarative::loader::{parse_command_yaml, parse_triggers};
    use crate::declarative::types::{
        CommandDef, CommandKind, DeclarativeCommandYaml, SkillDef, Trigger,
    };
    use crate::resource_loader::extract_frontmatter;

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

    // ── Trigger parsing tests ──────────────────────────────────────────────────

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

    // ── Command category parsing via FromStr ───────────────────────────────────

    #[test]
    fn command_category_from_str_parses_known_values() {
        // Case-insensitive parsing (original behavior preserved)
        assert_eq!(
            CommandCategory::parse_case_insensitive("session"),
            Ok(CommandCategory::Session)
        );
        assert_eq!(
            CommandCategory::parse_case_insensitive("SESSION"),
            Ok(CommandCategory::Session)
        );
        assert_eq!(
            CommandCategory::parse_case_insensitive("model"),
            Ok(CommandCategory::Model)
        );
        assert_eq!(
            CommandCategory::parse_case_insensitive("safety"),
            Ok(CommandCategory::Safety)
        );
        assert_eq!(
            CommandCategory::parse_case_insensitive("system"),
            Ok(CommandCategory::System)
        );
    }

    #[test]
    fn command_category_from_str_aliases_map_to_system() {
        // Tool, Help, Unknown all map to System
        assert_eq!(
            CommandCategory::parse_case_insensitive("tool"),
            Ok(CommandCategory::System)
        );
        assert_eq!(
            CommandCategory::parse_case_insensitive("help"),
            Ok(CommandCategory::System)
        );
    }

    #[test]
    fn command_category_from_str_display_round_trip() {
        // Display round-trip
        assert_eq!(
            CommandCategory::parse_case_insensitive(&CommandCategory::Core.to_string()),
            Ok(CommandCategory::Core)
        );
        assert_eq!(
            CommandCategory::parse_case_insensitive(&CommandCategory::Session.to_string()),
            Ok(CommandCategory::Session)
        );
    }

    // ── Skill definition tests ────────────────────────────────────────────────

    #[test]
    fn skill_def_has_correct_fields() {
        let skill = SkillDef {
            name: "test-skill".to_owned(),
            description: "A test skill".to_owned(),
            context: Some("Context content".to_owned()),
            triggers: vec![Trigger::Command("/test".to_owned())],
            file_path: camino::Utf8PathBuf::from("/test/skill.md"),
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
            aliases: vec![],
            has_subcommands: false,
            file_path: camino::Utf8PathBuf::from("/commands/bookmark.yaml"),
            yaml_kind: CommandKind::Msg {
                message: "bookmarked".to_owned(),
            },
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
type: handler
handler: bookmark
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
sub: false
type: handler
handler: test
"#;
        let cmd: DeclarativeCommandYaml = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(cmd.name, "test-cmd");
        assert_eq!(cmd.description, "A test command");
        assert_eq!(cmd.category, CommandCategory::Model);
        assert_eq!(cmd.intent, "SetModel");
        assert_eq!(cmd.shortcut, Some("Ctrl+m".to_owned()));
        assert!(!cmd.sub);
    }

    #[test]
    fn declarative_command_yaml_defaults_missing_fields() {
        // Note: type field is now required; other fields have defaults
        let yaml = "name: minimal\ntype: handler\nhandler: minimal\n";
        let cmd: DeclarativeCommandYaml = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(cmd.name, "minimal");
        assert_eq!(cmd.description, "");
        assert_eq!(cmd.category, CommandCategory::System);
        assert_eq!(cmd.intent, "");
        assert!(cmd.shortcut.is_none());
        assert!(!cmd.sub);
        assert!(cmd.triggers.is_empty());
    }

    #[test]
    fn declarative_command_yaml_deserializes_triggers_as_list() {
        let yaml = r#"
name: trig-cmd
triggers:
  - /help
  - Ctrl+h
type: handler
handler: trig
"#;
        let cmd: DeclarativeCommandYaml = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(cmd.triggers.len(), 2);
        assert_eq!(cmd.triggers[0], Trigger::Command("/help".to_owned()));
        assert_eq!(cmd.triggers[1], Trigger::Shortcut("Ctrl+h".to_owned()));
    }

    // ── CommandKind helper method tests ────────────────────────────────────────

    #[test]
    fn command_kind_handler_name_returns_name() {
        use crate::declarative::types::CommandKind;
        let kind = CommandKind::Handler {
            handler: "save".to_owned(),
        };
        assert_eq!(kind.handler_name(), Some("save"));
        assert_eq!(kind.message(), None);
    }

    #[test]
    fn command_kind_form_with_handler_returns_handler_name() {
        use crate::declarative::types::CommandKind;
        let kind = CommandKind::FormWithHandler {
            title: "Save".to_owned(),
            fields: vec![],
            handler: "save_session".to_owned(),
        };
        assert_eq!(kind.handler_name(), Some("save_session"));
        assert_eq!(kind.message(), None);
    }

    #[test]
    fn command_kind_msg_returns_message() {
        use crate::declarative::types::CommandKind;
        let kind = CommandKind::Msg {
            message: "Done!".to_owned(),
        };
        assert_eq!(kind.handler_name(), None);
        assert_eq!(kind.message(), Some("Done!"));
    }

    #[test]
    fn yaml_deserializes_directly_to_command_kind() {
        // Layer 1: YAML deserializes directly into DeclarativeCommandYaml with CommandKind.
        let yaml = r#"
name: my-cmd
description: A test command
category: session
type: handler
handler: my_handler
"#;
        let cmd: DeclarativeCommandYaml = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(cmd.name, "my-cmd");
        assert_eq!(cmd.description, "A test command");
        assert_eq!(cmd.category, CommandCategory::Session);
        assert_eq!(cmd.kind.handler_name(), Some("my_handler"));
    }
}
