use super::actor::{ToolExecutorActor, ToolOutcome};
use super::command::{ToolCommandRequest, ToolCommandResult};
use crate::types::{AgentContext, AssistantMessage, ToolCall, ToolExecutionMode};

impl ToolExecutorActor {
    pub async fn execute_command(
        &self,
        request: ToolCommandRequest,
        context: AgentContext,
        hooks: super::executor::ToolExecHooks,
    ) -> ToolCommandResult {
        let tool_name = request.tool_name.clone();
        let call = ToolCall {
            id: format!("command:{tool_name}"),
            name: tool_name.clone(),
            arguments: request.arguments,
            thought_signature: None,
        };
        let outcome = self
            .execute(
                AssistantMessage::default(),
                context,
                None,
                None,
                vec![call],
                ToolExecutionMode::Sequential,
                hooks,
            )
            .await;
        command_result(tool_name, outcome)
    }
}

fn command_result(tool_name: String, outcome: ToolOutcome) -> ToolCommandResult {
    match outcome {
        ToolOutcome::Completed {
            mut tool_results, ..
        } => tool_results
            .pop()
            .map(|result| ToolCommandResult {
                tool_name: tool_name.clone(),
                result: serde_json::json!({"content": result.content, "details": result.details}),
                is_error: result.is_error,
            })
            .unwrap_or(ToolCommandResult {
                tool_name,
                result: serde_json::json!({"error":"tool returned no result"}),
                is_error: true,
            }),
        ToolOutcome::Aborted { reason } => ToolCommandResult {
            tool_name,
            result: serde_json::json!({"error": reason}),
            is_error: true,
        },
    }
}
