use serde::{Deserialize, Serialize};

use super::HarnessSkill;

/// Configuration for the tool schema enricher skill.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ToolSchemaEnricherConfig {
    /// Whether enrichment is enabled. Defaults to true.
    #[serde(default = "super::default_true")]
    pub enabled: bool,
    /// Tools to skip enrichment for (empty = enrich all).
    #[serde(default)]
    pub skip_tools: Vec<String>,
}

impl Default for ToolSchemaEnricherConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            skip_tools: Vec::new(),
        }
    }
}

/// Tool schema enricher skill: adds examples to tool schemas.
pub struct ToolSchemaEnricherSkill {
    config: ToolSchemaEnricherConfig,
}

impl ToolSchemaEnricherSkill {
    pub fn new(config: ToolSchemaEnricherConfig) -> Self {
        Self { config }
    }

    /// Get example inputs for a tool.
    pub(crate) fn get_examples(tool_name: &str) -> Vec<serde_json::Value> {
        match tool_name {
            "bash" => vec![
                serde_json::json!({"command": "ls"}),
                serde_json::json!({"command": "cargo test"}),
                serde_json::json!({"command": "git status"}),
            ],
            "read_file" => vec![
                serde_json::json!({"path": "src/main.rs"}),
                serde_json::json!({"path": "README.md"}),
            ],
            "write_file" => vec![serde_json::json!({"path": "f.txt", "content": "hi"})],
            "edit_file" => vec![serde_json::json!({"path": "f.rs", "search": "a", "replace": "b"})],
            "list_dir" => vec![serde_json::json!({"path": "."})],
            "grep" => vec![serde_json::json!({"pattern": "TODO", "path": "."})],
            "find" => vec![serde_json::json!({"pattern": "*.rs", "path": "."})],
            "fetch_docs" => vec![serde_json::json!({"library": "serde"})],
            _ => Vec::new(),
        }
    }

    /// Check if a tool should be enriched (not in skip list).
    pub(crate) fn should_enrich(&self, tool_name: &str) -> bool {
        !self.config.skip_tools.contains(&tool_name.to_owned())
    }

    /// Enrich a tool schema with examples.
    pub(crate) fn enrich_schema(&self, schema: &serde_json::Value) -> serde_json::Value {
        let name = schema.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let examples = Self::get_examples(name);
        if examples.is_empty() || !self.should_enrich(name) {
            return schema.clone();
        }
        let mut enriched = schema.clone();
        if let Some(obj) = enriched
            .get_mut("input_schema")
            .and_then(|v| v.as_object_mut())
        {
            obj.insert("examples".to_owned(), serde_json::json!(examples));
        }
        enriched
    }

    /// Enrich a list of tool schemas with examples.
    pub fn enrich_schemas(&self, schemas: Vec<serde_json::Value>) -> Vec<serde_json::Value> {
        if !self.config.enabled {
            return schemas;
        }
        schemas
            .into_iter()
            .map(|s| self.enrich_schema(&s))
            .collect()
    }
}

impl HarnessSkill for ToolSchemaEnricherSkill {
    fn name(&self) -> &str {
        "tool_schema_enricher"
    }
}
