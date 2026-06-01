//! Debug server for runie TUI.
//!
//! HTTP server that exposes TUI state for debugging, allows screen capture,
//! and supports event injection for testing.

use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::broadcast;
use serde::{Serialize, Deserialize};
use ratatui::buffer::Buffer;

pub use crate::tui::state::{AppState, TuiMode};
pub use crate::components::MessageItem;

// ─── Agent Command (for event injection) ────────────────────────────────────

/// Commands that can be sent from the debug server to the TUI event loop.
#[derive(Debug)]
pub enum AgentCommand {
    /// Inject a key press event
    InjectKey { key: String },
    /// Inject text input
    InjectText { text: String },
    /// Request current state (response sent via channel)
    GetState { response_tx: tokio::sync::oneshot::Sender<SerializableState> },
    /// Request current screen (response sent via channel)
    GetScreen { response_tx: tokio::sync::oneshot::Sender<ScreenCapture> },
}

impl Clone for AgentCommand {
    fn clone(&self) -> Self {
        match self {
            AgentCommand::InjectKey { key } => AgentCommand::InjectKey { key: key.clone() },
            AgentCommand::InjectText { text } => AgentCommand::InjectText { text: text.clone() },
            // These can't really be cloned properly - create dead channels
            AgentCommand::GetState { .. } => {
                let (tx, _rx) = tokio::sync::oneshot::channel();
                AgentCommand::GetState { response_tx: tx }
            }
            AgentCommand::GetScreen { .. } => {
                let (tx, _rx) = tokio::sync::oneshot::channel();
                AgentCommand::GetScreen { response_tx: tx }
            }
        }
    }
}

impl AgentCommand {
    /// Check if this command can be broadcast to multiple receivers
    pub fn is_broadcastable(&self) -> bool {
        matches!(self, AgentCommand::InjectKey { .. } | AgentCommand::InjectText { .. })
    }
}

impl AgentCommand {
    /// Parse a key string into a crossterm KeyEvent
    pub fn parse_key(key: &str) -> Option<crossterm::event::KeyEvent> {
        use crossterm::event::{KeyEvent, KeyEventKind, KeyEventState};

        let lower = key.to_lowercase();
        let (code, modifiers) = parse_key_code(&lower)?;
        Some(KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        })
    }
}

fn parse_key_code(key: &str) -> Option<(crossterm::event::KeyCode, crossterm::event::KeyModifiers)> {
    use crossterm::event::{KeyCode, KeyModifiers};

    let empty = KeyModifiers::empty();
    match key {
        "enter" => (KeyCode::Enter, empty),
        "escape" | "esc" => (KeyCode::Esc, empty),
        "tab" => (KeyCode::Tab, empty),
        "backspace" => (KeyCode::Backspace, empty),
        "delete" | "del" => (KeyCode::Delete, empty),
        "up" => (KeyCode::Up, empty),
        "down" => (KeyCode::Down, empty),
        "left" => (KeyCode::Left, empty),
        "right" => (KeyCode::Right, empty),
        "home" => (KeyCode::Home, empty),
        "end" => (KeyCode::End, empty),
        "pageup" => (KeyCode::PageUp, empty),
        "pagedown" => (KeyCode::PageDown, empty),
        "ctrl+c" | "ctrlc" => (KeyCode::Char('c'), KeyModifiers::CONTROL),
        "ctrl+q" | "ctrlq" => (KeyCode::Char('q'), KeyModifiers::CONTROL),
        "ctrl+j" | "ctrlj" => (KeyCode::Char('j'), KeyModifiers::CONTROL),
        "ctrl+k" | "ctrlk" => (KeyCode::Char('k'), KeyModifiers::CONTROL),
        "ctrl+l" | "ctrll" => (KeyCode::Char('l'), KeyModifiers::CONTROL),
        "ctrl+a" | "ctrla" => (KeyCode::Char('a'), KeyModifiers::CONTROL),
        "ctrl+b" | "ctrlb" => (KeyCode::Char('b'), KeyModifiers::CONTROL),
        "shift+tab" => (KeyCode::Tab, KeyModifiers::SHIFT),
        s if s.starts_with("ctrl+") => {
            let c = s[5..].chars().next()?;
            (KeyCode::Char(c), KeyModifiers::CONTROL)
        }
        s if s.starts_with("shift+") => {
            let c = s[6..].chars().next()?;
            (KeyCode::Char(c), KeyModifiers::SHIFT)
        }
        s if s.starts_with("alt+") => {
            let c = s[4..].chars().next()?;
            (KeyCode::Char(c), KeyModifiers::ALT)
        }
        s if s.len() == 1 => (KeyCode::Char(s.chars().next()?), empty),
        _ => return None,
    }.into()
}

// ─── Serializable State ──────────────────────────────────────────────────────

/// Serializable version of AppState for JSON export.
/// Excludes non-serializable fields like TextArea.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableState {
    pub messages: Vec<SerializableMessageItem>,
    pub input_draft: String,
    pub scroll: SerializableScrollState,
    pub input_history: Vec<String>,
    pub input_history_index: Option<usize>,
    pub agent_running: bool,
    pub current_model: Option<String>,
    pub token_usage: SerializableTokenUsage,
    pub session_token_usage: SerializableTokenUsage,
    pub show_sidebar: bool,
    pub show_thoughts: bool,
    pub terminal_size: (u16, u16),
    pub running: bool,
    pub mock_mode: bool,
    pub mode: String,
    pub permission_modal_open: bool,
    pub permission_modal_tool: Option<String>,
    pub command_palette_open: bool,
    pub thinking_text: Option<String>,
    pub thinking_duration_secs: Option<f32>,
    pub top_bar: SerializableTopBarState,
    pub background_jobs: Vec<SerializableBackgroundJob>,
    pub last_turn_duration_secs: Option<u64>,
    pub last_turn_tokens: Option<usize>,
    pub last_turn_tool_calls: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableMessageItem {
    pub role: String,
    pub text: String,
    pub model: Option<String>,
    pub timestamp: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableScrollState {
    pub feed_offset: usize,
    pub user_scrolled_up: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableTokenUsage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableTopBarState {
    pub repo: String,
    pub branch: String,
    pub path: String,
    pub model: String,
    pub checks_passed: Option<usize>,
    pub checks_total: Option<usize>,
    pub percentage: Option<f32>,
    pub context_pct: Option<f32>,
    pub context_window: Option<usize>,
    pub estimated_tokens: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableBackgroundJob {
    pub name: String,
    pub status: String,
}

impl From<&AppState> for SerializableState {
    fn from(state: &AppState) -> Self {
        let messages = state.messages.iter().map(|m| match m {
            MessageItem::User { text, model, timestamp } => SerializableMessageItem {
                role: "user".to_string(),
                text: text.clone(),
                model: model.clone(),
                timestamp: timestamp.clone(),
            },
            MessageItem::Assistant { text, model, timestamp } => SerializableMessageItem {
                role: "assistant".to_string(),
                text: text.clone(),
                model: model.clone(),
                timestamp: timestamp.clone(),
            },
            MessageItem::Thought { text, duration_secs, .. } => SerializableMessageItem {
                role: "thought".to_string(),
                text: text.clone(),
                model: Some(format!("{:.2}s", duration_secs)),
                timestamp: None,
            },
            MessageItem::ToolCall { name, args, result, is_error } => SerializableMessageItem {
                role: "tool_call".to_string(),
                text: format!("{}({}) -> {:?}", name, args, result.as_ref().map(|s| if s.len() > 100 { &s[..100] } else { s })),
                model: Some(if *is_error { "error".to_string() } else { "ok".to_string() }),
                timestamp: None,
            },
            MessageItem::Edit { filename, diff, .. } => SerializableMessageItem {
                role: "edit".to_string(),
                text: format!("{}: {}", filename, diff.as_ref().map(|d| if d.len() > 100 { &d[..100] } else { d }).unwrap_or("")),
                model: None,
                timestamp: None,
            },
            MessageItem::System { text } => SerializableMessageItem {
                role: "system".to_string(),
                text: text.clone(),
                model: None,
                timestamp: None,
            },
            MessageItem::Error { message, recoverable } => SerializableMessageItem {
                role: "error".to_string(),
                text: message.clone(),
                model: Some(if *recoverable { "recoverable".to_string() } else { "fatal".to_string() }),
                timestamp: None,
            },
            MessageItem::Separator { elapsed_secs, tool_calls, tokens_used } => SerializableMessageItem {
                role: "separator".to_string(),
                text: format!("{}s, {} tools, {} tokens", elapsed_secs, tool_calls, tokens_used.map(|t| t.to_string()).unwrap_or_default()),
                model: None,
                timestamp: None,
            },
            MessageItem::PlanStep { step, text, status } => SerializableMessageItem {
                role: "plan_step".to_string(),
                text: format!("[{}] {}: {}", step, format!("{:?}", status), text),
                model: None,
                timestamp: None,
            },
            MessageItem::Interrupt => SerializableMessageItem {
                role: "interrupt".to_string(),
                text: "---".to_string(),
                model: None,
                timestamp: None,
            },
            MessageItem::Rewind { steps } => SerializableMessageItem {
                role: "rewind".to_string(),
                text: format!("--- rewind {} steps ---", steps),
                model: None,
                timestamp: None,
            },
            MessageItem::ToolRunning { name, args, duration_ms } => SerializableMessageItem {
                role: "tool_running".to_string(),
                text: format!("{}({}) [{}ms]", name, args, duration_ms),
                model: None,
                timestamp: None,
            },
            MessageItem::ToolComplete { name, result, lines } => SerializableMessageItem {
                role: "tool_complete".to_string(),
                text: format!("{}: {} lines", name, lines.map(|l| l.to_string()).unwrap_or_default()),
                model: Some(result.clone()),
                timestamp: None,
            },
        }).collect();

        let (permission_modal_tool, permission_modal_open) = if state.permission_modal.tool.is_some() {
            (state.permission_modal.tool.clone(), true)
        } else {
            (None, false)
        };

        let (thinking_text, thinking_duration_secs) = if let Some(ref thinking) = state.thinking {
            (Some(thinking.text.clone()), Some(thinking.accrued_duration.map(|d| d.as_secs_f32()).unwrap_or(0.0)))
        } else {
            (None, None)
        };

        let mode_str = match state.mode {
            TuiMode::Chat => "Chat",
            TuiMode::Overlay => "Overlay",
            TuiMode::Select => "Select",
            TuiMode::Permission => "Permission",
            TuiMode::CommandPalette => "CommandPalette",
            TuiMode::DiffViewer => "DiffViewer",
            TuiMode::SessionTree => "SessionTree",
            TuiMode::Onboarding => "Onboarding",
        };

        SerializableState {
            messages,
            input_draft: state.input_draft.clone(),
            scroll: SerializableScrollState {
                feed_offset: state.scroll.feed_offset,
                user_scrolled_up: state.scroll.user_scrolled_up,
            },
            input_history: state.input_history.clone(),
            input_history_index: state.input_history_index,
            agent_running: state.agent_running,
            current_model: state.current_model.clone(),
            token_usage: SerializableTokenUsage {
                prompt_tokens: state.token_usage.prompt_tokens,
                completion_tokens: state.token_usage.completion_tokens,
                total_tokens: state.token_usage.total_tokens,
            },
            session_token_usage: SerializableTokenUsage {
                prompt_tokens: state.session_token_usage.prompt_tokens,
                completion_tokens: state.session_token_usage.completion_tokens,
                total_tokens: state.session_token_usage.total_tokens,
            },
            show_sidebar: state.show_sidebar,
            show_thoughts: state.show_thoughts,
            terminal_size: state.terminal_size,
            running: state.running,
            mock_mode: state.mock_mode,
            mode: mode_str.to_string(),
            permission_modal_open,
            permission_modal_tool,
            command_palette_open: state.command_palette.open,
            thinking_text,
            thinking_duration_secs,
            top_bar: SerializableTopBarState {
                repo: state.top_bar.repo.clone(),
                branch: state.top_bar.branch.clone(),
                path: state.top_bar.path.clone(),
                model: state.top_bar.model.clone(),
                checks_passed: state.top_bar.checks_passed,
                checks_total: state.top_bar.checks_total,
                percentage: state.top_bar.percentage,
                context_pct: state.top_bar.context_pct,
                context_window: state.top_bar.context_window,
                estimated_tokens: state.top_bar.estimated_tokens,
            },
            background_jobs: state.background_jobs.iter().map(|j| SerializableBackgroundJob {
                name: j.name.clone(),
                status: format!("{:?}", j.status),
            }).collect(),
            last_turn_duration_secs: state.last_turn_duration_secs,
            last_turn_tokens: state.last_turn_tokens,
            last_turn_tool_calls: state.last_turn_tool_calls,
        }
    }
}

// ─── Screen Capture ──────────────────────────────────────────────────────────

/// Captured screen content as text/JSON
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenCapture {
    pub width: u16,
    pub height: u16,
    pub lines: Vec<ScreenLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenLine {
    pub y: u16,
    pub cells: Vec<ScreenCell>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenCell {
    pub x: u16,
    pub char: String,
    pub fg: Option<String>,
    pub bg: Option<String>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

impl ScreenCapture {
    /// Capture a ratatui Buffer and convert to ScreenCapture
    pub fn from_buffer(buffer: &Buffer) -> Self {
        let width = buffer.area.width;
        let height = buffer.area.height;

        let mut lines: Vec<ScreenLine> = Vec::with_capacity(height as usize);

        for y in 0..height {
            let mut cells: Vec<ScreenCell> = Vec::with_capacity(width as usize);

            for x in 0..width {
                let cell = match buffer.cell((x, y)) {
                    Some(c) => c,
                    None => continue,
                };
                let fg = if cell.fg == ratatui::style::Color::Reset {
                    None
                } else {
                    Some(format!("{:?}", cell.fg))
                };
                let bg = if cell.bg == ratatui::style::Color::Reset {
                    None
                } else {
                    Some(format!("{:?}", cell.bg))
                };

                cells.push(ScreenCell {
                    x,
                    char: cell.symbol().to_string(),
                    fg,
                    bg,
                    bold: cell.modifier.intersects(ratatui::style::Modifier::BOLD),
                    italic: cell.modifier.intersects(ratatui::style::Modifier::ITALIC),
                    underline: cell.modifier.intersects(ratatui::style::Modifier::UNDERLINED),
                });
            }

            lines.push(ScreenLine { y, cells });
        }

        ScreenCapture { width, height, lines }
    }

    /// Convert to plain text (no styles)
    pub fn to_plain_text(&self) -> String {
        let mut result = String::new();
        for line in &self.lines {
            for cell in &line.cells {
                result.push_str(&cell.char);
            }
            result.push('\n');
        }
        result
    }
}

// ─── Debug Server ────────────────────────────────────────────────────────────

/// HTTP debug server for the TUI
pub struct DebugServer {
    port: u16,
    command_tx: broadcast::Sender<AgentCommand>,
}

impl DebugServer {
    /// Create a new debug server on the given port
    pub fn new(port: u16) -> (Self, broadcast::Receiver<AgentCommand>) {
        let (command_tx, command_rx) = broadcast::channel(100);
        (
            DebugServer { port, command_tx },
            command_rx,
        )
    }

    /// Get the command sender to broadcast commands
    pub fn command_sender(&self) -> broadcast::Sender<AgentCommand> {
        self.command_tx.clone()
    }

    /// Start the debug server
    pub async fn start(self, state_arc: Arc<tokio::sync::Mutex<AppState>>) {
        let addr = format!("127.0.0.1:{}", self.port);
        let listener = match TcpListener::bind(&addr).await {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Failed to bind debug server to {}: {}", addr, e);
                return;
            }
        };

        println!("Debug server listening on http://{}", addr);
        println!("  GET  /health  - Health check");
        println!("  GET  /state   - Get current AppState as JSON");
        println!("  GET  /screen  - Get current screen as JSON");
        println!("  POST /inject  - Inject events into the app");

        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    let command_tx = self.command_tx.clone();
                    let state_arc = state_arc.clone();

                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(stream, peer_addr, command_tx, state_arc).await {
                            eprintln!("Error handling connection from {}: {}", peer_addr, e);
                        }
                    });
                }
                Err(e) => {
                    eprintln!("Failed to accept connection: {}", e);
                }
            }
        }
    }
}

async fn handle_connection(
    mut stream: TcpStream,
    _peer_addr: std::net::SocketAddr,
    command_tx: broadcast::Sender<AgentCommand>,
    state_arc: Arc<tokio::sync::Mutex<AppState>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut buffer = [0u8; 8192];
    let n = stream.read(&mut buffer).await?;

    if n == 0 {
        return Ok(());
    }

    let request = String::from_utf8_lossy(&buffer[..n]);
    let lines: Vec<&str> = request.lines().collect();

    if lines.is_empty() {
        return Ok(());
    }

    // Parse request line
    let parts: Vec<&str> = lines[0].split_whitespace().collect();
    if parts.len() < 2 {
        send_error(&mut stream, 400, "Bad Request").await?;
        return Ok(());
    }

    let method = parts[0];
    let path = parts[1];

    let response = match (method, path) {
        ("GET", "/health") => handle_health(),
        ("GET", "/state") => {
            let state = state_arc.lock().await;
            handle_state(&state)
        }
        ("GET", "/screen") => {
            // For screen, we need a channel response
            let (response_tx, response_rx) = tokio::sync::oneshot::channel();
            let _ = command_tx.send(AgentCommand::GetScreen { response_tx });
            match response_rx.await {
                Ok(capture) => handle_screen(&capture),
                Err(_) => handle_error(500, "Timeout getting screen"),
            }
        }
        ("POST", "/inject") => {
            // Extract body from request
            let body = lines.join("\n");
            let json_start = body.find('{');
            let json_end = body.rfind('}').map(|i| i + 1);

            if let (Some(start), Some(end)) = (json_start, json_end) {
                let json_str = &body[start..end];
                match handle_inject(json_str, &command_tx).await {
                    Ok(resp) => resp,
                    Err(e) => handle_error(400, &e.to_string()),
                }
            } else {
                handle_error(400, "Missing JSON body")
            }
        }
        _ => handle_error(404, "Not Found"),
    };

    write_response(&mut stream, response).await?;
    Ok(())
}

fn handle_health() -> HttpResponse {
    HttpResponse {
        status: 200,
        headers: vec![("Content-Type".to_string(), "application/json".to_string())],
        body: r#"{"status":"ok"}"#.to_string(),
    }
}

fn handle_state(state: &AppState) -> HttpResponse {
    let serializable: SerializableState = state.into();
    match serde_json::to_string_pretty(&serializable) {
        Ok(body) => HttpResponse {
            status: 200,
            headers: vec![("Content-Type".to_string(), "application/json".to_string())],
            body,
        },
        Err(e) => handle_error(500, &format!("Serialization error: {}", e)),
    }
}

fn handle_screen(capture: &ScreenCapture) -> HttpResponse {
    match serde_json::to_string_pretty(capture) {
        Ok(body) => HttpResponse {
            status: 200,
            headers: vec![("Content-Type".to_string(), "application/json".to_string())],
            body,
        },
        Err(e) => handle_error(500, &format!("Serialization error: {}", e)),
    }
}

async fn handle_inject(body: &str, command_tx: &broadcast::Sender<AgentCommand>) -> Result<HttpResponse, Box<dyn std::error::Error>> {
    #[derive(Deserialize)]
    struct InjectRequest {
        #[serde(rename = "type")]
        type_: String,
        key: Option<String>,
        text: Option<String>,
    }

    let req: InjectRequest = serde_json::from_str(body)?;

    match req.type_.as_str() {
        "key" => {
            if let Some(key) = req.key {
                let _ = command_tx.send(AgentCommand::InjectKey { key });
                Ok(HttpResponse {
                    status: 200,
                    headers: vec![("Content-Type".to_string(), "application/json".to_string())],
                    body: r#"{"status":"ok","message":"key injected"}"#.to_string(),
                })
            } else {
                Err("Missing 'key' field".into())
            }
        }
        "text" => {
            if let Some(text) = req.text {
                let _ = command_tx.send(AgentCommand::InjectText { text });
                Ok(HttpResponse {
                    status: 200,
                    headers: vec![("Content-Type".to_string(), "application/json".to_string())],
                    body: r#"{"status":"ok","message":"text injected"}"#.to_string(),
                })
            } else {
                Err("Missing 'text' field".into())
            }
        }
        _ => Err(format!("Unknown inject type: {}", req.type_).into()),
    }
}

fn handle_error(status: u16, message: &str) -> HttpResponse {
    HttpResponse {
        status,
        headers: vec![("Content-Type".to_string(), "application/json".to_string())],
        body: serde_json::json!({ "error": message }).to_string(),
    }
}

async fn send_error(stream: &mut TcpStream, status: u16, message: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let response = handle_error(status, message);
    write_response(stream, response).await
}

struct HttpResponse {
    status: u16,
    headers: Vec<(String, String)>,
    body: String,
}

async fn write_response(stream: &mut TcpStream, response: HttpResponse) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let status_text = match response.status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "Unknown",
    };

    let headers_str = response.headers.iter()
        .map(|(k, v)| format!("{}: {}\r\n", k, v))
        .collect::<String>();

    let response_str = format!(
        "HTTP/1.1 {} {}\r\n{}Content-Length: {}\r\n\r\n{}",
        response.status, status_text, headers_str, response.body.len(), response.body
    );

    stream.write_all(response_str.as_bytes()).await?;
    stream.flush().await?;
    Ok(())
}
