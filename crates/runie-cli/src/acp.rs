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
use runie_agent::AgentActor;
use runie_core::actors::{
    IoActor, ProviderActor, RactorConfigActor, SessionActor,
};
use runie_core::actors::permission::RactorPermissionActor;
use runie_core::bus::EventBus;
use runie_core::event::Event;
use runie_protocol::Notification;
use runie_protocol::{Error, Message, Request, Response};
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;
use tokio::time::timeout;

const ACP_PROTOCOL_VERSION: &str = "1.0.0";

/// Run the ACP stdio adapter.
pub async fn run() -> Result<()> {
    let bus = EventBus::<Event>::new(100);
    let (event_tx, mut event_rx) = mpsc::channel::<Event>(100);
    let runtime = spawn_runtime(bus.clone()).await?;
    spawn_event_forwarder(bus.clone());
    spawn_combined_receiver(bus, event_tx);
    let stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    process_stdin_loop(stdin, &mut stdout, &runtime, &mut event_rx).await
}

fn spawn_combined_receiver(bus: EventBus<Event>, event_tx: mpsc::Sender<Event>) {
    let (tx, mut local_rx) = mpsc::channel::<Event>(100);
    tokio::spawn(async move {
        let mut sub = bus.subscribe();
        loop {
            tokio::select! {
                biased;
                Some(evt) = local_rx.recv() => {
                    if event_tx.send(evt).await.is_err() { break; }
                }
                result = sub.recv() => {
                    match result {
                        Ok(evt) => {
                            if let Some(notif) = event_to_notification(&evt) {
                                let msg = Message::Notification(notif);
                                if let Ok(json) = serde_json::to_string(&msg) {
                                    let mut out = tokio::io::stdout();
                                    let _ = out.write_all(json.as_bytes()).await;
                                    let _ = out.write_all(b"\n").await;
                                    let _ = out.flush().await;
                                }
                            }
                            if tx.send(evt).await.is_err() { break; }
                        }
                        Err(_) => break,
                    }
                }
            }
        }
    });
}

async fn process_stdin_loop(
    stdin: tokio::io::Stdin,
    stdout: &mut tokio::io::Stdout,
    runtime: &AcpRuntime,
    event_rx: &mut mpsc::Receiver<Event>,
) -> Result<()> {
    let mut lines = BufReader::new(stdin).lines();
    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }
        let req = match serde_json::from_str::<Request>(&line) {
            Ok(r) => r,
            Err(e) => {
                let err = Message::error(Some(Value::String(line)), Error::parse(format!("Parse error: {e}")));
                write_message(stdout, &err).await?;
                continue;
            }
        };
        let id = req.id.clone();
        match dispatch_acp_request(&req, runtime, event_rx).await {
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

async fn write_message<W>(writer: &mut W, msg: &Message) -> Result<()>
where
    W: AsyncWriteExt + Unpin,
{
    let json = serde_json::to_string(msg)?;
    writer.write_all(json.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;
    Ok(())
}

struct AcpRuntime {
    event_tx: mpsc::Sender<Event>,
}

async fn spawn_runtime(bus: EventBus<Event>) -> Result<AcpRuntime> {
    let (event_tx, _event_rx) = mpsc::channel::<Event>(100);
    // Spawn actors
    let (config_handle, _) = RactorConfigActor::spawn(bus.clone(), None).await;
    let (provider_handle, _provider_actor) = ProviderActor::spawn_with_ractor_handle(
        bus.clone(),
        config_handle.clone(),
        Arc::new(runie_provider::DynProviderFactory),
    );
    let (_session_handle, _session_actor) = SessionActor::spawn(bus.clone());
    let (_io_handle, _io_actor) = IoActor::spawn(bus.clone());
    let (permission_handle, _permission_actor) = RactorPermissionActor::spawn(bus.clone()).await;

    // Spawn agent actor
    let (_agent_handle, _agent_actor) = AgentActor::spawn(
        bus.clone(),
        provider_handle.clone(),
        permission_handle.clone(),
    );

    Ok(AcpRuntime { event_tx })
}

async fn dispatch_acp_request(
    req: &Request,
    runtime: &AcpRuntime,
    event_rx: &mut mpsc::Receiver<Event>,
) -> Result<Value> {
    match req.method.as_str() {
        "initialize" => initialize(),
        "submit_input" => submit_input(&req.params, runtime, event_rx).await,
        "interrupt" => interrupt(runtime).await,
        "permission_resp" => permission_resp(&req.params, runtime).await,
        "shutdown" => shutdown(runtime).await,
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

async fn submit_input(
    params: &Value,
    runtime: &AcpRuntime,
    event_rx: &mut mpsc::Receiver<Event>,
) -> Result<Value> {
    let params: SubmitInputParams = serde_json::from_value(params.clone())?;
    let turn_id = turn_id_from_time();

    // Send input event using InputState::default() + set input text
    let mut state = runie_core::model::InputState::default();
    state.input = params.input;
    runtime.event_tx.send(Event::InputChanged { state: Box::new(state) }).await?;
    runtime.event_tx.send(Event::Submit).await?;

    // Wait for turn completion with timeout
    timeout(Duration::from_secs(300), async {
        while let Some(evt) = event_rx.recv().await {
            if let Event::TurnComplete { id, .. } = evt {
                return Ok(serde_json::json!({ "turnId": id, "responseId": id }));
            }
        }
        Ok(serde_json::json!({ "turnId": turn_id }))
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

async fn interrupt(runtime: &AcpRuntime) -> Result<Value> {
    runtime.event_tx.send(Event::Abort).await?;
    Ok(serde_json::json!({}))
}

#[derive(Debug, Deserialize)]
struct PermissionRespParams {
    request_id: String,
    action: String,
}

async fn permission_resp(params: &Value, runtime: &AcpRuntime) -> Result<Value> {
    let params: PermissionRespParams = serde_json::from_value(params.clone())?;
    let action = match params.action.as_str() {
        "allow" => runie_core::permissions::PermissionAction::Allow,
        _ => runie_core::permissions::PermissionAction::Deny,
    };
    runtime.event_tx.send(Event::PermissionResponse {
        request_id: params.request_id,
        action,
    }).await?;
    Ok(serde_json::json!({}))
}

async fn shutdown(runtime: &AcpRuntime) -> Result<Value> {
    let _ = runtime.event_tx.send(Event::Quit).await;
    // Note: shutdown_tx.take() would require interior mutability
    // For ACP mode, shutdown is best-effort
    Ok(serde_json::json!({}))
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
