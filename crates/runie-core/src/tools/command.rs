//! Replayable data exchanged by slash-command adapters and the tool actor.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCommandRequest {
    pub tool_name: String,
    pub arguments: serde_json::Value,
    pub approval_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCommandResult {
    pub tool_name: String,
    pub result: serde_json::Value,
    pub is_error: bool,
}

impl ToolCommandRequest {
    pub fn new(tool_name: impl Into<String>, arguments: serde_json::Value) -> Self {
        Self {
            tool_name: tool_name.into(),
            arguments,
            approval_required: false,
        }
    }

    pub fn requiring_approval(mut self) -> Self {
        self.approval_required = true;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_command_request_round_trips_as_replay_data() {
        let request = ToolCommandRequest::new(
            "git_commit_prepare",
            serde_json::json!({"message":"ship it"}),
        );
        let restored: ToolCommandRequest =
            serde_json::from_str(&serde_json::to_string(&request).unwrap()).unwrap();
        assert_eq!(restored, request);
        assert!(!restored.approval_required);
    }

    #[test]
    fn approval_is_explicit_data() {
        let request = ToolCommandRequest::new("git_commit", serde_json::json!({}));
        assert!(request.requiring_approval().approval_required);
    }
}
