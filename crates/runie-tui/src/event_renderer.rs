//! `EventRenderer` — subscribes to `runie-core`'s event bus and mutates widgets.

use std::sync::Arc;

use parking_lot::Mutex;
use runie_core::types::{
    AgentEvent, AssistantContent, AssistantMessageEvent, StopReason,
};
use tokio::sync::broadcast;

use crate::widgets::{Line, LineKind, Scrollback, Status, StatusBar};

pub struct EventRenderer {
    pub scrollback: Arc<Mutex<Scrollback>>,
    pub status: Arc<Mutex<StatusBar>>,
    /// Accumulated text while an assistant message is streaming.
    streaming_buffer: String,
    /// Accumulated tool args/lines while a tool is executing.
    tool_buffer: String,
    /// True between MessageStart(assistant) and MessageEnd(assistant).
    in_assistant_stream: bool,
    /// True between ToolExecutionStart and ToolExecutionEnd.
    in_tool_exec: bool,
    /// If true, the next AgentStart emits the welcome modal lines
    /// (matching grok's minimal-mode chrome) and then clears this flag.
    emit_welcome: bool,
}

impl EventRenderer {
    pub fn new(scrollback: Arc<Mutex<Scrollback>>, status: Arc<Mutex<StatusBar>>) -> Self {
        Self::with_welcome(scrollback, status, true)
    }

    pub fn with_welcome(
        scrollback: Arc<Mutex<Scrollback>>,
        status: Arc<Mutex<StatusBar>>,
        emit_welcome: bool,
    ) -> Self {
        Self {
            scrollback,
            status,
            streaming_buffer: String::new(),
            tool_buffer: String::new(),
            in_assistant_stream: false,
            in_tool_exec: false,
            emit_welcome,
        }
    }

    /// Drain bus events until the channel closes. Returns when receiver hits
    /// `RecvStreamLagged` or `Closed`.
    pub async fn run(mut self, mut rx: broadcast::Receiver<AgentEvent>, mut shutdown: tokio::sync::watch::Receiver<bool>) {
        eprintln!("[renderer] run start");
        loop {
            tokio::select! {
                biased;
                _ = shutdown.changed() => {
                    if *shutdown.borrow() { eprintln!("[renderer] shutdown"); break; }
                }
                result = rx.recv() => {
                    match result {
                        Ok(event) => {
                            eprintln!("[renderer] rx: {:?}", event);
                            self.apply_event(event);
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            let mut sb = self.scrollback.lock();
                            sb.append(Line::new(LineKind::System, format!("(skipped {n} events)")));
                        }
                        Err(broadcast::error::RecvError::Closed) => { eprintln!("[renderer] closed"); break; }
                    }
                }
            }
        }
        eprintln!("[renderer] run end");
    }

    pub fn apply_event(&mut self, event: AgentEvent) {
        match event {
            AgentEvent::AgentStart => {
                // Emit the welcome modal *before* clearing so the very first
                // run has both the welcome block and any subsequent
                // transcript in the scrollback.
                if self.emit_welcome {
                    self.emit_welcome_modal();
                    self.emit_welcome = false;
                }
                self.status.lock().set(Status::Thinking);
                self.streaming_buffer.clear();
                self.tool_buffer.clear();
                self.in_assistant_stream = false;
                self.in_tool_exec = false;
            }
            AgentEvent::AgentEnd { messages } => {
                let mut sb = self.scrollback.lock();
                sb.append(Line::new(LineKind::System, format!("(run finished, {} new messages)", messages.len())));
                drop(sb);
                self.status.lock().set(Status::Ready);
            }
            AgentEvent::TurnStart => {
                self.status.lock().set(Status::Thinking);
            }
            AgentEvent::TurnEnd { .. } => {
                self.status.lock().set(Status::Ready);
            }
            AgentEvent::MessageStart { message } => {
                use runie_core::types::AgentMessage;
                let mut sb = self.scrollback.lock();
                match &message {
                    AgentMessage::User(u) => {
                        let text = u
                            .content
                            .iter()
                            .map(|c| match c {
                                runie_core::types::UserContent::Text { text } => text.as_str(),
                                runie_core::types::UserContent::Image { .. } => "[image]",
                            })
                            .collect::<Vec<_>>()
                            .join("");
                        sb.append(Line::new(LineKind::User, text));
                    }
                    AgentMessage::Assistant(_) => {
                        self.in_assistant_stream = true;
                        self.streaming_buffer.clear();
                        // Placeholder line; text will append via MessageUpdate.
                        sb.append(Line::new(LineKind::Assistant, String::new()));
                    }
                    AgentMessage::ToolResult(_) => {
                        // Will be appended on MessageEnd.
                    }
                    AgentMessage::Custom(_) => {}
                }
            }
            AgentEvent::MessageUpdate { event: AssistantMessageEvent::TextDelta { delta }, .. } => {
                if self.in_assistant_stream {
                    self.streaming_buffer.push_str(&delta);
                    // Replace last line with updated buffer.
                    self.replace_last_assistant_line(&self.streaming_buffer.clone());
                }
            }
            AgentEvent::MessageUpdate { event: AssistantMessageEvent::ThinkingDelta { delta }, .. } => {
                if self.in_assistant_stream {
                    self.streaming_buffer.push_str(&format!("[think]{delta}[/think]"));
                    self.replace_last_assistant_line(&self.streaming_buffer.clone());
                }
            }
            AgentEvent::MessageUpdate { event: AssistantMessageEvent::ToolCallDelta { partial, .. }, .. } => {
                // Tool calls are handled by the loop's tool executor; the
                // TUI just shows the ToolExecution events.
                let _ = partial;
            }
            AgentEvent::MessageUpdate { event: AssistantMessageEvent::Done { .. }, .. } => {
                self.status.lock().set(Status::Ready);
            }
            AgentEvent::MessageUpdate { event: AssistantMessageEvent::Error { error }, .. } => {
                self.status.lock().set(Status::Error(error));
            }
            AgentEvent::MessageUpdate { .. } => {}
            AgentEvent::MessageEnd { message } => {
                use runie_core::types::AgentMessage;
                match &message {
                    AgentMessage::Assistant(_) => {
                        self.in_assistant_stream = false;
                        // The placeholder line is already in place; ensure its
                        // text matches the final streaming buffer.
                        self.replace_last_assistant_line(&self.streaming_buffer.clone());
                    }
                    AgentMessage::ToolResult(tr) => {
                        let mut sb = self.scrollback.lock();
                        let text = tr
                            .content
                            .iter()
                            .map(|c| match c {
                                runie_core::types::ToolResultContent::Text { text } => text.as_str(),
                                runie_core::types::ToolResultContent::Image { .. } => "[image]",
                            })
                            .collect::<Vec<_>>()
                            .join("");
                        let prefix = if tr.is_error { "  ✗ " } else { "  ↳ " };
                        sb.append(Line::new(LineKind::ToolResult, format!("{prefix}{text}")));
                    }
                    _ => {}
                }
            }
            AgentEvent::ToolExecutionStart { tool_name, args, .. } => {
                self.in_tool_exec = true;
                self.tool_buffer.clear();
                self.tool_buffer.push_str(&format!("{tool_name} {}", serde_json::to_string(&args).unwrap_or_default()));
                let mut sb = self.scrollback.lock();
                sb.append(Line::new(LineKind::Tool, self.tool_buffer.clone()));
            }
            AgentEvent::ToolExecutionUpdate { partial_result, .. } => {
                if self.in_tool_exec {
                    self.tool_buffer.push_str(&format!(" | update: {}", serde_json::to_string(&partial_result).unwrap_or_default()));
                    self.replace_last_tool_line(&self.tool_buffer.clone());
                }
            }
            AgentEvent::ToolExecutionEnd { tool_name, result, is_error, .. } => {
                self.in_tool_exec = false;
                let marker = if is_error { "✗" } else { "✓" };
                self.tool_buffer.push_str(&format!(" → {marker} {}", serde_json::to_string(&result).unwrap_or_default()));
                self.replace_last_tool_line(&self.tool_buffer.clone());
                let _ = tool_name;
            }
        }
    }

    fn replace_last_assistant_line(&self, text: &str) {
        let mut sb = self.scrollback.lock();
        if let Some(last) = sb.lines_mut_last_assistant() {
            last.text = text.to_string();
        } else {
            sb.append(Line::new(LineKind::Assistant, text.to_string()));
        }
    }

    fn replace_last_tool_line(&self, text: &str) {
        let mut sb = self.scrollback.lock();
        if let Some(last) = sb.lines_mut_last_tool() {
            last.text = text.to_string();
        } else {
            sb.append(Line::new(LineKind::Tool, text.to_string()));
        }
    }

    /// Emit the welcome-modal lines (matches grok-build's minimal-mode chrome).
    /// Called once on the first `AgentStart` to seed the transcript with the
    /// version/cwd/model block, the event-log entry, and the hint line.
    fn emit_welcome_modal(&mut self) {
        for line in welcome_modal_lines() {
            self.scrollback.lock().append(line);
        }
    }
}

/// Pure function: returns the welcome-modal lines (matches grok-build's
/// minimal-mode chrome). Adopts grok's `insta::assert_snapshot!` pattern:
/// the function is a pure formatter, the test pins its output to a snapshot.
pub fn welcome_modal_lines() -> Vec<Line> {
    let cwd = std::env::current_dir()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "runie".into());
    let branch = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()
        .and_then(|o| if o.status.success() { String::from_utf8(o.stdout).ok() } else { None })
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "main".into());
    let version = env!("CARGO_PKG_VERSION");
    vec![
        Line::new(LineKind::System, format!("╭─ Runie  v{version} ─")),
        Line::new(LineKind::System, format!("│ {branch} {cwd}")),
        Line::new(LineKind::System, String::from("│ Model · runie-core")),
        Line::new(LineKind::System, String::from("│ /help for commands")),
        Line::new(LineKind::System, String::from("╰─")),
        Line::new(LineKind::System, String::from("◆ session_start")),
    ]
}

// Extension methods on Scrollback for last-line replacement. Kept here to
// avoid touching the widget from the renderer module.
trait ScrollbackExt {
    fn lines_mut_last_assistant(&mut self) -> Option<&mut Line>;
    fn lines_mut_last_tool(&mut self) -> Option<&mut Line>;
}
impl ScrollbackExt for Scrollback {
    fn lines_mut_last_assistant(&mut self) -> Option<&mut Line> {
        self.last_mut_by_kind(LineKind::Assistant)
    }
    fn lines_mut_last_tool(&mut self) -> Option<&mut Line> {
        self.last_mut_by_kind(LineKind::Tool)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use runie_core::types::{AgentMessage, StopReason, Usage, UserContent, UserMessage};

    fn new_renderer() -> (EventRenderer, Arc<Mutex<Scrollback>>, Arc<Mutex<StatusBar>>) {
        let sb = Arc::new(Mutex::new(Scrollback::new()));
        let st = Arc::new(Mutex::new(StatusBar::new()));
        (EventRenderer::new(sb.clone(), st.clone()), sb, st)
    }

    #[test]
    fn agent_start_emits_welcome_and_sets_thinking() {
        let (mut r, sb, st) = new_renderer();
        // Pre-seed a stale line. AgentStart now emits the welcome modal
        // instead of clearing (matches grok's minimal-mode chrome where
        // the welcome block persists across runs).
        r.apply_event(AgentEvent::AgentStart);
        assert!(!sb.lock().is_empty(), "welcome modal should populate scrollback");
        assert!(sb.lock().find_first_containing("Runie").is_some());
        assert_eq!(st.lock().current(), &Status::Thinking);
    }

    #[test]
    fn message_update_appends_text_to_assistant_line() {
        let (mut r, sb, _) = new_renderer();
        r.apply_event(AgentEvent::AgentStart);
        r.apply_event(AgentEvent::MessageStart {
            message: AgentMessage::Assistant(runie_core::types::AssistantMessage {
                content: vec![],
                stop_reason: None,
                model: "t".into(),
                timestamp: 0,
            }),
        });
        r.apply_event(AgentEvent::MessageUpdate {
            message: AgentMessage::Assistant(runie_core::types::AssistantMessage {
                content: vec![AssistantContent::Text { text: "hi".into() }],
                stop_reason: None,
                model: "t".into(),
                timestamp: 0,
            }),
            event: AssistantMessageEvent::TextDelta { delta: "Hello".into() },
        });
        let snap = sb.lock().find_first_containing("Hello").is_some();
        assert!(snap);
    }

    #[test]
    fn agent_end_sets_ready() {
        let (mut r, _, st) = new_renderer();
        r.apply_event(AgentEvent::AgentStart);
        r.apply_event(AgentEvent::AgentEnd { messages: vec![] });
        assert_eq!(st.lock().current(), &Status::Ready);
    }

    #[test]
    fn tool_execution_lifecycle() {
        let (mut r, sb, _) = new_renderer();
        r.apply_event(AgentEvent::AgentStart);
        r.apply_event(AgentEvent::ToolExecutionStart {
            tool_call_id: "1".into(),
            tool_name: "bash".into(),
            args: serde_json::json!({"cmd": "ls"}),
        });
        r.apply_event(AgentEvent::ToolExecutionEnd {
            tool_call_id: "1".into(),
            tool_name: "bash".into(),
            result: serde_json::json!({"ok": true}),
            is_error: false,
        });
        let lines = sb.lock();
        assert!(lines.find_first_containing("bash").is_some());
        assert!(lines.find_first_containing("✓").is_some());
        let _ = (StopReason::Stop, Usage::default(), UserContent::Text { text: "x".into() }, UserMessage { content: vec![], timestamp: 0 });
    }

    /// Pure-function snapshot (adopted from grok-build's `insta` pattern).
    /// The welcome modal is a deterministic formatter; the test pins its
    /// text to a saved snapshot so accidental layout drift gets caught.
    #[test]
    fn welcome_modal_snapshot() {
        let text: String = super::welcome_modal_lines()
            .iter()
            .map(|l| l.text.clone())
            .collect::<Vec<_>>()
            .join("\n");
        insta::assert_snapshot!("welcome_modal", text);
    }
}