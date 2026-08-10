use super::{MCP_HTTP_MAX_RESPONSE_BYTES, MCP_MAX_STREAM_EVENTS};

/// One server-sent MCP event. The JSON-RPC envelope remains data so callers
/// can project responses, notifications, and errors without losing fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpStreamEvent {
    pub event: Option<String>,
    pub data: serde_json::Value,
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
}
