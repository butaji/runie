//! Typed subagent requests. Execution is supplied by the owning loop actor.

use crate::types::{AgentTool, AgentToolResult, ToolResultContent};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubagentRole {
    Explore,
    Plan,
    Code,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubagentCapability {
    ReadWorkspace,
    ProducePlan,
    WriteWorkspace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SubagentResourceLimits {
    pub max_turns: u32,
    pub max_output_bytes: u64,
    pub max_tool_calls: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SubagentResourceUsage {
    pub turns: u32,
    pub output_bytes: u64,
    pub tool_calls: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SubagentTurnEvent {
    pub output_bytes: u64,
    pub tool_calls: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SubagentUsageState {
    pub usage: SubagentResourceUsage,
}

pub fn reduce_subagent_turn(
    role: SubagentRole,
    state: &mut SubagentUsageState,
    event: SubagentTurnEvent,
) -> Result<(), String> {
    let usage = SubagentResourceUsage {
        turns: state.usage.turns.saturating_add(1),
        output_bytes: state.usage.output_bytes.saturating_add(event.output_bytes),
        tool_calls: state.usage.tool_calls.saturating_add(event.tool_calls),
    };
    role.validate_resource_usage(usage)?;
    state.usage = usage;
    Ok(())
}

impl SubagentResourceUsage {
    pub const fn one_turn_one_tool(output_bytes: u64) -> Self {
        Self {
            turns: 1,
            output_bytes,
            tool_calls: 1,
        }
    }
}

impl SubagentRole {
    pub const fn resource_limits(self) -> SubagentResourceLimits {
        match self {
            Self::Explore => SubagentResourceLimits {
                max_turns: 4,
                max_output_bytes: 32 * 1024,
                max_tool_calls: 16,
            },
            Self::Plan => SubagentResourceLimits {
                max_turns: 6,
                max_output_bytes: 48 * 1024,
                max_tool_calls: 24,
            },
            Self::Code => SubagentResourceLimits {
                max_turns: 12,
                max_output_bytes: 128 * 1024,
                max_tool_calls: 64,
            },
        }
    }

    pub fn validate_resource_usage(self, usage: SubagentResourceUsage) -> Result<(), String> {
        let limits = self.resource_limits();
        if usage.turns > limits.max_turns {
            return Err(format!("subagent exceeded {} turns", limits.max_turns));
        }
        if usage.output_bytes > limits.max_output_bytes {
            return Err(format!(
                "subagent exceeded {} output bytes",
                limits.max_output_bytes
            ));
        }
        if usage.tool_calls > limits.max_tool_calls {
            return Err(format!(
                "subagent exceeded {} tool calls",
                limits.max_tool_calls
            ));
        }
        Ok(())
    }

    pub fn output_bytes(output: &serde_json::Value) -> Result<u64, String> {
        serde_json::to_vec(output)
            .map_err(|error| format!("subagent output is not serializable: {error}"))?
            .len()
            .try_into()
            .map_err(|_| "subagent output byte count overflowed".to_owned())
    }

    pub fn validate_usage(self, usage: SubagentResourceUsage) -> Result<(), String> {
        self.validate_resource_usage(usage)
    }

    pub fn validate_output(self, output: &serde_json::Value) -> Result<(), String> {
        self.clone().validate_usage(self.usage_for_output(output)?)
    }

    pub fn usage_for_output(
        self,
        output: &serde_json::Value,
    ) -> Result<SubagentResourceUsage, String> {
        Ok(SubagentResourceUsage::one_turn_one_tool(
            Self::output_bytes(output)?,
        ))
    }

    pub fn validate_output_usage(
        self,
        output: &serde_json::Value,
    ) -> Result<SubagentResourceUsage, String> {
        let usage = self.clone().usage_for_output(output)?;
        self.validate_usage(usage)?;
        Ok(usage)
    }
}

impl SubagentRole {
    pub const fn capabilities(self) -> &'static [SubagentCapability] {
        match self {
            Self::Explore => &[SubagentCapability::ReadWorkspace],
            Self::Plan => &[
                SubagentCapability::ReadWorkspace,
                SubagentCapability::ProducePlan,
            ],
            Self::Code => &[
                SubagentCapability::ReadWorkspace,
                SubagentCapability::WriteWorkspace,
            ],
        }
    }

    pub fn authorize(self, requested: &[SubagentCapability]) -> Result<(), String> {
        let allowed = self.clone().capabilities();
        let missing = requested
            .iter()
            .filter(|capability| !allowed.contains(capability))
            .map(|capability| format!("{capability:?}"))
            .collect::<Vec<_>>();
        if missing.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "subagent role {:?} does not grant: {}",
                self,
                missing.join(", ")
            ))
        }
    }

    pub fn system_prompt(self) -> &'static str {
        match self {
            Self::Explore => "You are an isolated exploration subagent. Inspect and report facts; do not edit files.",
            Self::Plan => "You are an isolated planning subagent. Return a bounded event-based plan; do not edit files.",
            Self::Code => "You are an isolated coding subagent. Make only the requested implementation changes and verify them.",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SubagentRequest {
    pub role: SubagentRole,
    pub task: String,
    #[serde(default)]
    pub context: String,
    #[serde(default)]
    pub capabilities: Vec<SubagentCapability>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SubagentResult {
    pub role: SubagentRole,
    pub capabilities: Vec<SubagentCapability>,
    pub usage: SubagentResourceUsage,
    pub output: serde_json::Value,
}

#[derive(Default)]
pub struct SubagentTool;

#[async_trait::async_trait]
impl AgentTool for SubagentTool {
    fn name(&self) -> &str {
        "subagent"
    }
    fn label(&self) -> &str {
        "Run subagent"
    }
    fn description(&self) -> &str {
        "Run an isolated explore, plan, or code task."
    }
    fn parameters(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "type": "object",
            "properties": {
                "role": { "type": "string", "enum": ["explore", "plan", "code"] },
                "task": { "type": "string", "minLength": 1 },
                "context": { "type": "string" }
            },
            "required": ["role", "task"]
        }))
    }
    fn validate_arguments(&self, args: &serde_json::Value) -> Result<(), String> {
        let request: SubagentRequest = serde_json::from_value(args.clone())
            .map_err(|error| format!("invalid subagent request: {error}"))?;
        if request.task.trim().is_empty() {
            return Err("subagent task must not be empty".into());
        }
        if request.task.chars().count() > 20_000 {
            return Err("subagent task is too long".into());
        }
        request.role.authorize(&request.capabilities)?;
        Ok(())
    }
    async fn execute(
        &self,
        _tool_call_id: &str,
        _args: serde_json::Value,
        _signal: Option<tokio_util::sync::CancellationToken>,
        _on_update: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
    ) -> Result<AgentToolResult, String> {
        Err("subagent requires an owning subagent hook".into())
    }
}

pub(crate) fn result(
    request: &SubagentRequest,
    value: serde_json::Value,
    usage: SubagentResourceUsage,
) -> AgentToolResult {
    let result = SubagentResult {
        role: request.role.clone(),
        capabilities: request.role.clone().capabilities().to_vec(),
        usage,
        output: value,
    };
    let details = serde_json::to_value(result).expect("subagent result is serializable");
    AgentToolResult {
        content: vec![ToolResultContent::Text {
            text: details.to_string(),
        }],
        details,
        ..AgentToolResult::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_role_and_non_empty_task() {
        let tool = SubagentTool;
        assert!(tool
            .validate_arguments(&serde_json::json!({
                "role": "explore", "task": "find the provider boundary"
            }))
            .is_ok());
        assert!(tool
            .validate_arguments(&serde_json::json!({
                "role": "code", "task": " "
            }))
            .is_err());
        assert!(tool
            .validate_arguments(&serde_json::json!({
                "role": "unknown", "task": "inspect"
            }))
            .is_err());
    }

    #[test]
    fn roles_have_distinct_system_boundaries() {
        assert_ne!(
            SubagentRole::Explore.system_prompt(),
            SubagentRole::Code.system_prompt()
        );
        assert!(SubagentRole::Plan.system_prompt().contains("planning"));
    }

    #[test]
    fn roles_declare_replayable_capabilities() {
        assert_eq!(
            SubagentRole::Explore.capabilities(),
            &[SubagentCapability::ReadWorkspace]
        );
        assert!(SubagentRole::Plan
            .capabilities()
            .contains(&SubagentCapability::ProducePlan));
        assert!(SubagentRole::Code
            .capabilities()
            .contains(&SubagentCapability::WriteWorkspace));
        assert!(!SubagentRole::Explore
            .capabilities()
            .contains(&SubagentCapability::WriteWorkspace));
    }

    #[test]
    fn result_keeps_role_capabilities_and_output_as_data() {
        let request = SubagentRequest {
            role: SubagentRole::Plan,
            task: "outline the change".into(),
            context: String::new(),
            capabilities: vec![SubagentCapability::ProducePlan],
        };
        let result = result(
            &request,
            serde_json::json!({"steps": ["test"]}),
            SubagentResourceUsage::one_turn_one_tool(16),
        );
        let decoded: SubagentResult = serde_json::from_value(result.details).unwrap();
        assert_eq!(decoded.role, SubagentRole::Plan);
        assert_eq!(decoded.output["steps"][0], "test");
        assert_eq!(decoded.usage.turns, 1);
        assert!(decoded
            .capabilities
            .contains(&SubagentCapability::ProducePlan));
    }

    #[test]
    fn capability_admission_rejects_escalation() {
        assert!(SubagentRole::Plan
            .authorize(&[SubagentCapability::ProducePlan])
            .is_ok());
        assert!(SubagentRole::Explore
            .authorize(&[SubagentCapability::WriteWorkspace])
            .unwrap_err()
            .contains("WriteWorkspace"));
    }

    #[test]
    fn role_limits_are_typed_and_reject_resource_escalation() {
        let limits = SubagentRole::Explore.resource_limits();
        assert!(SubagentRole::Explore
            .validate_resource_usage(SubagentResourceUsage {
                turns: limits.max_turns,
                output_bytes: limits.max_output_bytes,
                tool_calls: limits.max_tool_calls,
            })
            .is_ok());
        assert!(SubagentRole::Explore
            .validate_resource_usage(SubagentResourceUsage {
                turns: limits.max_turns + 1,
                output_bytes: 0,
                tool_calls: 0,
            })
            .is_err());
    }

    #[test]
    fn output_boundary_enforces_role_isolation() {
        assert!(SubagentRole::Explore
            .validate_output(&serde_json::json!({"ok": true}))
            .is_ok());
        let output = serde_json::json!("x".repeat(32 * 1024));
        assert!(SubagentRole::Explore.validate_output(&output).is_err());
    }

    #[test]
    fn usage_counts_owned_execution_boundary() {
        let usage = SubagentResourceUsage::one_turn_one_tool(8);
        assert!(SubagentRole::Explore.validate_usage(usage).is_ok());
        assert_eq!(usage.tool_calls, 1);
    }

    #[test]
    fn multi_turn_usage_reduces_and_rejects_only_the_overflowing_trace() {
        let mut state = SubagentUsageState::default();
        for _ in 0..SubagentRole::Explore.resource_limits().max_turns {
            reduce_subagent_turn(
                SubagentRole::Explore,
                &mut state,
                SubagentTurnEvent {
                    output_bytes: 8,
                    tool_calls: 1,
                },
            )
            .unwrap();
        }
        assert_eq!(state.usage.turns, 4);
        assert!(reduce_subagent_turn(
            SubagentRole::Explore,
            &mut state,
            SubagentTurnEvent {
                output_bytes: 1,
                tool_calls: 1,
            },
        )
        .is_err());
        assert_eq!(state.usage.turns, 4);
    }
}
