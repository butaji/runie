//! Pi-agent-core's closed event contract.
//!
//! The contract is declared once through `pi_event_contract!`; the enum,
//! adapters, and serialization shape are generated from that declaration.

use serde::{Deserialize, Serialize};

use crate::types::{AgentEvent, AgentMessage, AssistantMessageEvent, ToolResultMessage};

macro_rules! pi_event_contract {
    (
        unit { $( $unit:ident ),* $(,)? },
        payload { $(
            $variant:ident {
                $( $field:ident $(=> $wire:literal)? : $ty:ty ),* $(,)?
            }
        ),* $(,)? }
    ) => {
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
        #[allow(
            clippy::large_enum_variant,
            reason = "the Pi wire contract keeps assistant payloads inline"
        )]
        #[serde(tag = "type", rename_all = "snake_case", rename_all_fields = "camelCase")]
        pub enum PiAgentEvent {
            $( $unit, )*
            $( $variant {
                $( $(#[serde(rename = $wire)])? $field: $ty ),*
            }, )*
        }

        impl TryFrom<AgentEvent> for PiAgentEvent {
            type Error = AgentEvent;

            #[allow(clippy::too_many_lines, reason = "generated boundary mapping is auditable")]
            fn try_from(event: AgentEvent) -> Result<Self, Self::Error> {
                Ok(match event {
                    $( AgentEvent::$unit => Self::$unit, )*
                    $( AgentEvent::$variant { $( $field ),* } =>
                        Self::$variant { $( $field ),* }, )*
                    other => return Err(other),
                })
            }
        }

        impl PiAgentEvent {
            #[allow(clippy::too_many_lines, reason = "generated boundary mapping is auditable")]
            pub fn try_into_agent_event(self) -> AgentEvent {
                match self {
                    $( Self::$unit => AgentEvent::$unit, )*
                    $( Self::$variant { $( $field ),* } =>
                        AgentEvent::$variant { $( $field ),* }, )*
                }
            }
        }
    };
}

pi_event_contract! {
    unit { AgentStart, TurnStart },
    payload {
        AgentEnd { messages: Vec<AgentMessage> },
        TurnEnd {
            message: AgentMessage,
            tool_results: Vec<ToolResultMessage>
        },
        MessageStart { message: AgentMessage },
        MessageUpdate {
            message: AgentMessage,
            event => "assistantMessageEvent": AssistantMessageEvent
        },
        MessageEnd { message: AgentMessage },
        ToolExecutionStart {
            tool_call_id: String,
            tool_name: String,
            args: serde_json::Value
        },
        ToolExecutionUpdate {
            tool_call_id: String,
            tool_name: String,
            args: serde_json::Value,
            partial_result: serde_json::Value
        },
        ToolExecutionEnd {
            tool_call_id: String,
            tool_name: String,
            result: serde_json::Value,
            is_error: bool
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
    fn model_configuration_events_are_rejected_at_pi_boundary() {
        let event = AgentEvent::ModelChanged {
            model: crate::types::Model {
                id: "runie-model".into(),
                context_window: 42_000,
                ..crate::types::Model::default()
            },
        };
        assert!(PiAgentEvent::try_from(event).is_err());
    }

    #[test]
    fn pi_lifecycle_round_trips_through_boundary() {
        let pi = PiAgentEvent::try_from(AgentEvent::TurnStart).expect("Pi event");
        assert!(matches!(pi.try_into_agent_event(), AgentEvent::TurnStart));
    }

    #[test]
    fn generated_wire_names_preserve_pi_message_update_contract() {
        let event = PiAgentEvent::MessageUpdate {
            message: AgentMessage::Assistant(crate::types::AssistantMessage::default()),
            event: AssistantMessageEvent::Start {
                partial: crate::types::AssistantMessage::default(),
            },
        };
        let value = serde_json::to_value(event).expect("serialize Pi event");
        assert!(value.get("assistantMessageEvent").is_some());
        assert!(value.get("event").is_none());
    }
}
