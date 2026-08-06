//! Pi-agent-core's closed event contract.
//!
//! `AgentEvent` also carries Runie application projections for compatibility.
//! New core consumers should accept this type instead; conversion rejects
//! TUI/application-only variants at the boundary.

use serde::{Deserialize, Serialize};

use crate::types::{AgentEvent, AgentMessage, AssistantMessageEvent, ToolResultMessage};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(
    clippy::large_enum_variant,
    reason = "the Pi wire contract keeps the assistant event payload inline"
)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum PiAgentEvent {
    AgentStart,
    AgentEnd {
        messages: Vec<AgentMessage>,
    },
    TurnStart,
    TurnEnd {
        message: AgentMessage,
        tool_results: Vec<ToolResultMessage>,
    },
    MessageStart {
        message: AgentMessage,
    },
    MessageUpdate {
        message: AgentMessage,
        #[serde(rename = "assistantMessageEvent")]
        event: AssistantMessageEvent,
    },
    MessageEnd {
        message: AgentMessage,
    },
    ToolExecutionStart {
        tool_call_id: String,
        tool_name: String,
        args: serde_json::Value,
    },
    ToolExecutionUpdate {
        tool_call_id: String,
        tool_name: String,
        args: serde_json::Value,
        partial_result: serde_json::Value,
    },
    ToolExecutionEnd {
        tool_call_id: String,
        tool_name: String,
        result: serde_json::Value,
        is_error: bool,
    },
}

impl TryFrom<AgentEvent> for PiAgentEvent {
    type Error = AgentEvent;

    #[allow(
        clippy::too_many_lines,
        reason = "explicit boundary mapping is auditable"
    )]
    fn try_from(event: AgentEvent) -> Result<Self, Self::Error> {
        Ok(match event {
            AgentEvent::AgentStart => Self::AgentStart,
            AgentEvent::AgentEnd { messages } => Self::AgentEnd { messages },
            AgentEvent::TurnStart => Self::TurnStart,
            AgentEvent::TurnEnd {
                message,
                tool_results,
            } => Self::TurnEnd {
                message,
                tool_results,
            },
            AgentEvent::MessageStart { message } => Self::MessageStart { message },
            AgentEvent::MessageUpdate { message, event } => Self::MessageUpdate { message, event },
            AgentEvent::MessageEnd { message } => Self::MessageEnd { message },
            AgentEvent::ToolExecutionStart {
                tool_call_id,
                tool_name,
                args,
            } => Self::ToolExecutionStart {
                tool_call_id,
                tool_name,
                args,
            },
            AgentEvent::ToolExecutionUpdate {
                tool_call_id,
                tool_name,
                args,
                partial_result,
            } => Self::ToolExecutionUpdate {
                tool_call_id,
                tool_name,
                args,
                partial_result,
            },
            AgentEvent::ToolExecutionEnd {
                tool_call_id,
                tool_name,
                result,
                is_error,
            } => Self::ToolExecutionEnd {
                tool_call_id,
                tool_name,
                result,
                is_error,
            },
            other => return Err(other),
        })
    }
}

impl PiAgentEvent {
    #[allow(
        clippy::too_many_lines,
        reason = "explicit boundary mapping is auditable"
    )]
    pub fn try_into_agent_event(self) -> AgentEvent {
        match self {
            Self::AgentStart => AgentEvent::AgentStart,
            Self::AgentEnd { messages } => AgentEvent::AgentEnd { messages },
            Self::TurnStart => AgentEvent::TurnStart,
            Self::TurnEnd {
                message,
                tool_results,
            } => AgentEvent::TurnEnd {
                message,
                tool_results,
            },
            Self::MessageStart { message } => AgentEvent::MessageStart { message },
            Self::MessageUpdate { message, event } => AgentEvent::MessageUpdate { message, event },
            Self::MessageEnd { message } => AgentEvent::MessageEnd { message },
            Self::ToolExecutionStart {
                tool_call_id,
                tool_name,
                args,
            } => AgentEvent::ToolExecutionStart {
                tool_call_id,
                tool_name,
                args,
            },
            Self::ToolExecutionUpdate {
                tool_call_id,
                tool_name,
                args,
                partial_result,
            } => AgentEvent::ToolExecutionUpdate {
                tool_call_id,
                tool_name,
                args,
                partial_result,
            },
            Self::ToolExecutionEnd {
                tool_call_id,
                tool_name,
                result,
                is_error,
            } => AgentEvent::ToolExecutionEnd {
                tool_call_id,
                tool_name,
                result,
                is_error,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn application_events_are_rejected_at_pi_boundary() {
        let event = AgentEvent::ThemeChanged {
            theme: crate::types::ThemeKind::GrokNight,
        };
        assert!(PiAgentEvent::try_from(event).is_err());
    }

    #[test]
    fn pi_lifecycle_round_trips_through_boundary() {
        let event = AgentEvent::TurnStart;
        let pi = PiAgentEvent::try_from(event).expect("Pi event");
        assert!(matches!(pi.try_into_agent_event(), AgentEvent::TurnStart));
    }
}
