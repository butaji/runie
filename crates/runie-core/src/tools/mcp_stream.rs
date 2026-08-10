use super::{MCP_HTTP_MAX_RESPONSE_BYTES, MCP_MAX_STREAM_EVENTS};
use std::collections::BTreeMap;
use std::collections::VecDeque;

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

impl McpStreamSnapshot {
    pub fn terminal_lines(&self) -> Vec<String> {
        vec![
            format!("responses: {}", self.responses.len()),
            format!("notifications: {}", self.notifications.len()),
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct McpNotificationQueue {
    pub capacity: usize,
    pub pending: VecDeque<serde_json::Value>,
    pub dropped: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum McpNotificationQueueEvent {
    Push(serde_json::Value),
    Pop,
    Clear,
}

pub fn reduce_mcp_notification_queue(
    queue: &mut McpNotificationQueue,
    event: McpNotificationQueueEvent,
) -> Option<serde_json::Value> {
    match event {
        McpNotificationQueueEvent::Push(notification) => {
            if queue.pending.len() >= queue.capacity {
                queue.dropped = queue.dropped.saturating_add(1);
            } else {
                queue.pending.push_back(notification);
            }
            None
        }
        McpNotificationQueueEvent::Pop => queue.pending.pop_front(),
        McpNotificationQueueEvent::Clear => {
            queue.pending.clear();
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpBackpressureStatus {
    Clear,
    Saturated,
    Dropping,
}

impl McpNotificationQueue {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            pending: VecDeque::new(),
            dropped: 0,
        }
    }

    pub fn replay<I>(capacity: usize, events: I) -> Self
    where
        I: IntoIterator<Item = McpNotificationQueueEvent>,
    {
        let mut queue = Self::new(capacity);
        for event in events {
            queue.apply(event);
        }
        queue
    }

    pub fn push(&mut self, notification: serde_json::Value) {
        self.apply(McpNotificationQueueEvent::Push(notification));
    }

    pub fn apply(&mut self, event: McpNotificationQueueEvent) -> Option<serde_json::Value> {
        reduce_mcp_notification_queue(self, event)
    }

    pub fn clear(&mut self) {
        self.apply(McpNotificationQueueEvent::Clear);
    }

    pub fn pop(&mut self) -> Option<serde_json::Value> {
        self.apply(McpNotificationQueueEvent::Pop)
    }

    pub fn backpressure(&self) -> McpBackpressureStatus {
        if self.dropped > 0 {
            McpBackpressureStatus::Dropping
        } else if self.capacity > 0 && self.pending.len() >= self.capacity {
            McpBackpressureStatus::Saturated
        } else {
            McpBackpressureStatus::Clear
        }
    }

    pub fn terminal_lines(&self) -> Vec<String> {
        vec![
            format!("pending: {}", self.pending.len()),
            format!("capacity: {}", self.capacity),
            format!("dropped: {}", self.dropped),
            format!("backpressure: {:?}", self.backpressure()),
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct McpReconnectPolicy {
    pub max_attempts: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpConnectionStatus {
    Connected,
    Reconnecting,
    Exhausted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct McpReconnectState {
    pub attempts: u32,
    pub status: McpConnectionStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpReconnectDecision {
    RetryAfter { delay_ms: u64 },
    Exhausted,
}

impl McpReconnectState {
    pub const fn connected() -> Self {
        Self {
            attempts: 0,
            status: McpConnectionStatus::Connected,
        }
    }

    pub fn disconnected(self, policy: McpReconnectPolicy) -> (Self, McpReconnectDecision) {
        let attempts = self.attempts.saturating_add(1);
        match policy.delay_ms(attempts - 1) {
            Some(delay_ms) => (
                Self {
                    attempts,
                    status: McpConnectionStatus::Reconnecting,
                },
                McpReconnectDecision::RetryAfter { delay_ms },
            ),
            None => (
                Self {
                    attempts,
                    status: McpConnectionStatus::Exhausted,
                },
                McpReconnectDecision::Exhausted,
            ),
        }
    }

    pub const fn reconnected() -> Self {
        Self::connected()
    }
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
        assert_eq!(
            snapshot.terminal_lines(),
            ["responses: 1", "notifications: 1"]
        );
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
    fn reconnect_state_reduces_disconnects_and_resets_after_connection() {
        let policy = McpReconnectPolicy::bounded();
        let state = McpReconnectState::connected();
        let (state, decision) = state.disconnected(policy);
        assert_eq!(decision, McpReconnectDecision::RetryAfter { delay_ms: 250 });
        assert_eq!(state.status, McpConnectionStatus::Reconnecting);
        let (state, _) = state.disconnected(policy);
        let (state, _) = state.disconnected(policy);
        let (state, exhausted) = state.disconnected(policy);
        assert_eq!(exhausted, McpReconnectDecision::Exhausted);
        assert_eq!(state.status, McpConnectionStatus::Exhausted);
        assert_eq!(McpReconnectState::reconnected().attempts, 0);
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

    #[test]
    fn notification_queue_accounts_for_backpressure_without_reordering() {
        let mut queue = McpNotificationQueue::new(2);
        queue.push(serde_json::json!({"n": 1}));
        queue.push(serde_json::json!({"n": 2}));
        queue.push(serde_json::json!({"n": 3}));
        assert_eq!(queue.dropped, 1);
        assert_eq!(queue.pop().unwrap()["n"], 1);
        assert_eq!(queue.pop().unwrap()["n"], 2);
        assert!(queue.pop().is_none());
        assert_eq!(serde_json::to_value(&queue).unwrap()["capacity"], 2);
    }

    #[test]
    fn notification_queue_events_replay_clear_as_data() {
        let mut queue = McpNotificationQueue::new(2);
        for event in [
            McpNotificationQueueEvent::Push(serde_json::json!({"n": 1})),
            McpNotificationQueueEvent::Push(serde_json::json!({"n": 2})),
            McpNotificationQueueEvent::Clear,
        ] {
            queue.apply(event);
        }
        assert!(queue.pending.is_empty());
        assert_eq!(queue.dropped, 0);
    }

    #[test]
    fn notification_queue_reducer_is_directly_replayable() {
        let mut queue = McpNotificationQueue::new(1);
        reduce_mcp_notification_queue(
            &mut queue,
            McpNotificationQueueEvent::Push(serde_json::json!({"method": "notice"})),
        );
        reduce_mcp_notification_queue(&mut queue, McpNotificationQueueEvent::Clear);
        assert!(queue.pending.is_empty());
    }

    #[test]
    fn notification_queue_replay_preserves_order_and_drop_data() {
        let queue = McpNotificationQueue::replay(
            1,
            [
                McpNotificationQueueEvent::Push(serde_json::json!({"n": 1})),
                McpNotificationQueueEvent::Push(serde_json::json!({"n": 2})),
                McpNotificationQueueEvent::Pop,
                McpNotificationQueueEvent::Push(serde_json::json!({"n": 3})),
            ],
        );
        assert_eq!(
            queue.pending.iter().collect::<Vec<_>>(),
            [&serde_json::json!({"n": 3})]
        );
        assert_eq!(queue.dropped, 1);
    }

    #[test]
    fn notification_queue_projects_stable_backpressure_rows() {
        let mut queue = McpNotificationQueue::new(2);
        queue.push(serde_json::json!({"n": 1}));
        queue.push(serde_json::json!({"n": 2}));
        queue.push(serde_json::json!({"n": 3}));
        assert_eq!(
            queue.terminal_lines(),
            vec![
                "pending: 2",
                "capacity: 2",
                "dropped: 1",
                "backpressure: Dropping"
            ]
        );
    }

    #[test]
    fn notification_queue_distinguishes_saturation_before_drops() {
        let mut queue = McpNotificationQueue::new(1);
        queue.push(serde_json::json!({"n": 1}));
        assert_eq!(queue.backpressure(), McpBackpressureStatus::Saturated);
        queue.push(serde_json::json!({"n": 2}));
        assert_eq!(queue.backpressure(), McpBackpressureStatus::Dropping);
    }
}
