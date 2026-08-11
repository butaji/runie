//! Structured user questions. Presentation belongs to the TUI hook.

use crate::types::{AgentTool, AgentToolResult, ToolResultContent};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct UserQuestionOption {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub label: String,
    #[serde(default)]
    pub description: String,
}

impl UserQuestionOption {
    pub fn stable_id(&self, index: usize) -> String {
        self.id
            .as_deref()
            .filter(|id| !id.trim().is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| format!("option-{}", index.saturating_add(1)))
    }

    fn has_valid_id(&self) -> bool {
        self.id.as_deref().is_none_or(|id| !id.trim().is_empty())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct UserQuestionRequest {
    pub question: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(default)]
    pub options: Vec<UserQuestionOption>,
    #[serde(default)]
    pub allow_multiple: bool,
}

#[derive(Default)]
pub struct AskUserQuestionTool;

#[async_trait::async_trait]
impl AgentTool for AskUserQuestionTool {
    fn name(&self) -> &str {
        "ask_user_question"
    }
    fn label(&self) -> &str {
        "Ask user"
    }
    fn description(&self) -> &str {
        "Ask the user a structured question and wait for an answer."
    }
    fn parameters(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "type": "object",
            "properties": {
                "question": { "type": "string", "minLength": 1 },
                "header": { "type": "string" },
                "body": { "type": "string" },
                "options": { "type": "array", "items": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "minLength": 1 },
                        "label": { "type": "string", "minLength": 1 },
                        "description": { "type": "string" }
                    },
                    "required": ["label"]
                }},
                "allow_multiple": { "type": "boolean" }
            },
            "required": ["question"]
        }))
    }
    fn validate_arguments(&self, args: &serde_json::Value) -> Result<(), String> {
        let request: UserQuestionRequest = serde_json::from_value(args.clone())
            .map_err(|error| format!("invalid question: {error}"))?;
        if request.question.trim().is_empty() {
            return Err("question must not be empty".into());
        }
        if request.options.is_empty() {
            return Err("at least one option is required".into());
        }
        if request
            .options
            .iter()
            .any(|option| option.label.trim().is_empty())
        {
            return Err("option labels must not be empty".into());
        }
        if request.options.iter().any(|option| !option.has_valid_id()) {
            return Err("option ids must not be empty".into());
        }
        if !request.allow_multiple && request.options.len() > 32 {
            return Err("a question may contain at most 32 options".into());
        }
        Ok(())
    }
    async fn execute(
        &self,
        _tool_call_id: &str,
        _args: serde_json::Value,
        _signal: Option<tokio_util::sync::CancellationToken>,
        _on_update: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
    ) -> Result<AgentToolResult, String> {
        Err("ask_user_question requires an interactive question hook".into())
    }
}

pub(crate) fn answer_result(answer: serde_json::Value) -> AgentToolResult {
    AgentToolResult {
        content: vec![ToolResultContent::Text {
            text: answer.to_string(),
        }],
        details: answer,
        ..AgentToolResult::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_structured_questions() {
        let tool = AskUserQuestionTool;
        assert!(tool
            .validate_arguments(&serde_json::json!({
                "question": "Ship it?", "header": "Release", "body": "Choose one.",
                "options": [{"label": "Yes"}, {"label": "No"}]
            }))
            .is_ok());
        assert!(tool
            .validate_arguments(&serde_json::json!({
                "question": " ", "options": [{"label": "Yes"}]
            }))
            .is_err());
        assert!(tool
            .validate_arguments(&serde_json::json!({
                "question": "Ship it?", "options": []
            }))
            .is_err());
    }

    #[test]
    fn option_identity_is_explicit_or_deterministically_fallbacked() {
        let explicit = UserQuestionOption {
            id: Some("ship".into()),
            label: "Ship".into(),
            description: String::new(),
        };
        let fallback = UserQuestionOption {
            id: None,
            label: "Wait".into(),
            description: String::new(),
        };
        assert_eq!(explicit.stable_id(0), "ship");
        assert_eq!(fallback.stable_id(1), "option-2");
    }
}
