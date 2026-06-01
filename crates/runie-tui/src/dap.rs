//! Basic Debug Adapter Protocol (DAP) implementation for runie TUI.
//!
//! This provides a minimal DAP server that can be used by editors
//! that support the Debug Adapter Protocol (VS Code, Neovim, etc.)

use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, Mutex};
use serde::Serialize;
use serde_json::Value;

/// DAP message types
#[derive(Debug, Clone)]
pub enum DapMessage {
    /// Request from client
    Request(DapRequest),
    /// Response to a request
    Response(DapResponse),
    /// Event from server
    Event(DapEvent),
}

#[derive(Debug, Clone)]
pub struct DapRequest {
    pub seq: i64,
    pub command: String,
    pub arguments: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct DapResponse {
    pub seq: i64,
    pub request_seq: i64,
    pub success: bool,
    pub command: String,
    pub body: Value,
    pub message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DapEvent {
    pub seq: i64,
    pub event: String,
    pub body: Value,
}

/// DAP Server capabilities
#[derive(Debug, Clone, Serialize)]
pub struct DapCapabilities {
    pub supports_configuration_done_request: bool,
    pub supports_function_breakpoints: bool,
    pub supports_conditional_breakpoints: bool,
    pub supports_hit_conditional_breakpoints: bool,
    pub supports_evaluate_for_helpers: bool,
    pub supports_step_back: bool,
    pub supports_set_variable: bool,
    pub supports_restart_frame: bool,
    pub supports_goto_targets_request: bool,
    pub supports_cancel_request: bool,
    pub supports_breakpoints: bool,
    pub supports_exception_info: bool,
    pub supports_terminate_request: bool,
}

impl Default for DapCapabilities {
    fn default() -> Self {
        DapCapabilities {
            supports_configuration_done_request: false,
            supports_function_breakpoints: false,
            supports_conditional_breakpoints: false,
            supports_hit_conditional_breakpoints: false,
            supports_evaluate_for_helpers: false,
            supports_step_back: false,
            supports_set_variable: false,
            supports_restart_frame: false,
            supports_goto_targets_request: false,
            supports_cancel_request: false,
            supports_breakpoints: false,
            supports_exception_info: false,
            supports_terminate_request: false,
        }
    }
}

/// Connection state for the DAP server
pub struct DapConnection {
    pub initialized: bool,
    pub launched: bool,
    pub stopped: bool,
    pub capabilities: DapCapabilities,
}

impl Default for DapConnection {
    fn default() -> Self {
        DapConnection {
            initialized: false,
            launched: false,
            stopped: false,
            capabilities: DapCapabilities::default(),
        }
    }
}

/// DAP Adapter - minimal implementation
pub struct DapAdapter {
    connection: Arc<Mutex<DapConnection>>,
    command_tx: broadcast::Sender<super::debug_server::AgentCommand>,
}

impl DapAdapter {
    /// Create a new DAP adapter
    pub fn new(command_tx: broadcast::Sender<super::debug_server::AgentCommand>) -> Self {
        DapAdapter {
            connection: Arc::new(Mutex::new(DapConnection::default())),
            command_tx,
        }
    }

    /// Start the DAP server on the given port
    pub async fn start(self, port: u16) {
        let addr = format!("127.0.0.1:{}", port);
        let listener = match TcpListener::bind(&addr).await {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Failed to bind DAP server to {}: {}", addr, e);
                return;
            }
        };

        println!("DAP server listening on {}", addr);

        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    let connection = self.connection.clone();
                    let command_tx = self.command_tx.clone();

                    tokio::spawn(async move {
                        if let Err(e) = handle_dap_connection(stream, peer_addr, connection, command_tx).await {
                            eprintln!("DAP error from {}: {}", peer_addr, e);
                        }
                    });
                }
                Err(e) => {
                    eprintln!("DAP failed to accept: {}", e);
                }
            }
        }
    }
}

async fn handle_dap_connection(
    stream: TcpStream,
    _peer_addr: std::net::SocketAddr,
    connection: Arc<Mutex<DapConnection>>,
    command_tx: broadcast::Sender<super::debug_server::AgentCommand>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (mut reader, mut writer) = tokio::io::split(stream);
    let mut seq: i64 = 1;

    loop {
        let mut header_buffer = String::new();
        let mut content_length: Option<usize> = None;

        // Read headers
        loop {
            let mut buf = [0u8; 1];
            reader.read_exact(&mut buf).await?;

            if buf[0] == b'\r' {
                // Peek next byte
                reader.read_exact(&mut buf).await?;
                if buf[0] == b'\n' {
                    // Check if this is the end of headers
                    if header_buffer.trim().is_empty() {
                        break;
                    }
                    // Parse Content-Length
                    if header_buffer.starts_with("Content-Length:") {
                        let len_str = header_buffer.trim_start_matches("Content-Length:").trim();
                        content_length = Some(len_str.parse()?);
                    }
                    header_buffer.clear();
                }
            } else if buf[0] != b'\n' {
                header_buffer.push(buf[0] as char);
            }
        }

        let body_len = content_length.ok_or("Missing Content-Length header")?;
        let mut body = vec![0u8; body_len];
        reader.read_exact(&mut body).await?;

        let body_str = String::from_utf8_lossy(&body);
        let message: Value = serde_json::from_str(&body_str)?;

        let response = if let (Some("request"), Some(cmd), Some(args), Some(msg_seq)) =
            (message.get("type").and_then(|v| v.as_str()),
             message.get("command").and_then(|v| v.as_str()),
             message.get("arguments"),
             message.get("seq").and_then(|v| v.as_i64()))
        {
            if cmd == "initialize" {
                let response_body = serde_json::json!({
                    "capabilities": DapCapabilities::default()
                });
                DapMessage::Response(DapResponse {
                    seq,
                    request_seq: msg_seq,
                    success: true,
                    command: cmd.to_string(),
                    body: response_body,
                    message: None,
                })
            } else if cmd == "launch" {
                let mut conn = connection.lock().await;
                conn.launched = true;
                DapMessage::Response(DapResponse {
                    seq,
                    request_seq: msg_seq,
                    success: true,
                    command: cmd.to_string(),
                    body: serde_json::json!({}),
                    message: None,
                })
            } else if cmd == "disconnect" {
                let mut conn = connection.lock().await;
                conn.launched = false;
                conn.initialized = false;
                DapMessage::Response(DapResponse {
                    seq,
                    request_seq: msg_seq,
                    success: true,
                    command: cmd.to_string(),
                    body: serde_json::json!({}),
                    message: None,
                })
            } else if cmd == "configurationDone" {
                DapMessage::Response(DapResponse {
                    seq,
                    request_seq: msg_seq,
                    success: true,
                    command: cmd.to_string(),
                    body: serde_json::json!({}),
                    message: None,
                })
            } else if cmd == "setBreakpoints" {
                // Silently accept breakpoint requests
                DapMessage::Response(DapResponse {
                    seq,
                    request_seq: msg_seq,
                    success: true,
                    command: cmd.to_string(),
                    body: serde_json::json!({"breakpoints": []}),
                    message: None,
                })
            } else if cmd == "threads" {
                DapMessage::Response(DapResponse {
                    seq,
                    request_seq: msg_seq,
                    success: true,
                    command: cmd.to_string(),
                    body: serde_json::json!({"threads": [{
                        "id": 1,
                        "name": "runie"
                    }]}),
                    message: None,
                })
            } else if cmd == "stackTrace" {
                DapMessage::Response(DapResponse {
                    seq,
                    request_seq: msg_seq,
                    success: true,
                    command: cmd.to_string(),
                    body: serde_json::json!({"stackFrames": [{
                        "id": 1,
                        "name": "main",
                        "source": { "name": "runie" },
                        "line": 1,
                        "column": 1
                    }]}),
                    message: None,
                })
            } else if cmd == "scopes" {
                DapMessage::Response(DapResponse {
                    seq,
                    request_seq: msg_seq,
                    success: true,
                    command: cmd.to_string(),
                    body: serde_json::json!({"scopes": [{
                        "name": "Locals",
                        "variablesReference": 1,
                        "expensive": false
                    }]}),
                    message: None,
                })
            } else if cmd == "variables" {
                DapMessage::Response(DapResponse {
                    seq,
                    request_seq: msg_seq,
                    success: true,
                    command: cmd.to_string(),
                    body: serde_json::json!({"variables": []}),
                    message: None,
                })
            } else if cmd == "pause" || cmd == "continue" || cmd == "stepOver" || cmd == "stepIn" || cmd == "stepOut" {
                // Accept but don't actually pause
                DapMessage::Response(DapResponse {
                    seq,
                    request_seq: msg_seq,
                    success: true,
                    command: cmd.to_string(),
                    body: serde_json::json!({}),
                    message: None,
                })
            } else if cmd == "evaluate" {
                // Evaluate is often used for REPL-like functionality
                DapMessage::Response(DapResponse {
                    seq,
                    request_seq: msg_seq,
                    success: true,
                    command: cmd.to_string(),
                    body: serde_json::json!({
                        "result": "evaluated",
                        "type": "string",
                        "variablesReference": 0
                    }),
                    message: None,
                })
            } else {
                DapMessage::Response(DapResponse {
                    seq,
                    request_seq: msg_seq,
                    success: false,
                    command: cmd.to_string(),
                    body: serde_json::json!({}),
                    message: Some(format!("Command '{}' not supported", cmd)),
                })
            }
        } else {
            // Unknown message format
            continue;
        };

        // Send response
        let msg_type = match &response {
            DapMessage::Request(_) => "request",
            DapMessage::Response(_) => "response",
            DapMessage::Event(_) => "event",
        };

        let response_body = match &response {
            DapMessage::Response(r) => serde_json::json!({
                "request_seq": r.request_seq,
                "success": r.success,
                "command": r.command,
                "body": r.body
            }),
            _ => serde_json::json!({}),
        };

        let response_json = serde_json::to_string(&serde_json::json!({
            "type": msg_type,
            "seq": seq,
            "body": response_body,
        }))?;

        let response_str = format!("Content-Length: {}\r\n\r\n{}", response_json.len(), response_json);
        writer.write_all(response_str.as_bytes()).await?;
        writer.flush().await?;

        seq += 1;
    }
}
