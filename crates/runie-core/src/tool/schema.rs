//! Schema-based tool definitions using rmcp and schemars.
//!
//! This module provides typed tool definitions derived from Rust structs using
//! schemars for JSON schema generation. Tools are defined by implementing the
//! [`ToolDef`] trait with typed input/output structs.
//!
//! All tools are MCP tools. The [`ToolDef`] trait is the single interface for
//! tool definitions, schema generation, and execution.
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
//!     
//!     async fn execute(input: Self::Input, ctx: &ToolContext) -> ToolOutput {
//!         // implementation
//!     }
//! }

use std::sync::Arc;

use rmcp::model::{Tool, ToolAnnotations};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::context::{ToolContext, ToolOutput};

/// A typed tool definition.
///
/// This is the single interface for tool definitions, schema generation, and execution.
/// All tools implement this trait; there is no separate `Tool` trait.
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

    /// Execute the tool with typed input and context.
    #[allow(async_fn_in_trait)]
    async fn execute(input: Self::Input, ctx: &ToolContext) -> ToolOutput;
}

/// Generate JSON schema for a type implementing `JsonSchema`.
pub fn generate_schema<T: JsonSchema>() -> Value {
    serde_json::to_value(schemars::schema_for!(T)).unwrap_or_default()
}

/// Supported tool format output targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolFormat {
    /// MCP (Model Context Protocol) format.
    Mcp,
    /// OpenAI function calling format.
    OpenAi,
    /// Anthropic tool format (no type wrapper, uses input_schema).
    Anthropic,
    /// Google Gemini function declaration format.
    Gemini,
}

impl ToolFormat {
    /// Convert a `ToolDef` implementation to the target format.
    pub fn convert<T: ToolDef>(&self) -> Value {
        match self {
            ToolFormat::Mcp => to_mcp_tool::<T>().into(),
            ToolFormat::OpenAi => to_openai_function::<T>(),
            ToolFormat::Anthropic => to_anthropic_tool::<T>(),
            ToolFormat::Gemini => to_gemini_tool::<T>(),
        }
    }
}

/// Generate MCP tool definition from a `ToolDef` implementation.
pub fn to_mcp_tool<T: ToolDef>() -> Tool {
    let schema = generate_schema::<T::Input>();
    let input_schema = Arc::new(schema.as_object().cloned().unwrap_or_default());

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

/// Generate Anthropic tool format.
///
/// Unlike OpenAI, this uses `input_schema` directly without the `type` and
/// `function` wrapper. The output is a bare tool object with `name`,
/// `description`, and `input_schema` fields.
pub fn to_anthropic_tool<T: ToolDef>() -> Value {
    let schema = generate_schema::<T::Input>();
    serde_json::json!({
        "name": T::NAME,
        "description": T::DESCRIPTION,
        "input_schema": schema,
    })
}

/// Generate Google Gemini function declaration format.
///
/// The output is a `function_declarations` array containing the tool definition
/// with `name`, `description`, and `parameters` fields at the top level.
pub fn to_gemini_tool<T: ToolDef>() -> Value {
    let schema = generate_schema::<T::Input>();
    serde_json::json!({
        "function_declarations": [{
            "name": T::NAME,
            "description": T::DESCRIPTION,
            "parameters": schema,
        }]
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

    #[test]
    fn openai_function_has_correct_structure() {
        let result = to_openai_function::<TestInput>();
        assert!(result.get("type").is_some(), "should have 'type' field");
        assert!(result.get("function").is_some(), "should have 'function' wrapper");
        let func = result.get("function").unwrap().as_object().unwrap();
        assert_eq!(func.get("name").and_then(|v| v.as_str()), Some(""));
        assert!(func.get("description").is_some());
        assert!(func.get("parameters").is_some());
    }

    #[test]
    fn anthropic_tool_has_correct_structure() {
        let result = to_anthropic_tool::<TestInput>();
        // Anthropic format has no 'type' or 'function' wrapper
        assert!(result.get("type").is_none(), "should NOT have 'type' field");
        assert!(result.get("function").is_none(), "should NOT have 'function' wrapper");
        assert!(result.get("name").is_some(), "should have 'name' field");
        assert!(result.get("description").is_some(), "should have 'description' field");
        assert!(result.get("input_schema").is_some(), "should have 'input_schema' field");
        assert!(result.get("parameters").is_none(), "should NOT have 'parameters' field");
    }

    #[test]
    fn gemini_tool_has_correct_structure() {
        let result = to_gemini_tool::<TestInput>();
        // Gemini format wraps in function_declarations array
        assert!(result.get("function_declarations").is_some(), "should have 'function_declarations' field");
        let decls = result.get("function_declarations").unwrap().as_array().unwrap();
        assert_eq!(decls.len(), 1);
        let decl = &decls[0];
        assert!(decl.get("name").is_some());
        assert!(decl.get("description").is_some());
        assert!(decl.get("parameters").is_some(), "should have 'parameters' field");
    }

    #[test]
    fn tool_format_convert_mcp() {
        let result = ToolFormat::Mcp.convert::<TestInput>();
        let obj = result.as_object().unwrap();
        assert!(obj.contains_key("name"));
        assert!(obj.contains_key("description"));
    }

    #[test]
    fn tool_format_convert_openai() {
        let result = ToolFormat::OpenAi.convert::<TestInput>();
        assert!(result.get("type").is_some());
        assert!(result.get("function").is_some());
    }

    #[test]
    fn tool_format_convert_anthropic() {
        let result = ToolFormat::Anthropic.convert::<TestInput>();
        assert!(result.get("type").is_none());
        assert!(result.get("name").is_some());
        assert!(result.get("input_schema").is_some());
    }

    #[test]
    fn tool_format_convert_gemini() {
        let result = ToolFormat::Gemini.convert::<TestInput>();
        assert!(result.get("function_declarations").is_some());
    }

    #[test]
    fn tool_format_is_copy() {
        use std::mem;
        let fmt = ToolFormat::Anthropic;
        let size = mem::size_of_val(&fmt);
        assert_eq!(size, 1, "ToolFormat should be 1 byte (copyable enum)");
    }
}
