//! Renderer-independent JSONL contract for CI and scripted callers.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum JsonlEvent {
    Started {
        run_id: String,
    },
    Text {
        text: String,
    },
    Tool {
        name: String,
        result: serde_json::Value,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jsonl_events_round_trip_and_exit_codes_are_stable() {
        let events = vec![
            JsonlEvent::Started {
                run_id: "r1".into(),
            },
            JsonlEvent::Text {
                text: "done".into(),
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
}
