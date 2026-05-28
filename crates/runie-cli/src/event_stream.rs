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
    pub fn new(runie_dir: &PathBuf) -> Self {
        let events_dir = runie_dir.join("events");

        // Create directory with proper error handling (don't silently ignore failures)
        if let Err(e) = std::fs::create_dir_all(&events_dir) {
            tracing::warn!("Failed to create events directory: {}. Files will be created in current dir.", e);
        }

        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let jsonl_path = events_dir.join(format!("events_{}.jsonl", timestamp));
        let log_path = events_dir.join(format!("events_{}.log", timestamp));

        // Create JSONL file with proper error handling
        let jsonl_file = match OpenOptions::new()
            .create(true)
            .append(true)
            .open(&jsonl_path)
        {
            Ok(f) => f,
            Err(e) => {
                tracing::error!("Failed to create JSONL file {}: {}", jsonl_path.display(), e);
                // Fall back to creating in current directory
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(format!("events_{}.jsonl", timestamp))
                    .expect("Failed to create fallback JSONL file")
            }
        };

        // Create log file with proper error handling
        let log_file = match OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
        {
            Ok(f) => f,
            Err(e) => {
                tracing::error!("Failed to create log file {}: {}", log_path.display(), e);
                // Fall back to creating in current directory
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(format!("events_{}.log", timestamp))
                    .expect("Failed to create fallback log file")
            }
        };

        tracing::info!("Event stream logging to: {} and {}", jsonl_path.display(), log_path.display());

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

        // Write machine-readable JSONL
        if let Ok(mut file) = self.jsonl_file.lock() {
            if let Ok(line) = serde_json::to_string(&entry) {
                writeln!(file, "{}", line).ok();
                file.flush().ok();
            }
        }

        // Write human-readable log
        if let Ok(mut file) = self.log_file.lock() {
            let readable = Self::format_human_readable(event_type, payload);
            writeln!(file, "{}", readable).ok();
            file.flush().ok();
        }
    }

    fn format_human_readable(event_type: &str, payload: &serde_json::Value) -> String {
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S");
        let tag = match event_type {
            "agent" => "[AGENT]",
            "ui" => "[UI]",
            _ => "[EVENT]",
        };

        // Check for error conditions in agent events
        if event_type == "agent" {
            if let Some(msg_obj) = payload.get("message") {
                if let Some(err_msg) = msg_obj.get("error_message").and_then(|v| v.as_str()) {
                    if !err_msg.is_empty() {
                        let type_str = payload.get("type").and_then(|v| v.as_str()).unwrap_or("Unknown");
                        return format!("[{}] [{}] [ERROR] {} - {}",
                            timestamp, tag, type_str, err_msg);
                    }
                }
            }
            // Handle Error events directly
            if let Some(err_msg) = payload.get("message").and_then(|v| v.as_str()) {
                if !err_msg.is_empty() {
                    let err_type = payload.get("error_type").and_then(|v| v.as_str()).unwrap_or("error");
                    return format!("[{}] [{}] [ERROR] {} - {}", timestamp, tag, err_type, err_msg);
                }
            }
        }

        // Format based on event type and payload
        match event_type {
            "ui" => {
                let msg = payload.get("msg").and_then(|v| v.as_str()).unwrap_or("unknown");
                format!("[{}] {} [INFO] {}", timestamp, tag, msg)
            }
            "agent" => {
                let type_str = payload.get("type").and_then(|v| v.as_str()).unwrap_or("Unknown");
                let turn = payload.get("turn").and_then(|v| v.as_u64()).unwrap_or(0);

                // Extract meaningful info based on event type
                let detail = Self::extract_agent_detail(payload, type_str);
                if detail.is_empty() {
                    format!("[{}] {} [INFO] {} (turn {})", timestamp, tag, type_str, turn)
                } else {
                    format!("[{}] {} [INFO] {} - {} (turn {})", timestamp, tag, type_str, detail, turn)
                }
            }
            _ => {
                // Fallback to JSON payload summary
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
        }
    }

    fn extract_agent_detail(payload: &serde_json::Value, event_type: &str) -> String {
        match event_type {
            "MessageStart" | "MessageUpdate" | "MessageEnd" => {
                // Extract content preview from message
                if let Some(msg) = payload.get("message") {
                    if let Some(content) = msg.get("content").and_then(|v| v.as_array()) {
                        for part in content {
                            if let Some(text) = part.get("text").and_then(|v| v.as_str()) {
                                if !text.is_empty() {
                                    let preview = if text.len() > 80 { format!("{}...", &text[..80]) } else { text.to_string() };
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
            "ToolExecutionStart" => {
                let tool_name = payload.get("tool_name").and_then(|v| v.as_str()).unwrap_or("?");
                let args_preview = payload.get("tool_args").and_then(|v| v.as_str())
                    .map(|a| if a.len() > 50 { format!("{}...", &a[..50]) } else { a.to_string() })
                    .unwrap_or_default();
                if args_preview.is_empty() {
                    tool_name.to_string()
                } else {
                    format!("{} with {}", tool_name, args_preview)
                }
            }
            "ToolExecutionEnd" => {
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
            "TurnEnd" => {
                let msg_count = payload.get("message_count").and_then(|v| v.as_u64()).unwrap_or(0);
                let tool_count = payload.get("tool_results_count").and_then(|v| v.as_u64()).unwrap_or(0);
                format!("{} messages, {} tool results", msg_count, tool_count)
            }
            "AgentEnd" => {
                let total_turns = payload.get("total_turns").and_then(|v| v.as_u64()).unwrap_or(0);
                format!("completed {} turns", total_turns)
            }
            "PermissionRequest" => {
                let tool_name = payload.get("tool_name").and_then(|v| v.as_str()).unwrap_or("?");
                let description = payload.get("tool_description").and_then(|v| v.as_str())
                    .map(|d| if d.len() > 60 { format!("{}...", &d[..60]) } else { d.to_string() })
                    .unwrap_or_default();
                if description.is_empty() {
                    format!("[WARN] {} requires permission", tool_name)
                } else {
                    format!("[WARN] {} requires permission: {}", tool_name, description)
                }
            }
            "PermissionGranted" | "PermissionDenied" => {
                let tool_name = payload.get("tool_name").and_then(|v| v.as_str()).unwrap_or("?");
                let decision = if event_type == "PermissionGranted" { "allowed" } else { "denied" };
                format!("{} ({})", tool_name, decision)
            }
            "Error" => {
                let msg = payload.get("message").and_then(|v| v.as_str()).unwrap_or("unknown error");
                let ctx = payload.get("context").and_then(|v| v.as_str()).unwrap_or("");
                if ctx.is_empty() {
                    format!("[ERROR] {}", msg)
                } else {
                    format!("[ERROR] {} - {}", msg, ctx)
                }
            }
            _ => String::new(),
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
