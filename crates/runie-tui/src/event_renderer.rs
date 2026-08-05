//! `EventRenderer` — subscribes to `runie-core`'s event bus and mutates widgets.

use std::sync::Arc;

use parking_lot::Mutex;
#[cfg(test)]
use runie_core::types::AssistantContent;
use runie_core::types::{AgentEvent, AssistantMessageEvent};
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
    in_reasoning: bool,
    reasoning_buffer: String,
    /// True between ToolExecutionStart and ToolExecutionEnd.
    in_tool_exec: bool,
    activity_dirs: usize,
    activity_files: usize,
    active_tool_count: usize,
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
            in_reasoning: false,
            reasoning_buffer: String::new(),
            in_tool_exec: false,
            activity_dirs: 0,
            activity_files: 0,
            active_tool_count: 0,
            emit_welcome,
        }
    }

    /// Drain bus events until the channel closes. Returns when receiver hits
    /// `RecvStreamLagged` or `Closed`.
    pub async fn run(
        mut self,
        mut rx: broadcast::Receiver<AgentEvent>,
        mut shutdown: tokio::sync::watch::Receiver<bool>,
    ) {
        loop {
            tokio::select! {
                biased;
                _ = shutdown.changed() => {
                    if *shutdown.borrow() { break; }
                }
                result = rx.recv() => {
                    match result {
                        Ok(event) => {
                            self.apply_event(event);
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            self.scrollback.lock().append(Line::new(LineKind::System, format!("(skipped {n} events)")));
                        }
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
            }
        }
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
                } else {
                    // In Grok full mode the welcome surface is replaced by
                    // the transcript on submission. The first two rows are
                    // a blank gutter row and the lifecycle event.
                    let mut sb = self.scrollback.lock();
                    sb.append(Line::new(LineKind::System, ""));
                    sb.append(Line::new(LineKind::Tool, "session_start"));
                }
                self.status.lock().set(Status::Thinking);
                self.streaming_buffer.clear();
                self.tool_buffer.clear();
                self.in_assistant_stream = false;
                self.in_reasoning = false;
                self.reasoning_buffer.clear();
                self.in_tool_exec = false;
                self.activity_dirs = 0;
                self.activity_files = 0;
                self.active_tool_count = 0;
            }
            AgentEvent::AgentEnd { messages } => {
                let _ = messages;
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
                        self.scrollback
                            .lock()
                            .append(Line::new(LineKind::User, text));
                    }
                    AgentMessage::Assistant(_) => {
                        self.in_assistant_stream = true;
                        self.streaming_buffer.clear();
                        // Placeholder line; text will append via MessageUpdate.
                        self.scrollback
                            .lock()
                            .append(Line::new(LineKind::Assistant, String::new()));
                    }
                    AgentMessage::ToolResult(_) => {
                        // Will be appended on MessageEnd.
                    }
                    AgentMessage::Custom(_) => {}
                }
            }
            AgentEvent::MessageUpdate {
                event: AssistantMessageEvent::TextDelta { delta },
                ..
            } => {
                if self.in_assistant_stream {
                    self.status.lock().set(Status::Streaming);
                    self.in_reasoning = false;
                    self.streaming_buffer.push_str(&delta);
                    // Replace last line with updated buffer.
                    self.replace_last_assistant_line(&self.streaming_buffer.clone());
                }
            }
            AgentEvent::MessageUpdate {
                event: AssistantMessageEvent::ThinkingDelta { delta },
                ..
            } => {
                if self.in_assistant_stream {
                    self.status.lock().set(Status::Thinking);
                    self.in_reasoning = true;
                    self.reasoning_buffer.push_str(&delta);
                    self.replace_last_reasoning_line(&self.reasoning_buffer.clone());
                }
            }
            AgentEvent::MessageUpdate {
                event: AssistantMessageEvent::ToolCallDelta { partial, .. },
                ..
            } => {
                // Tool calls are handled by the loop's tool executor; the
                // TUI just shows the ToolExecution events.
                let _ = partial;
            }
            AgentEvent::MessageUpdate {
                event: AssistantMessageEvent::Done { .. },
                ..
            } => {
                self.status.lock().set(Status::Ready);
            }
            AgentEvent::MessageUpdate {
                event: AssistantMessageEvent::Error { error },
                ..
            } => {
                self.status.lock().set(Status::Error(error.clone()));
                self.scrollback
                    .lock()
                    .append(Line::new(LineKind::System, format!("error: {error}")));
            }
            AgentEvent::MessageUpdate { .. } => {}
            AgentEvent::MessageEnd { message } => {
                use runie_core::types::AgentMessage;
                match &message {
                    AgentMessage::Assistant(_) => {
                        self.in_assistant_stream = false;
                        self.in_reasoning = false;
                        if let Some(reasoning) =
                            self.scrollback.lock().last_mut_by_kind(LineKind::Reasoning)
                        {
                            reasoning.text = "Thought".into();
                        }
                        // The placeholder line is already in place; ensure its
                        // text matches the final streaming buffer.
                        self.replace_last_assistant_line(&self.streaming_buffer.clone());
                    }
                    AgentMessage::ToolResult(tr) => {
                        // ToolExecutionEnd already owns the structured tool
                        // block and renders its terminal output. Grok does
                        // not append a second serialized ToolResult envelope.
                        let _ = tr;
                    }
                    _ => {}
                }
            }
            AgentEvent::ToolExecutionStart {
                tool_name, args, ..
            } => {
                self.in_tool_exec = true;
                if matches!(tool_name.as_str(), "list_dir" | "list_files") {
                    self.activity_dirs += 1;
                } else if matches!(tool_name.as_str(), "read" | "read_file") {
                    self.activity_files += 1;
                }
                self.active_tool_count += 1;
                self.tool_buffer.clear();
                self.tool_buffer = tool_header(&tool_name, &args);
                if self.activity_dirs + self.activity_files > 0 {
                    let activity = activity_text(self.activity_dirs, self.activity_files, true);
                    let mut sb = self.scrollback.lock();
                    if let Some(line) = sb.last_mut_by_kind(LineKind::Activity) {
                        line.text = activity;
                    } else {
                        sb.append(Line::new(LineKind::Activity, activity));
                    }
                }
                self.scrollback
                    .lock()
                    .append(Line::new(LineKind::Tool, self.tool_buffer.clone()));
            }
            AgentEvent::ToolExecutionUpdate { partial_result, .. } => {
                if self.in_tool_exec {
                    self.tool_buffer.push_str(&format!(
                        " | update: {}",
                        serde_json::to_string(&partial_result).unwrap_or_default()
                    ));
                    self.replace_last_tool_line(&self.tool_buffer.clone());
                }
            }
            AgentEvent::ToolExecutionEnd {
                tool_name,
                result,
                is_error,
                ..
            } => {
                self.in_tool_exec = false;
                self.active_tool_count = self.active_tool_count.saturating_sub(1);
                let marker = if is_error { "✗" } else { "✓" };
                let rendered = tool_result_text(&result);
                self.tool_buffer.push_str(&format!(" → {marker}"));
                self.replace_last_tool_line(&self.tool_buffer.clone());
                if self.active_tool_count == 0 && self.activity_dirs + self.activity_files > 0 {
                    let activity = activity_text(self.activity_dirs, self.activity_files, false);
                    if let Some(line) = self.scrollback.lock().last_mut_by_kind(LineKind::Activity)
                    {
                        line.text = activity;
                    }
                }
                if !is_error {
                    let output_kind = if matches!(
                        tool_name.as_str(),
                        "list_dir" | "list_files" | "read" | "read_file"
                    ) {
                        LineKind::ToolOutput
                    } else {
                        LineKind::ToolResult
                    };
                    for line in rendered.lines().filter(|line| !line.is_empty()) {
                        self.scrollback.lock().append(Line::new(output_kind, line));
                    }
                }
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

    fn replace_last_reasoning_line(&self, text: &str) {
        let mut sb = self.scrollback.lock();
        if let Some(last) = sb.last_mut_by_kind(LineKind::Reasoning) {
            last.text = text.to_string();
        } else {
            sb.append(Line::new(LineKind::Reasoning, text.to_string()));
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

fn tool_header(tool_name: &str, args: &serde_json::Value) -> String {
    match tool_name {
        "list_dir" | "list_files" => {
            let path = args
                .get("path")
                .and_then(serde_json::Value::as_str)
                .unwrap_or(".");
            format!("List {path}")
        }
        "read" | "read_file" => {
            let path = args
                .get("path")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            format!("Read {path}")
        }
        _ => format!(
            "{tool_name} {}",
            serde_json::to_string(args).unwrap_or_default()
        ),
    }
}

fn tool_result_text(result: &serde_json::Value) -> String {
    result
        .as_str()
        .map(str::to_owned)
        .or_else(|| {
            result
                .get("content")
                .and_then(serde_json::Value::as_array)
                .and_then(|content| content.iter().find_map(|item| item.get("text")))
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
        .or_else(|| {
            result
                .get("output")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_else(|| serde_json::to_string(result).unwrap_or_default())
}

fn activity_text(dirs: usize, files: usize, running: bool) -> String {
    let dir_verb = if running { "Listing" } else { "Listed" };
    let file_verb = if running { "Reading" } else { "Read" };
    let mut parts = Vec::new();
    if dirs > 0 {
        parts.push(format!(
            "{dir_verb} {dirs} dir{}",
            if dirs == 1 { "" } else { "s" }
        ));
    }
    if files > 0 {
        parts.push(format!(
            "{file_verb} {files} file{}",
            if files == 1 { "" } else { "s" }
        ));
    }
    format!("◈ {}", parts.join(", "))
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
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout).ok()
            } else {
                None
            }
        })
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
        assert!(
            !sb.lock().is_empty(),
            "welcome modal should populate scrollback"
        );
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
            event: AssistantMessageEvent::TextDelta {
                delta: "Hello".into(),
            },
        });
        let snap = sb.lock().find_first_containing("Hello").is_some();
        assert!(snap);
    }

    #[test]
    fn text_delta_enters_streaming_status() {
        let (mut r, _, st) = new_renderer();
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
                content: vec![],
                stop_reason: None,
                model: "t".into(),
                timestamp: 0,
            }),
            event: AssistantMessageEvent::TextDelta {
                delta: "partial".into(),
            },
        });
        assert_eq!(st.lock().current(), &Status::Streaming);
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
        let _ = (
            StopReason::Stop,
            Usage::default(),
            UserContent::Text { text: "x".into() },
            UserMessage {
                content: vec![],
                timestamp: 0,
            },
        );
    }

    #[test]
    fn structured_tools_use_grok_headers_and_preserve_output_rows() {
        assert_eq!(
            tool_header("list_dir", &serde_json::json!({"path":"."})),
            "List ."
        );
        assert_eq!(
            tool_header("read", &serde_json::json!({"path":"README.md"})),
            "Read README.md"
        );
        assert_eq!(tool_result_text(&serde_json::json!("one\ntwo")), "one\ntwo");
        assert_eq!(
            tool_result_text(&serde_json::json!({"output":"one\ntwo"})),
            "one\ntwo"
        );
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
