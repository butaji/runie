//! Tool call representation.

use serde::{Deserialize, Serialize};

use super::parts::Part;

/// A first-class tool invocation carried by an assistant message.
///
/// `args` is the structured argument object. Use `arguments_string()` to get
/// the JSON-encoded wire format expected by OpenAI-compatible APIs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    /// Structured arguments. Serializes to JSON string for OpenAI wire format.
    #[serde(default)]
    pub args: serde_json::Value,
}

impl ToolCall {
    pub fn new(id: impl Into<String>, name: impl Into<String>, args: serde_json::Value) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            args,
        }
    }

    /// Construct a ToolCall from a JSON string for the arguments field.
    pub fn with_json_args(
        id: impl Into<String>,
        name: impl Into<String>,
        arguments: impl AsRef<str>,
    ) -> Self {
        let args: serde_json::Value =
            serde_json::from_str(arguments.as_ref()).unwrap_or(serde_json::Value::Null);
        Self::new(id, name, args)
    }

    /// Serialize arguments to a JSON string for the OpenAI wire format.
    pub fn arguments_string(&self) -> String {
        serde_json::to_string(&self.args).unwrap_or_else(|_| "{}".to_owned())
    }

    /// Convert a `Part::ToolCall` into a `ToolCall`; other part variants become empty.
    pub fn from_part(part: Part) -> Self {
        match part {
            Part::ToolCall { id, name, args } => Self { id, name, args },
            _ => Self::new(String::new(), String::new(), serde_json::Value::Null),
        }
    }
}
