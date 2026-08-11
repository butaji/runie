//! Renderer-independent JSONL contract for CI and scripted callers.

use serde::{Deserialize, Serialize};

pub const MAX_JSONL_PROVIDER_EVENTS: usize = 4_096;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[allow(
    clippy::large_enum_variant,
    reason = "Provider events stay lossless and typed at the JSONL boundary"
)]
pub enum JsonlEvent {
    Started {
        run_id: String,
    },
    Metadata {
        provider: Option<String>,
        model: Option<String>,
        context_window: Option<u64>,
        thinking_level: Option<crate::types::ThinkingLevel>,
    },
    Text {
        text: String,
    },
    Usage {
        usage: crate::types::Usage,
    },
    Tool {
        name: String,
        result: serde_json::Value,
    },
    /// Lossless provider stream event for scripted consumers. Keeping the
    /// domain event typed avoids forcing each renderer to invent its own wire
    /// projection.
    Provider {
        event: crate::types::AssistantMessageEvent,
    },
    Finished {
        outcome: RunOutcome,
    },
    Error {
        message: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunOutcome {
    Completed,
    Aborted,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct NonInteractiveConfig {
    pub jsonl: bool,
    pub auto_approve: bool,
    pub prompt: Option<String>,
}

impl NonInteractiveConfig {
    pub fn parse(args: &[String]) -> Result<Self, String> {
        let mut config = Self::default();
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--jsonl" => config.jsonl = true,
                "--yes" | "--auto-approve" => config.auto_approve = true,
                "--prompt" => {
                    index += 1;
                    config.prompt =
                        Some(args.get(index).ok_or("--prompt requires a value")?.clone());
                }
                value if value.starts_with('-') => {
                    return Err(format!("unknown runie option: {value}"));
                }
                value => {
                    if config.prompt.is_some() {
                        return Err("prompt was provided more than once".into());
                    }
                    config.prompt = Some(value.into());
                }
            }
            index += 1;
        }
        Ok(config)
    }
}

impl RunOutcome {
    pub const fn exit_code(self) -> u8 {
        match self {
            Self::Completed => 0,
            Self::Aborted => 130,
            Self::Failed => 1,
        }
    }
}

pub fn encode_jsonl(events: &[JsonlEvent]) -> Result<String, serde_json::Error> {
    events
        .iter()
        .map(serde_json::to_string)
        .collect::<Result<Vec<_>, _>>()
        .map(|lines| format!("{}\n", lines.join("\n")))
}

pub fn decode_jsonl(input: &str) -> Result<Vec<JsonlEvent>, serde_json::Error> {
    input
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(serde_json::from_str)
        .collect()
}

pub fn forward_provider_events(
    events: &[crate::types::AssistantMessageEvent],
) -> Result<Vec<JsonlEvent>, String> {
    if events.len() > MAX_JSONL_PROVIDER_EVENTS {
        return Err(format!(
            "provider event stream exceeds {} events",
            MAX_JSONL_PROVIDER_EVENTS
        ));
    }
    Ok(events
        .iter()
        .cloned()
        .map(|event| JsonlEvent::Provider { event })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jsonl_events_round_trip_and_exit_codes_are_stable() {
        let events = vec![
            JsonlEvent::Started {
                run_id: "r1".into(),
            },
            JsonlEvent::Metadata {
                provider: Some("minimax".into()),
                model: Some("model-1".into()),
                context_window: Some(128_000),
                thinking_level: Some(crate::types::ThinkingLevel::High),
            },
            JsonlEvent::Text {
                text: "done".into(),
            },
            JsonlEvent::Usage {
                usage: crate::types::Usage {
                    input: 4,
                    output: 2,
                    total_tokens: 6,
                    ..Default::default()
                },
            },
            JsonlEvent::Provider {
                event: crate::types::AssistantMessageEvent::TextDelta {
                    index: 0,
                    delta: "streamed".into(),
                    partial: Default::default(),
                },
            },
            JsonlEvent::Finished {
                outcome: RunOutcome::Completed,
            },
        ];
        let encoded = encode_jsonl(&events).unwrap();
        assert_eq!(decode_jsonl(&encoded).unwrap(), events);
        assert_eq!(RunOutcome::Completed.exit_code(), 0);
        assert_eq!(RunOutcome::Aborted.exit_code(), 130);
        assert_eq!(RunOutcome::Failed.exit_code(), 1);
    }

    #[test]
    fn noninteractive_config_round_trips_as_replay_data() {
        let config = NonInteractiveConfig {
            jsonl: true,
            auto_approve: true,
            prompt: Some("inspect workspace".into()),
        };
        let encoded = serde_json::to_string(&config).unwrap();
        let decoded: NonInteractiveConfig = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, config);
    }

    #[test]
    fn noninteractive_args_are_typed_and_approval_is_explicit() {
        let args = ["--jsonl", "--yes", "--prompt", "inspect repo"]
            .into_iter()
            .map(String::from)
            .collect::<Vec<_>>();
        assert_eq!(
            NonInteractiveConfig::parse(&args).unwrap(),
            NonInteractiveConfig {
                jsonl: true,
                auto_approve: true,
                prompt: Some("inspect repo".into())
            }
        );
        assert!(NonInteractiveConfig::parse(&["--prompt".into()]).is_err());
    }

    #[test]
    fn provider_forwarding_is_typed_and_bounded() {
        let events = vec![crate::types::AssistantMessageEvent::TextDelta {
            index: 0,
            delta: "hello".into(),
            partial: Default::default(),
        }];
        let forwarded = forward_provider_events(&events).unwrap();
        assert!(matches!(forwarded[0], JsonlEvent::Provider { .. }));
        let oversized = vec![events[0].clone(); MAX_JSONL_PROVIDER_EVENTS + 1];
        assert!(forward_provider_events(&oversized).is_err());
    }
}
