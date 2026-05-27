use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use chrono::Utc;
use serde_json;

/// Event stream logger that writes all events to a JSONL file
pub struct EventStreamLogger {
    file: Arc<Mutex<File>>,
}

impl EventStreamLogger {
    pub fn new(runie_dir: &PathBuf) -> Self {
        let events_dir = runie_dir.join("events");
        std::fs::create_dir_all(&events_dir).ok();

        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let path = events_dir.join(format!("events_{}.jsonl", timestamp));

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .expect("Failed to create event stream file");

        tracing::info!("Event stream logging to: {}", path.display());

        Self {
            file: Arc::new(Mutex::new(file)),
        }
    }

    pub fn log_event(&self, event_type: &str, payload: &serde_json::Value) {
        let entry = serde_json::json!({
            "timestamp": Utc::now().to_rfc3339(),
            "type": event_type,
            "payload": payload,
        });

        if let Ok(mut file) = self.file.lock() {
            if let Ok(line) = serde_json::to_string(&entry) {
                writeln!(file, "{}", line).ok();
                file.flush().ok();
            }
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
