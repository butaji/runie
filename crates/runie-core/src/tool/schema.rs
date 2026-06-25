//! Schema-based tool definitions using rmcp and schemars.
//!
//! This module provides typed tool definitions derived from Rust structs using
//! schemars for JSON schema generation. Tools are defined by implementing the
//! [`ToolDef`] trait with typed input/output structs.
//!
//! # Example
//!
//! ```ignore
//! use schemars::JsonSchema;
//! use serde::Deserialize;
//!
//! #[derive(Debug, Deserialize, JsonSchema)]
//! struct ReadFileInput {
//!     path: String,
//!     #[serde(default)]
//!     offset: Option<u64>,
//!     #[serde(default)]
//!     limit: Option<u64>,
//! }
//!
//! impl ToolDef for ReadFileTool {
//!     type Input = ReadFileInput;
//!     const NAME: &'static str = "read_file";
//!     const DESCRIPTION: &'static str = "Read file contents";
//!     const READ_ONLY: bool = true;
//!     const REQUIRES_APPROVAL: bool = false;
//! }

use std::sync::Arc;

use rmcp::model::{Tool, ToolAnnotations};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde_json::Value;

/// A typed tool definition.
///
/// Implement this trait to define a tool with typed input using schemars
/// for automatic JSON schema generation.
pub trait ToolDef: Send + Sync {
    /// The input parameters type, must implement `Deserialize` and `JsonSchema`.
    type Input: DeserializeOwned + JsonSchema + Send + Sync + 'static;

    /// Tool name (snake_case).
    const NAME: &'static str;

    /// Human-readable description.
    const DESCRIPTION: &'static str;

    /// Whether this tool is read-only (no side effects).
    const READ_ONLY: bool = false;

    /// Whether this tool requires explicit approval.
    const REQUIRES_APPROVAL: bool = true;

    /// Execute the tool with typed input.
    fn call(input: Self::Input) -> impl std::future::Future<Output = Result<ToolResult, anyhow::Error>> + Send
    where
        Self: Sized;
}

/// Result of a tool execution.
#[derive(Debug, Clone)]
pub struct ToolResult {
    pub content: String,
    pub bytes_transferred: Option<u64>,
}

/// Generate JSON schema for a type implementing `JsonSchema`.
pub fn generate_schema<T: JsonSchema>() -> Value {
    serde_json::to_value(&schemars::schema_for!(T)).unwrap_or_default()
}

/// Generate MCP tool definition from a `ToolDef` implementation.
pub fn to_mcp_tool<T: ToolDef>() -> Tool {
    let schema = generate_schema::<T::Input>();
    let input_schema = Arc::new(
        schema
            .as_object()
            .cloned()
            .unwrap_or_default(),
    );

    let mut tool = Tool::new(T::NAME, T::DESCRIPTION, input_schema);

    // Add annotations for read-only and approval hints
    let mut annotations = ToolAnnotations::new();
    annotations.read_only_hint = Some(T::READ_ONLY);
    // Note: MCP doesn't have a direct "requires_approval" annotation,
    // so we handle this at the permission gate layer
    tool.annotations = Some(annotations);

    tool
}

/// Generate OpenAI function definition format.
pub fn to_openai_function<T: ToolDef>() -> Value {
    let schema = generate_schema::<T::Input>();
    serde_json::json!({
        "type": "function",
        "function": {
            "name": T::NAME,
            "description": T::DESCRIPTION,
            "parameters": schema,
        }
    })
}

/// Parse JSON input into typed input.
pub fn parse_input<T: DeserializeOwned>(input: &Value) -> Result<T, serde_json::Error> {
    serde_json::from_value(input.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use schemars::JsonSchema;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, JsonSchema)]
    struct TestInput {
        path: String,
        #[serde(default)]
        limit: Option<u64>,
    }

    #[test]
    fn schema_generation_produces_valid_schema() {
        let schema = generate_schema::<TestInput>();
        assert!(schema.is_object());
        let obj = schema.as_object().unwrap();
        assert!(obj.contains_key("type") || obj.contains_key("$ref"));
    }

    #[test]
    fn schema_includes_path_property() {
        let schema = generate_schema::<TestInput>();
        let obj = schema.as_object().unwrap();
        // Schema should contain properties
        if let Some(properties) = obj.get("properties") {
            assert!(properties.get("path").is_some());
        }
    }

    #[test]
    fn parse_input_round_trips() {
        let input = serde_json::json!({
            "path": "/tmp/test.txt",
            "limit": 100
        });
        let parsed: TestInput = parse_input(&input).unwrap();
        assert_eq!(parsed.path, "/tmp/test.txt");
        assert_eq!(parsed.limit, Some(100));
    }

    #[test]
    fn parse_input_with_defaults() {
        let input = serde_json::json!({
            "path": "/tmp/test.txt"
        });
        let parsed: TestInput = parse_input(&input).unwrap();
        assert_eq!(parsed.path, "/tmp/test.txt");
        assert_eq!(parsed.limit, None);
    }
}
