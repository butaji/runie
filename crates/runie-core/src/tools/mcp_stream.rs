use super::{MCP_HTTP_MAX_RESPONSE_BYTES, MCP_MAX_STREAM_EVENTS};
use std::collections::BTreeMap;

const MAX_MCP_STREAM_NOTIFICATIONS: usize = 4_096;

/// One server-sent MCP event. The JSON-RPC envelope remains data so callers
/// can project responses, notifications, and errors without losing fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpStreamEvent {
    pub event: Option<String>,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct McpStreamSnapshot {
    pub responses: BTreeMap<String, serde_json::Value>,
    pub notifications: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct McpReconnectPolicy {
    pub max_attempts: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
}

impl McpReconnectPolicy {
    pub const fn bounded() -> Self {
        Self {
            max_attempts: 3,
            base_delay_ms: 250,
            max_delay_ms: 4_000,
        }
    }

    pub fn delay_ms(self, attempt: u32) -> Option<u64> {
        if attempt >= self.max_attempts || self.max_attempts == 0 {
            return None;
        }
        let multiplier = 1_u64.checked_shl(attempt).unwrap_or(u64::MAX);
        Some(
            self.base_delay_ms
                .saturating_mul(multiplier)
                .min(self.max_delay_ms),
        )
    }
}

/// Reduce MCP JSON-RPC envelopes into a replayable response/notification
/// projection. Unknown JSON-RPC fields remain intact in the stored values.
pub fn reduce_mcp_stream_event(
    snapshot: &mut McpStreamSnapshot,
    event: &McpStreamEvent,
) -> Result<(), String> {
    let Some(object) = event.data.as_object() else {
        return Err("MCP stream data must be a JSON object".into());
    };
    if let Some(id) = object.get("id") {
        let key = match id {
            serde_json::Value::String(value) => value.clone(),
            serde_json::Value::Number(value) => value.to_string(),
            _ => return Err("MCP response id must be a string or number".into()),
        };
        snapshot.responses.insert(key, event.data.clone());
    } else if object.get("method").is_some() {
        if snapshot.notifications.len() >= MAX_MCP_STREAM_NOTIFICATIONS {
            return Err(format!(
                "MCP notification projection exceeds {} entries",
                MAX_MCP_STREAM_NOTIFICATIONS
            ));
        }
        snapshot.notifications.push(event.data.clone());
    } else {
        return Err("MCP stream envelope needs an id or method".into());
    }
    Ok(())
}

/// Parse a bounded MCP `text/event-stream` body into ordered events.
pub fn parse_mcp_event_stream(body: &[u8]) -> Result<Vec<McpStreamEvent>, String> {
    if body.len() > MCP_HTTP_MAX_RESPONSE_BYTES {
        return Err(format!(
            "MCP event stream exceeds {} bytes",
            MCP_HTTP_MAX_RESPONSE_BYTES
        ));
    }
    let text =
        std::str::from_utf8(body).map_err(|error| format!("invalid MCP event stream: {error}"))?;
    let mut events = Vec::new();
    let mut frame = EventFrame::default();
    for line in text.lines() {
        if line.is_empty() {
            append_frame(&mut events, &mut frame)?;
        } else {
            frame.accept(line)?;
        }
    }
    append_frame(&mut events, &mut frame)?;
    Ok(events)
}

#[derive(Default)]
struct EventFrame {
    event: Option<String>,
    data: Vec<String>,
}

impl EventFrame {
    fn accept(&mut self, line: &str) -> Result<(), String> {
        if line.starts_with(':') {
            return Ok(());
        }
        let (field, value) = line
            .split_once(':')
            .map(|(field, value)| (field, value.strip_prefix(' ').unwrap_or(value)))
            .ok_or_else(|| format!("invalid MCP event line: {line}"))?;
        match field {
            "event" => self.event = Some(value.to_owned()),
            "data" => self.data.push(value.to_owned()),
            _ => {}
        }
        Ok(())
    }
}

fn append_frame(events: &mut Vec<McpStreamEvent>, frame: &mut EventFrame) -> Result<(), String> {
    if frame.data.is_empty() {
        frame.event = None;
        return Ok(());
    }
    if events.len() >= MCP_MAX_STREAM_EVENTS {
        return Err(format!(
            "MCP event stream exceeds {} events",
            MCP_MAX_STREAM_EVENTS
        ));
    }
    let data = serde_json::from_str(&frame.data.join("\n"))
        .map_err(|error| format!("invalid MCP event data: {error}"))?;
    events.push(McpStreamEvent {
        event: frame.event.take(),
        data,
    });
    frame.data.clear();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_stream_reduces_ordered_frames_and_comments() {
        let events = parse_mcp_event_stream(
            b": heartbeat\nevent: message\ndata: {\"id\":1,\"result\":{}}\n\ndata: {\"method\":\"notifications/progress\"}\n",
        )
        .expect("events");
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event.as_deref(), Some("message"));
        assert_eq!(events[1].data["method"], "notifications/progress");
    }

    #[test]
    fn event_stream_rejects_invalid_json_and_unbounded_events() {
        assert!(parse_mcp_event_stream(b"data: nope\n").is_err());
        let body = (0..=MCP_MAX_STREAM_EVENTS)
            .map(|_| "data: {}\n\n")
            .collect::<String>();
        assert!(parse_mcp_event_stream(body.as_bytes()).is_err());
    }

    #[test]
    fn stream_projection_correlates_responses_and_preserves_notifications() {
        let events = parse_mcp_event_stream(
            b"data: {\"id\":2,\"result\":{\"ok\":true}}\n\ndata: {\"method\":\"notifications/progress\",\"params\":{\"n\":1}}\n",
        )
        .unwrap();
        let mut snapshot = McpStreamSnapshot::default();
        for event in &events {
            reduce_mcp_stream_event(&mut snapshot, event).unwrap();
        }
        assert_eq!(snapshot.responses["2"]["result"]["ok"], true);
        assert_eq!(snapshot.notifications.len(), 1);
    }

    #[test]
    fn reconnect_policy_is_bounded_data_without_sleeping() {
        let policy = McpReconnectPolicy::bounded();
        assert_eq!(policy.delay_ms(0), Some(250));
        assert_eq!(policy.delay_ms(1), Some(500));
        assert_eq!(policy.delay_ms(2), Some(1_000));
        assert_eq!(policy.delay_ms(3), None);
    }

    #[test]
    fn notification_projection_has_a_replayable_bound() {
        let mut snapshot = McpStreamSnapshot {
            notifications: vec![
                serde_json::json!({"method": "notice"});
                MAX_MCP_STREAM_NOTIFICATIONS
            ],
            ..McpStreamSnapshot::default()
        };
        let event = McpStreamEvent {
            event: None,
            data: serde_json::json!({"method": "notice"}),
        };
        assert!(reduce_mcp_stream_event(&mut snapshot, &event).is_err());
    }
}
