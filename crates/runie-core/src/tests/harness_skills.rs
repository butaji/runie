//! Tests for harness_skills module.

use crate::harness_skills::{
    HarnessConfig, HarnessSkill, HashlineEdit, HashlineEditConfig, HashlineEditSkill, LoopDetectorConfig,
    LoopDetectorSkill, SkillConfig, SkillRegistry, StartupContextConfig, StartupContextSkill, ToolSchemaEnricherConfig,
    ToolSchemaEnricherSkill, TurnEndCtx, TurnEndResult, TurnStartCtx, TurnStartResult, VerificationConfig,
    VerificationLoopSkill,
};

#[test]
fn skill_registry_loads_defaults() {
    let registry = SkillRegistry::new();
    // Empty registry has no enabled skills
    assert!(registry.enabled_skills().is_empty());
}

#[test]
fn skill_registry_respects_disabled_flag() {
    let mut registry = SkillRegistry::new();
    registry.register(StartupContextSkill::new(StartupContextConfig::default()));

    // Enabled by default
    assert!(registry.enabled_skills().contains(&"startup_context"));

    // Disable in config
    let mut config = HarnessConfig::default();
    config.skills.insert(
        "startup_context".into(),
        SkillConfig { enabled: false, ..Default::default() },
    );
    registry.set_config(config);
    assert!(registry.enabled_skills().is_empty());
}

#[test]
fn on_turn_start_all_continue() {
    let mut registry = SkillRegistry::new();
    registry.register(StartupContextSkill::new(StartupContextConfig {
        enabled: false,
        ..Default::default()
    }));
    registry.register(LoopDetectorSkill::new(LoopDetectorConfig::default()));

    let ctx =
        TurnStartCtx { message: "Hello".into(), system_prompt: "You are helpful".into(), skills_context: "".into() };

    let result = registry.on_turn_start(&ctx);
    assert!(matches!(result, TurnStartResult::Continue));
}

#[test]
fn on_turn_start_first_abort_wins() {
    struct AbortSkill;
    impl HarnessSkill for AbortSkill {
        fn name(&self) -> &str {
            "abort"
        }
        fn on_turn_start(&self, _: &TurnStartCtx) -> TurnStartResult {
            TurnStartResult::Abort("stopped".into())
        }
    }

    struct ContinueSkill;
    impl HarnessSkill for ContinueSkill {
        fn name(&self) -> &str {
            "continue"
        }
    }

    let mut registry = SkillRegistry::new();
    registry.register(AbortSkill);
    registry.register(ContinueSkill);

    let ctx = TurnStartCtx { message: "Hello".into(), system_prompt: "".into(), skills_context: "".into() };

    let result = registry.on_turn_start(&ctx);
    match result {
        TurnStartResult::Abort(msg) => assert_eq!(msg, "stopped"),
        _ => panic!("expected Abort"),
    }
}

#[test]
fn config_deserializes_from_toml() {
    let toml_str = r#"
[skills.startup_context]
enabled = true

[skills.loop_detector]
enabled = false
"#;
    let config: HarnessConfig = toml::from_str(toml_str).unwrap();
    assert!(config.skills.get("startup_context").unwrap().enabled);
    assert!(!config.skills.get("loop_detector").unwrap().enabled);
}

#[test]
fn verification_loop_needs_verification_code_edits() {
    assert!(VerificationLoopSkill::needs_verification(
        "Here is the fix: ```rust\nfn main() {}```"
    ));
    assert!(VerificationLoopSkill::needs_verification(
        "Updated the function in file.rs"
    ));
    assert!(VerificationLoopSkill::needs_verification(
        "class MyClass {}"
    ));
    assert!(VerificationLoopSkill::needs_verification(
        "const VALUE = 1;"
    ));
    assert!(VerificationLoopSkill::needs_verification("let x = 1;"));
    assert!(VerificationLoopSkill::needs_verification(
        "fn new_func() {}"
    ));
}

#[test]
fn verification_loop_no_verification_plain_text() {
    assert!(!VerificationLoopSkill::needs_verification(
        "Hello, how are you?"
    ));
    assert!(!VerificationLoopSkill::needs_verification(
        "I think we should do X"
    ));
    assert!(!VerificationLoopSkill::needs_verification(
        "Thanks for your question"
    ));
}

#[tokio::test]
async fn verification_loop_disabled_continues() {
    let skill = VerificationLoopSkill::new(VerificationConfig {
        enabled: false,
        command: Some("cargo test".into()),
        max_fix_passes: 3,
        ..Default::default()
    });

    let ctx = TurnEndCtx { assistant_message: "```rust\nfn main() {}\n```".into(), tool_call_count: 1, success: true };

    let result = skill.on_turn_end(&ctx).await;
    assert!(matches!(result, TurnEndResult::Continue));
}

#[tokio::test]
async fn verification_loop_no_command_continues() {
    let skill = VerificationLoopSkill::new(VerificationConfig {
        enabled: true,
        command: None,
        max_fix_passes: 3,
        ..Default::default()
    });

    let ctx = TurnEndCtx { assistant_message: "```rust\nfn main() {}\n```".into(), tool_call_count: 1, success: true };

    let result = skill.on_turn_end(&ctx).await;
    assert!(matches!(result, TurnEndResult::Continue));
}

#[test]
fn verification_config_deserializes() {
    let toml_str = r#"
command = "cargo test"
max_fix_passes = 5
enabled = true
"#;
    let config: VerificationConfig = toml::from_str(toml_str).unwrap();
    assert_eq!(config.command.as_deref(), Some("cargo test"));
    assert_eq!(config.max_fix_passes, 5);
    assert!(config.enabled);
}

// HashlineEditSkill tests

#[test]
fn hashline_compute_hash_is_deterministic() {
    let content = "fn main() {}";
    let hash1 = HashlineEditSkill::compute_hash(content, 6);
    let hash2 = HashlineEditSkill::compute_hash(content, 6);
    assert_eq!(hash1, hash2);
}

#[test]
fn hashline_different_content_different_hash() {
    let hash1 = HashlineEditSkill::compute_hash("fn main() {}", 6);
    let hash2 = HashlineEditSkill::compute_hash("fn other() {}", 6);
    assert_ne!(hash1, hash2);
}

#[test]
fn hashline_trims_trailing_whitespace() {
    let hash1 = HashlineEditSkill::compute_hash("fn main() {}", 6);
    let hash2 = HashlineEditSkill::compute_hash("fn main() {}   ", 6);
    assert_eq!(hash1, hash2);
}

#[test]
fn hashline_schema_is_valid_json() {
    let schema = HashlineEditSkill::hashline_schema();
    assert!(schema.get("type").is_some());
    assert!(schema.get("properties").is_some());
    let edits = schema.pointer("/properties/edits");
    assert!(edits.is_some());
}

#[test]
fn hashline_edit_deserializes() {
    let json = serde_json::json!({
        "line": 10,
        "hash": "abc123",
        "content": "new content"
    });
    let edit: HashlineEdit = serde_json::from_value(json).unwrap();
    assert_eq!(edit.line, 10);
    assert_eq!(edit.hash, "abc123");
    assert_eq!(edit.content, "new content");
}

#[test]
fn hashline_edit_serialize_round_trip() {
    let edit = HashlineEdit { line: 5, hash: "def456".to_string(), content: "test".to_string() };
    let json = serde_json::to_value(&edit).unwrap();
    let round_trip: HashlineEdit = serde_json::from_value(json).unwrap();
    assert_eq!(edit.line, round_trip.line);
    assert_eq!(edit.hash, round_trip.hash);
    assert_eq!(edit.content, round_trip.content);
}

#[test]
fn hashline_edit_config_default() {
    let config = HashlineEditConfig::default();
    assert!(config.enabled);
    assert_eq!(config.hash_length, 6);
}

#[test]
fn hashline_edit_config_deserializes() {
    let toml_str = r#"
enabled = true
hash_length = 8
"#;
    let config: HashlineEditConfig = toml::from_str(toml_str).unwrap();
    assert!(config.enabled);
    assert_eq!(config.hash_length, 8);
}

// ToolSchemaEnricherSkill tests

#[test]
fn schemas_contain_examples_when_enabled() {
    let skill = ToolSchemaEnricherSkill::new(ToolSchemaEnricherConfig::default());
    let schemas = vec![serde_json::json!({
        "name": "bash",
        "description": "Run a bash command",
        "input_schema": {"type": "object", "properties": {"command": {"type": "string"}}}
    })];
    // Verify get_examples returns content for bash
    let examples = ToolSchemaEnricherSkill::get_examples("bash");
    assert!(!examples.is_empty(), "bash should have examples");

    // Verify enrich_schemas adds examples
    let enriched = skill.enrich_schemas(schemas);
    let result = &enriched[0]["input_schema"]["examples"];
    assert!(
        result.is_array(),
        "expected examples array, got: {:?}",
        result
    );
    assert!(!result.as_array().unwrap().is_empty());
}

#[test]
fn schemas_unchanged_when_disabled() {
    let skill = ToolSchemaEnricherSkill::new(ToolSchemaEnricherConfig { enabled: false, ..Default::default() });
    let schemas = vec![serde_json::json!({
        "name": "bash",
        "description": "Run a bash command",
        "input_schema": {"type": "object"}
    })];
    let enriched = skill.enrich_schemas(schemas);
    assert!(enriched[0]["input_schema"]["examples"].is_null());
}

#[test]
fn unknown_tool_no_examples() {
    let skill = ToolSchemaEnricherSkill::new(ToolSchemaEnricherConfig::default());
    let schemas = vec![serde_json::json!({
        "name": "unknown_tool",
        "description": "Unknown",
        "input_schema": {"type": "object"}
    })];
    let enriched = skill.enrich_schemas(schemas);
    assert!(enriched[0]["input_schema"]["examples"].is_null());
}

#[test]
fn skip_tool_excludes_examples() {
    let skill =
        ToolSchemaEnricherSkill::new(ToolSchemaEnricherConfig { enabled: true, skip_tools: vec!["bash".to_string()] });
    let schemas = vec![
        serde_json::json!({"name": "bash", "input_schema": {"type": "object"}}),
        serde_json::json!({"name": "read_file", "input_schema": {"type": "object"}}),
    ];
    let enriched = skill.enrich_schemas(schemas);
    assert!(
        enriched[0]["input_schema"]["examples"].is_null(),
        "bash should be skipped"
    );
    assert!(
        enriched[1]["input_schema"]["examples"].is_array(),
        "read_file should have examples"
    );
}

// LoopDetectorSkill tests

#[test]
fn loop_detector_fires_on_repeated_failed_edit() {
    let skill = LoopDetectorSkill::new(LoopDetectorConfig::default());
    let input = serde_json::json!({"path": "src/main.rs", "search": "old", "replace": "new"});

    // Record 3 failed edits on the same file
    for _ in 0..3 {
        skill.record_call("edit_file", &input, false);
    }

    let msg = skill.check_loop();
    assert!(msg.is_some(), "should detect loop after 3 failed attempts");
    assert!(msg.unwrap().contains("Loop detected"));
}

#[test]
fn loop_detector_ignores_successful_repetition() {
    let skill = LoopDetectorSkill::new(LoopDetectorConfig::default());
    let input = serde_json::json!({"path": "README.md"});

    // Record 5 successful reads - should not trigger
    for _ in 0..5 {
        skill.record_call("read_file", &input, true);
    }

    let msg = skill.check_loop();
    assert!(
        msg.is_none(),
        "successful operations should not trigger detection"
    );
}

#[test]
fn loop_detector_disabled_returns_none() {
    let skill = LoopDetectorSkill::new(LoopDetectorConfig { enabled: false, max_repeats: 3 });
    let input = serde_json::json!({"path": "test.rs"});

    for _ in 0..5 {
        skill.record_call("edit_file", &input, false);
    }

    assert!(skill.check_loop().is_none());
}

#[test]
fn loop_detector_reset_clears_state() {
    let skill = LoopDetectorSkill::new(LoopDetectorConfig::default());
    let input = serde_json::json!({"path": "test.rs"});

    for _ in 0..3 {
        skill.record_call("edit_file", &input, false);
    }
    assert!(skill.check_loop().is_some());

    skill.reset();
    assert!(skill.check_loop().is_none());
}

#[test]
fn startup_context_disabled_returns_continue() {
    let skill = StartupContextSkill::new(StartupContextConfig { enabled: false, ..Default::default() });
    let ctx = TurnStartCtx { message: "test".into(), system_prompt: "sys".into(), skills_context: "".into() };
    assert!(matches!(
        skill.on_turn_start(&ctx),
        TurnStartResult::Continue
    ));
}

#[tokio::test]
async fn startup_context_injects_workspace_context() {
    let skill = StartupContextSkill::new(StartupContextConfig {
        enabled: true,
        max_output_bytes: 2048,
        commands: vec!["pwd".into()],
    });
    let ctx = TurnStartCtx { message: "hello".into(), system_prompt: "sys".into(), skills_context: "".into() };
    let result = skill.on_turn_start(&ctx);
    if let TurnStartResult::SkipWithMessage(msg) = result {
        assert!(msg.contains("=== Workspace Context ==="));
        assert!(msg.contains("pwd"));
    } else {
        panic!("Expected SkipWithMessage, got {:?}", result);
    }
}

#[tokio::test]
async fn startup_context_respects_max_output_bytes() {
    let skill = StartupContextSkill::new(StartupContextConfig {
        enabled: true,
        max_output_bytes: 50, // Very small
        commands: vec!["pwd".into()],
    });
    let ctx = skill.get_context();
    assert!(ctx.len() <= 50);
}
