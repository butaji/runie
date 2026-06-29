use serde::{Deserialize, Serialize};

use crate::tool::BUILTIN_TOOL_NAMES;
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

    /// Get example inputs for a tool from canonical examples map.
    /// Returns empty vec for unknown tools.
    fn get_canonical_examples(tool_name: &str) -> Vec<serde_json::Value> {
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
            "search" => vec![serde_json::json!({"query": "function name", "path": "."})],
            "find_definitions" => vec![serde_json::json!({"symbol": "fn main", "path": "."})],
            _ => Vec::new(),
        }
    }

    /// Get example inputs for a tool.
    /// Uses canonical examples from `BUILTIN_TOOL_NAMES`.
    pub(crate) fn get_examples(tool_name: &str) -> Vec<serde_json::Value> {
        if BUILTIN_TOOL_NAMES.contains(&tool_name) {
            Self::get_canonical_examples(tool_name)
        } else {
            Vec::new()
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

#[cfg(test)]
mod tests {
    use super::*;

    // Layer 1: all BUILTIN_TOOL_NAMES have examples.
    #[test]
    fn schema_enricher_covers_all_builtin_tools() {
        for tool_name in BUILTIN_TOOL_NAMES {
            let examples = ToolSchemaEnricherSkill::get_examples(tool_name);
            assert!(
                !examples.is_empty(),
                "tool '{}' should have examples",
                tool_name
            );
        }
    }

    // Layer 1: unknown tools return empty examples.
    #[test]
    fn schema_enricher_unknown_tool_returns_empty() {
        let examples = ToolSchemaEnricherSkill::get_examples("nonexistent_tool");
        assert!(examples.is_empty());
    }

    // Layer 1: enrich_schema adds examples for known tools.
    #[test]
    fn enrich_schema_adds_examples_for_known_tool() {
        let skill = ToolSchemaEnricherSkill::new(ToolSchemaEnricherConfig::default());
        let schema = serde_json::json!({
            "name": "bash",
            "input_schema": {
                "type": "object",
                "properties": {
                    "command": {"type": "string"}
                }
            }
        });
        let enriched = skill.enrich_schema(&schema);
        let examples = enriched
            .get("input_schema")
            .and_then(|v| v.get("examples"))
            .and_then(|v| v.as_array());
        assert!(
            examples.is_some() && !examples.unwrap().is_empty(),
            "enriched schema should have examples"
        );
    }

    // Layer 1: enrich_schema skips unknown tools.
    #[test]
    fn enrich_schema_skips_unknown_tool() {
        let skill = ToolSchemaEnricherSkill::new(ToolSchemaEnricherConfig::default());
        let schema = serde_json::json!({
            "name": "unknown_tool",
            "input_schema": {"type": "object"}
        });
        let enriched = skill.enrich_schema(&schema);
        assert!(!
            enriched
                .get("input_schema")
                .and_then(|v| v.get("examples"))
                .is_some());
    }
}
