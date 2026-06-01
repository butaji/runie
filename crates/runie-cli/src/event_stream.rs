use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use chrono::Utc;
use serde_json;

/// Event stream logger that writes events to both JSONL (machine-readable) and human-readable log files
pub struct EventStreamLogger {
    jsonl_file: Arc<Mutex<File>>,
    log_file: Arc<Mutex<File>>,
}

impl EventStreamLogger {

    #[must_use]
    pub fn new(runie_dir: &PathBuf) -> Self {
        let events_dir = runie_dir.join("events");
        create_events_dir(&events_dir);

        let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let jsonl_file = open_log_file(&events_dir, &timestamp, "jsonl");
        let log_file = open_log_file(&events_dir, &timestamp, "log");

        tracing::info!("Event stream logging to: {} and {}",
            events_dir.join(format!("events_{}.jsonl", timestamp)).display(),
            events_dir.join(format!("events_{}.log", timestamp)).display());

        Self {
            jsonl_file: Arc::new(Mutex::new(jsonl_file)),
            log_file: Arc::new(Mutex::new(log_file)),
        }
    }

    pub fn log_event(&self, event_type: &str, payload: &serde_json::Value) {
        let entry = serde_json::json!({
            "timestamp": Utc::now().to_rfc3339(),
            "type": event_type,
            "payload": payload,
        });

        if let Ok(mut file) = self.jsonl_file.lock() {
            if let Ok(line) = serde_json::to_string(&entry) {
                writeln!(file, "{}", line).ok();
                file.flush().ok();
            }
        }

        if let Ok(mut file) = self.log_file.lock() {
            let readable = Self::format_human_readable(event_type, payload);
            writeln!(file, "{}", readable).ok();
            file.flush().ok();
        }
    }

    fn format_human_readable(event_type: &str, payload: &serde_json::Value) -> String {
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let tag = Self::event_tag(event_type);

        if event_type == "agent" {
            if let Some(msg) = Self::extract_error_message(payload, &timestamp, tag) {
                return msg;
            }
        }

        match event_type {
            "ui" => Self::format_ui_event(&timestamp, tag),
            "agent" => Self::format_agent_event(&timestamp, tag, payload),
            _ => Self::format_fallback_event(&timestamp, tag, payload),
        }
    }

    fn event_tag(event_type: &str) -> &'static str {
        match event_type {
            "agent" => "[AGENT]",
            "ui" => "[UI]",
            _ => "[EVENT]",
        }
    }

    fn extract_error_message(payload: &serde_json::Value, timestamp: &str, tag: &str) -> Option<String> {
        if let Some(msg_obj) = payload.get("message") {
            if let Some(err_msg) = msg_obj.get("error_message").and_then(|v| v.as_str()) {
                if !err_msg.is_empty() {
                    let type_str = payload.get("type").and_then(|v| v.as_str()).unwrap_or("Unknown");
                    return Some(format!("[{}] [{}] [ERROR] {} - {}", timestamp, tag, type_str, err_msg));
                }
            }
        }

        if let Some(err_msg) = payload.get("message").and_then(|v| v.as_str()) {
            if !err_msg.is_empty() {
                let err_type = payload.get("error_type").and_then(|v| v.as_str()).unwrap_or("error");
                return Some(format!("[{}] [{}] [ERROR] {} - {}", timestamp, tag, err_type, err_msg));
            }
        }

        None
    }

    fn format_ui_event(timestamp: &str, tag: &str) -> String {
        format!("[{}] {} [INFO] unknown ui event", timestamp, tag)
    }

    fn format_agent_event(timestamp: &str, tag: &str, payload: &serde_json::Value) -> String {
        let type_str = payload.get("type").and_then(|v| v.as_str()).unwrap_or("Unknown");
        let turn = payload.get("turn").and_then(|v| v.as_u64()).unwrap_or(0);

        let detail = Self::extract_agent_detail(payload, type_str);
        if detail.is_empty() {
            format!("[{}] {} [INFO] {} (turn {})", timestamp, tag, type_str, turn)
        } else {
            format!("[{}] {} [INFO] {} - {} (turn {})", timestamp, tag, type_str, detail, turn)
        }
    }

    fn format_fallback_event(timestamp: &str, tag: &str, payload: &serde_json::Value) -> String {
        let summary = if let Ok(s) = serde_json::to_string(payload) {
            if s.len() > 100 {
                format!("{}...", &s[..100])
            } else {
                s
            }
        } else {
            "?".to_string()
        };
        format!("[{}] {} [INFO] {}", timestamp, tag, summary)
    }

    fn extract_agent_detail(payload: &serde_json::Value, event_type: &str) -> String {
        // Message events - same handler
        if matches!(event_type, "MessageStart" | "MessageUpdate" | "MessageEnd") {
            return Self::extract_message_detail(payload);
        }
        // Tool events
        if event_type == "ToolExecutionStart" {
            return Self::extract_tool_start_detail(payload);
        }
        if event_type == "ToolExecutionEnd" {
            return Self::extract_tool_end_detail(payload);
        }
        // Turn/Agent lifecycle
        if event_type == "TurnEnd" {
            return Self::extract_turn_end_detail(payload);
        }
        if event_type == "AgentEnd" {
            return Self::extract_agent_end_detail(payload);
        }
        // Permission events
        if event_type == "PermissionRequest" {
            return Self::extract_permission_request_detail(payload);
        }
        if matches!(event_type, "PermissionGranted" | "PermissionDenied") {
            return Self::extract_permission_result_detail(payload, event_type);
        }
        // Error
        if event_type == "Error" {
            return Self::extract_error_detail(payload);
        }
        String::new()
    }

    fn extract_message_detail(payload: &serde_json::Value) -> String {
        if let Some(msg) = payload.get("message") {
            if let Some(content) = msg.get("content").and_then(|v| v.as_array()) {
                for part in content {
                    if let Some(text) = part.get("text").and_then(|v| v.as_str()) {
                        if !text.is_empty() {
                            let preview = Self::truncate_text(text, 80);
                            return format!("content: {}", preview);
                        }
                    }
                    if let Some(tool_name) = part.get("name").and_then(|v| v.as_str()) {
                        return format!("tool: {}", tool_name);
                    }
                }
            }
        }
        String::new()
    }

    fn extract_tool_start_detail(payload: &serde_json::Value) -> String {
        let tool_name = payload.get("tool_name").and_then(|v| v.as_str()).unwrap_or("?");
        let args_preview = payload.get("tool_args").and_then(|v| v.as_str())
            .map(|a| Self::truncate_text(a, 50))
            .unwrap_or_default();
        if args_preview.is_empty() {
            tool_name.to_string()
        } else {
            format!("{} with {}", tool_name, args_preview)
        }
    }

    fn extract_tool_end_detail(payload: &serde_json::Value) -> String {
        let tool_name = payload.get("tool_name").and_then(|v| v.as_str()).unwrap_or("?");
        let is_error = payload.get("result")
            .and_then(|r| r.get("is_error"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if is_error {
            let content = payload.get("result")
                .and_then(|r| r.get("content"))
                .and_then(|c| c.as_array())
                .and_then(|arr| arr.first())
                .and_then(|p| p.get("text"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            format!("{} [ERROR: {}]", tool_name, content)
        } else {
            tool_name.to_string()
        }
    }

    fn extract_turn_end_detail(payload: &serde_json::Value) -> String {
        let msg_count = payload.get("message_count").and_then(|v| v.as_u64()).unwrap_or(0);
        let tool_count = payload.get("tool_results_count").and_then(|v| v.as_u64()).unwrap_or(0);
        format!("{} messages, {} tool results", msg_count, tool_count)
    }

    fn extract_agent_end_detail(payload: &serde_json::Value) -> String {
        let total_turns = payload.get("total_turns").and_then(|v| v.as_u64()).unwrap_or(0);
        format!("completed {} turns", total_turns)
    }

    fn extract_permission_request_detail(payload: &serde_json::Value) -> String {
        let tool_name = payload.get("tool_name").and_then(|v| v.as_str()).unwrap_or("?");
        let description = payload.get("tool_description").and_then(|v| v.as_str())
            .map(|d| Self::truncate_text(d, 60))
            .unwrap_or_default();
        if description.is_empty() {
            format!("[WARN] {} requires permission", tool_name)
        } else {
            format!("[WARN] {} requires permission: {}", tool_name, description)
        }
    }

    fn extract_permission_result_detail(payload: &serde_json::Value, event_type: &str) -> String {
        let tool_name = payload.get("tool_name").and_then(|v| v.as_str()).unwrap_or("?");
        let decision = if event_type == "PermissionGranted" { "allowed" } else { "denied" };
        format!("{} ({})", tool_name, decision)
    }

    fn extract_error_detail(payload: &serde_json::Value) -> String {
        let msg = payload.get("message").and_then(|v| v.as_str()).unwrap_or("unknown error");
        let ctx = payload.get("context").and_then(|v| v.as_str()).unwrap_or("");
        if ctx.is_empty() {
            format!("[ERROR] {}", msg)
        } else {
            format!("[ERROR] {} - {}", msg, ctx)
        }
    }

    fn truncate_text(text: &str, max_len: usize) -> String {
        if text.len() > max_len {
            format!("{}...", &text[..max_len])
        } else {
            text.to_string()
        }
    }

    pub fn log_agent_event(&self, event: &runie_agent::events::AgentEvent) {
        let payload = serde_json::to_value(event).unwrap_or(serde_json::json!({"error": "serialization failed"}));
        self.log_event("agent", &payload);
    }

    pub fn log_ui_event(&self, msg: &str) {
        self.log_event("ui", &serde_json::json!({"msg": msg}));
    }
}

// ─── Helper functions ────────────────────────────────────────────────────────────

fn create_events_dir(events_dir: &std::path::Path) {
    if let Err(e) = std::fs::create_dir_all(events_dir) {
        tracing::warn!("Failed to create events directory: {}. Files will be created in current dir.", e);
    }
}

fn open_log_file(events_dir: &std::path::Path, timestamp: &str, ext: &str) -> File {
    let path = events_dir.join(format!("events_{}.{}", timestamp, ext));
    let fallback = format!("events_{}.{}", timestamp, ext);

    match OpenOptions::new().create(true).append(true).open(&path) {
        Ok(f) => f,
        Err(e) => {
            tracing::error!("Failed to create {} file {}: {}", ext, path.display(), e);
            match OpenOptions::new().create(true).append(true).open(&fallback) {
                Ok(f) => f,
                Err(e2) => {
                    tracing::error!("Failed to create fallback log file {}: {}", fallback, e2);
                    match open_null_device() {
                        Ok(f) => f,
                        Err(e3) => {
                            tracing::error!(
                                "Cannot open null device ({}). Creating temp file as last resort.",
                                e3
                            );
                            match tempfile_in_tmp() {
                                Ok(f) => f,
                                Err(e4) => {
                                    tracing::error!(
                                        "All log sinks failed. Dropping events silently. errors: fallback={} null={} tmp={}",
                                        e2, e3, e4
                                    );
                                    // All sinks exhausted. Return a no-op file in /dev/null
                                    // since the previous attempt failed. If THIS fails too,
                                    // we have no choice but to surface the error rather than
                                    // panic the entire CLI. The events subsystem will be inert.
                                    OpenOptions::new()
                                        .write(true)
                                        .open("/dev/null")
                                        .unwrap_or_else(|_| {
                                            // On systems without /dev/null, write to a file in
                                            // the working dir which we then immediately unlink.
                                            // If even THIS fails, the world is broken.
                                            std::fs::File::create("runie-null-sink.tmp")
                                                .unwrap_or_else(|_| {
                                                    // Last-resort: create the file but don't
                                                    // unwrap. If this fails too, return a dummy
                                                    // File handle from a closed memfd. Cannot
                                                    // construct one without unsafe, so we open
                                                    // /dev/null one more time; if that fails the
                                                    // OS is unsalvageable.
                                                    #[allow(clippy::expect_used)]
                                                    OpenOptions::new()
                                                        .write(true)
                                                        .open("/dev/null")
                                                        .expect("OS cannot open any sink file")
                                                })
                                        })
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn tempfile_in_tmp() -> std::io::Result<File> {
    let mut path = std::env::temp_dir();
    path.push(format!("runie-events-{}.log", std::process::id()));
    OpenOptions::new().create(true).append(true).open(&path)
}

#[cfg(unix)]
fn open_null_device() -> std::io::Result<File> {
    OpenOptions::new().write(true).open("/dev/null")
}

#[cfg(not(unix))]
fn open_null_device() -> std::io::Result<File> {
    OpenOptions::new().write(true).open("NUL")
}
