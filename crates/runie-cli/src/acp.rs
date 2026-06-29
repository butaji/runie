//! ACP (Agent Client Protocol) over stdio — JSON-RPC 2.0 interface to Runie.
//!
//! This module exposes the full Runie event bus as JSON-RPC 2.0 over stdin/stdout.
//! Clients send requests (user input, interrupts, permission responses) and receive
//! notifications (turn progress, tool calls, completions) as an async event stream.
//!
//! Request/Response flow:
//!   Client → stdin → ACP adapter → EventBus → Actors
//!   Client ← stdout ← ACP adapter ← EventBus ← Facts/Events
//!
//! JSON-RPC Methods:
//!   initialize     → { "name": "runie-acp", "version": "...", "protocolVersion": "1.0.0" }
//!   submit_input   → { "input": "..." } → { "turnId": "..." }
//!   interrupt      → {} → {}
//!   permission_resp → { "requestId": "...", "action": "allow|deny" } → {}
//!   shutdown       → {} → {}
//!
//! JSON-RPC Notifications (events from Runie):
//!   turn_started       → { "turnId": "..." }
//!   turn_complete      → { "turnId": "...", "responseId": "..." }
//!   text_delta         → { "content": "..." }
//!   tool_start         → { "id": "...", "name": "...", "input": {...} }
//!   tool_input_delta   → { "id": "...", "delta": "..." }
//!   tool_end           → { "id": "...", "durationSecs": 0.5, "output": "..." }
//!   permission_request → { "requestId": "...", "tool": "...", "input": {...} }
//!   error              → { "code": "...", "message": "..." }
//!   end                → { "stopReason": "...", "sessionId": "...", "responseId": "..." }

use anyhow::Result;
use runie_agent::AgentActorFactoryImpl;
use runie_core::actors::leader::{Leader, LeaderHandle};
use runie_core::bus::EventBus;
use runie_core::event::Event;
use runie_core::proto::Notification;
use runie_core::proto::{Error, Message, Request, Response};
use runie_provider::DynProviderFactory;
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::time::timeout;

use crate::transport::{parse_request, write_message};

const ACP_PROTOCOL_VERSION: &str = "1.0.0";

/// Run the ACP stdio adapter.
pub async fn run() -> Result<()> {
    let leader = Leader::new();
    let agent_factory = Arc::new(AgentActorFactoryImpl);
    let provider_factory = Arc::new(DynProviderFactory);
    let handle = leader
        .start(provider_factory, agent_factory)
        .await
        .map_err(|e| anyhow::anyhow!("leader bootstrap failed: {}", e))?;

    // Single task: subscribe to bus events and forward them to stdout as JSON-RPC.
    spawn_event_forwarder(handle.event_bus().clone());

    let stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    process_stdin_loop(stdin, &mut stdout, &handle).await
}

/// Spawn a task that forwards events from the bus to stdout as JSON-RPC notifications.
fn spawn_event_forwarder(bus: EventBus<Event>) {
    tokio::spawn(async move {
        let mut sub = bus.subscribe();
        while let Ok(evt) = sub.recv().await {
            if let Some(notif) = event_to_notification(&evt) {
                let msg = Message::Notification(notif);
                if let Ok(json) = serde_json::to_string(&msg) {
                    let mut stdout = tokio::io::stdout();
                    let _ = stdout.write_all(json.as_bytes()).await;
                    let _ = stdout.write_all(b"\n").await;
                    let _ = stdout.flush().await;
                }
            }
        }
    });
}

async fn process_stdin_loop(
    stdin: tokio::io::Stdin,
    stdout: &mut tokio::io::Stdout,
    handle: &LeaderHandle,
) -> Result<()> {
    let mut lines = BufReader::new(stdin).lines();
    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }
        let req = match parse_request(&line) {
            Ok(r) => r,
            Err((data, err)) => {
                let msg = Message::error(data, err);
                write_message(stdout, &msg).await?;
                continue;
            }
        };
        let id = req.id.clone();
        match dispatch_acp_request(&req, handle).await {
            Ok(result) => {
                let resp = Message::Response(Response::ok(id, result));
                write_message(stdout, &resp).await?;
            }
            Err(e) => {
                let err = Message::error(id, Error::internal(format!("ACP error: {e}")));
                write_message(stdout, &err).await?;
            }
        }
    }
    Ok(())
}

async fn dispatch_acp_request(req: &Request, handle: &LeaderHandle) -> Result<Value> {
    match req.method.as_str() {
        "initialize" => initialize(),
        "submit_input" => submit_input(&req.params, handle).await,
        "interrupt" => interrupt(handle.event_bus()).await,
        "permission_resp" => permission_resp(&req.params, handle.event_bus()).await,
        "shutdown" => shutdown(handle.event_bus()).await,
        _ => Err(anyhow::anyhow!("Unknown method: {}", req.method)),
    }
}

fn initialize() -> Result<Value> {
    Ok(serde_json::json!({
        "name": "runie-acp",
        "version": env!("CARGO_PKG_VERSION"),
        "protocolVersion": ACP_PROTOCOL_VERSION
    }))
}

#[derive(Debug, Deserialize)]
struct SubmitInputParams {
    input: String,
}

async fn submit_input(params: &Value, handle: &LeaderHandle) -> Result<Value> {
    let params: SubmitInputParams = serde_json::from_value(params.clone())?;
    let id = turn_id_from_time();

    // Send the message directly to the turn actor so it picks up the turn.
    use runie_core::actors::TurnMsg;
    handle.turn.send(TurnMsg::SubmitUserMessage { content: params.input, id: id.clone() }).await;

    // Wait for turn completion by subscribing to the bus.
    let bus = handle.event_bus();
    let mut sub = bus.subscribe();
    timeout(Duration::from_secs(300), async {
        while let Ok(evt) = sub.recv().await {
            if let Event::TurnComplete { id, .. } = evt {
                return Ok(serde_json::json!({ "turnId": id, "responseId": id }));
            }
        }
        Ok(serde_json::json!({ "turnId": id }))
    })
    .await
    .map_err(|_| anyhow::anyhow!("Timeout waiting for turn"))?
}

fn turn_id_from_time() -> String {
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("turn_{ms}")
}

async fn interrupt(bus: &EventBus<Event>) -> Result<Value> {
    bus.publish(Event::Abort);
    Ok(serde_json::json!({}))
}

#[derive(Debug, Deserialize)]
struct PermissionRespParams {
    request_id: String,
    action: String,
}

async fn permission_resp(params: &Value, bus: &EventBus<Event>) -> Result<Value> {
    let params: PermissionRespParams = serde_json::from_value(params.clone())?;
    let action = match params.action.as_str() {
        "allow" => runie_core::permissions::PermissionAction::Allow,
        _ => runie_core::permissions::PermissionAction::Deny,
    };
    bus.publish(Event::PermissionResponse {
        request_id: params.request_id,
        action,
    });
    Ok(serde_json::json!({}))
}

async fn shutdown(bus: &EventBus<Event>) -> Result<Value> {
    bus.publish(Event::Quit);
    Ok(serde_json::json!({}))
}

/// Convert a core Event to a JSON-RPC Notification.
fn event_to_notification(event: &Event) -> Option<Notification> {
    let (method, params) = match event {
        Event::TurnComplete { id, duration_secs } => (
            "turn_complete",
            serde_json::json!({ "turnId": id, "responseId": id, "durationSecs": duration_secs }),
        ),
        Event::TextStart { id } => ("text_start", serde_json::json!({ "id": id })),
        Event::TextEnd { id } => ("text_end", serde_json::json!({ "id": id })),
        Event::Response { id, content } => (
            "text_delta",
            serde_json::json!({ "id": id, "content": content }),
        ),
        Event::ThinkingStart { id } => ("thinking_start", serde_json::json!({ "id": id })),
        Event::ThinkingEnd { id } => ("thinking_end", serde_json::json!({ "id": id })),
        Event::ThinkingDelta { id, content } => (
            "thinking_delta",
            serde_json::json!({ "id": id, "content": content }),
        ),
        Event::ToolStart { id, name, input } => (
            "tool_start",
            serde_json::json!({ "id": id, "name": name, "input": input }),
        ),
        Event::ToolEnd { id, duration_secs, output } => (
            "tool_end",
            serde_json::json!({ "id": id, "durationSecs": duration_secs, "output": output }),
        ),
        Event::PermissionRequest { request_id, tool, input } => (
            "permission_request",
            serde_json::json!({ "requestId": request_id, "tool": tool, "input": input }),
        ),
        Event::Error { id, message } => (
            "error",
            serde_json::json!({ "id": id, "message": message }),
        ),
        Event::Done { id } => ("end", serde_json::json!({ "stopReason": "done", "responseId": id })),
        Event::Quit => ("shutdown", serde_json::json!({})),
        _ => return None,
    };
    Some(Notification::new(method, params))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acp_initialize_returns_version() {
        let result = initialize().unwrap();
        assert_eq!(result["name"], "runie-acp");
        assert_eq!(result["protocolVersion"], ACP_PROTOCOL_VERSION);
    }

    #[test]
    fn submit_input_params_parse() {
        let json = serde_json::json!({ "input": "hello world" });
        let params: SubmitInputParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.input, "hello world");
    }

    #[test]
    fn permission_resp_params_parse() {
        let json = serde_json::json!({
            "request_id": "req_123",
            "action": "allow"
        });
        let params: PermissionRespParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.request_id, "req_123");
        assert_eq!(params.action, "allow");
    }

    #[test]
    fn event_to_notification_turn_complete() {
        let event = Event::TurnComplete {
            id: "t1".into(),
            duration_secs: 1.5,
        };
        let notif = event_to_notification(&event);
        assert!(notif.is_some());
        let notif = notif.unwrap();
        assert_eq!(notif.method, "turn_complete");
    }

    #[test]
    fn event_to_notification_tool_start() {
        let event = Event::ToolStart {
            id: "c1".into(),
            name: "bash".into(),
            input: serde_json::json!({"cmd": "ls"}),
        };
        let notif = event_to_notification(&event);
        assert!(notif.is_some());
        let notif = notif.unwrap();
        assert_eq!(notif.method, "tool_start");
    }

    #[test]
    fn event_to_notification_ignores_input_events() {
        let event = Event::Input('x');
        let notif = event_to_notification(&event);
        assert!(notif.is_none());
    }
}
