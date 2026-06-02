//! Comprehensive event logger for Runie.
//! Writes structured event logs to the dev-folder logs directory.

#![allow(dead_code)]

use chrono::Local;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;

/// Global event logger instance
static EVENT_LOGGER: Lazy<Arc<Mutex<Option<EventLogger>>>> = Lazy::new(|| Arc::new(Mutex::new(None)));

/// Log entry level
#[derive(Debug, Clone, Copy)]
pub enum LogLevel {
    DEBUG,
    INFO,
    WARN,
    ERROR,
}

impl LogLevel {
    fn as_str(&self) -> &'static str {
        match self {
            LogLevel::DEBUG => "DEBUG",
            LogLevel::INFO => "INFO",
            LogLevel::WARN => "WARN",
            LogLevel::ERROR => "ERROR",
        }
    }
}

/// Subsystem that generated the log entry
#[derive(Debug, Clone, Copy)]
pub enum Subsystem {
    TUI,
    AGENT,
    PROVIDER,
}

impl Subsystem {
    fn as_str(&self) -> &'static str {
        match self {
            Subsystem::TUI => "TUI",
            Subsystem::AGENT => "AGENT",
            Subsystem::PROVIDER => "PROVIDER",
        }
    }
}

/// Event logger that writes to a file in the logs directory
pub struct EventLogger {
    file: Arc<Mutex<File>>,
}

impl EventLogger {
    /// Create a new event logger that writes to the specified logs directory
    #[must_use]
    pub fn new(logs_dir: &PathBuf) -> std::io::Result<Self> {
        std::fs::create_dir_all(logs_dir)?;

        let timestamp = Local::now().format("%Y%m%d");
        let log_path = logs_dir.join(format!("runie_events_{}.log", timestamp));

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?;

        tracing::info!("Event logger writing to: {}", log_path.display());

        Ok(Self {
            file: Arc::new(Mutex::new(file)),
        })
    }

    /// Log an event with the specified format
    pub fn log(&self, subsystem: Subsystem, level: LogLevel, message: &str) {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let entry = format!("[{}] [{}] [{}] {}\n", timestamp, subsystem.as_str(), level.as_str(), message);

        if let Ok(mut file) = self.file.lock() {
            let _ = file.write_all(entry.as_bytes());
            let _ = file.flush();
        }
    }

    /// Log a mode transition event
    pub fn log_mode_transition(&self, from: &str, to: &str) {
        self.log(Subsystem::TUI, LogLevel::INFO, &format!("{} → {}", from, to));
    }

    /// Log agent_running state change
    pub fn log_agent_running_change(&self, from: bool, to: bool) {
        self.log(Subsystem::TUI, LogLevel::INFO, &format!("agent_running: {} → {}", from, to));
    }

    /// Log message submission
    pub fn log_submit(&self, text_preview: &str) {
        self.log(Subsystem::TUI, LogLevel::INFO, &format!("Submit: \"{}\"", text_preview));
    }

    /// Log permission decision
    pub fn log_permission_decision(&self, tool: &str, decision: &str) {
        self.log(Subsystem::TUI, LogLevel::INFO, &format!("Permission {}: {}", decision, tool));
    }

    /// Log palette command execution
    pub fn log_palette_command(&self, command: &str) {
        self.log(Subsystem::TUI, LogLevel::INFO, &format!("Palette command: {}", command));
    }

    /// Log key event
    pub fn log_key_event(&self, key: &str) {
        self.log(Subsystem::TUI, LogLevel::DEBUG, &format!("Key: {}", key));
    }

    /// Log paste event
    pub fn log_paste(&self, text_len: usize) {
        self.log(Subsystem::TUI, LogLevel::DEBUG, &format!("Paste: {} chars", text_len));
    }

    /// Log resize event
    pub fn log_resize(&self, w: u16, h: u16) {
        self.log(Subsystem::TUI, LogLevel::INFO, &format!("Resize: {}x{}", w, h));
    }

    /// Log error
    pub fn log_error(&self, context: &str, error: &str) {
        self.log(Subsystem::TUI, LogLevel::ERROR, &format!("{}: {}", context, error));
    }

    /// Log agent loop start
    pub fn log_agent_start(&self, provider: &str, model: &str) {
        self.log(Subsystem::AGENT, LogLevel::INFO, &format!("Loop started, provider={}, model={}", provider, model));
    }

    /// Log LLM event
    pub fn log_llm_event(&self, event_type: &str, detail: &str) {
        self.log(Subsystem::AGENT, LogLevel::DEBUG, &format!("{}: {}", event_type, detail));
    }

    /// Log tool call
    pub fn log_tool_call(&self, tool: &str, args_preview: &str) {
        self.log(Subsystem::AGENT, LogLevel::INFO, &format!("{} requested: {}", tool, args_preview));
    }

    /// Log tool result
    pub fn log_tool_result(&self, tool: &str, result_len: usize) {
        self.log(Subsystem::AGENT, LogLevel::INFO, &format!("{} result: {} chars", tool, result_len));
    }

    /// Log agent turn end
    pub fn log_turn_end(&self, turn: usize) {
        self.log(Subsystem::AGENT, LogLevel::INFO, &format!("turn_count={}", turn));
    }

    /// Log agent loop end
    pub fn log_agent_end(&self, total_turns: usize) {
        self.log(Subsystem::AGENT, LogLevel::INFO, &format!("Loop ended, total_turns={}", total_turns));
    }

    /// Log provider creation
    pub fn log_provider_created(&self, provider: &str, model: &str, success: bool) {
        if success {
            self.log(Subsystem::PROVIDER, LogLevel::INFO, &format!("Created: {} ({})", provider, model));
        } else {
            self.log(Subsystem::PROVIDER, LogLevel::ERROR, &format!("Failed to create: {} ({})", provider, model));
        }
    }

    /// Log agent task spawned
    pub fn log_agent_spawned(&self) {
        self.log(Subsystem::AGENT, LogLevel::INFO, "Agent task spawned");
    }

    /// Log agent task completed
    pub fn log_agent_completed(&self) {
        self.log(Subsystem::AGENT, LogLevel::INFO, "Agent task completed");
    }

    /// Log agent task error
    pub fn log_agent_error(&self, error: &str) {
        self.log(Subsystem::AGENT, LogLevel::ERROR, &format!("Agent task error: {}", error));
    }
}

/// Initialize the global event logger
pub fn init_event_logger(logs_dir: &PathBuf) {
    if let Ok(logger) = EventLogger::new(logs_dir) {
        if let Ok(mut global) = EVENT_LOGGER.lock() {
            *global = Some(logger);
        } else {
            tracing::warn!("Event logger mutex poisoned, skipping initialization");
        }
    }
}

/// Get a clone of the logger if available
pub fn get_logger() -> Option<Arc<Mutex<Option<EventLogger>>>> {
    Some(EVENT_LOGGER.clone())
}

/// Convenience function to log via the global logger
pub fn log(subsystem: Subsystem, level: LogLevel, message: &str) {
    if let Some(logger_arc) = get_logger() {
        if let Ok(guard) = logger_arc.lock() {
            if let Some(ref logger) = *guard {
                logger.log(subsystem, level, message);
            }
        }
    }
}

pub fn log_mode_transition(from: &str, to: &str) {
    if let Some(logger_arc) = get_logger() {
        if let Ok(guard) = logger_arc.lock() {
            if let Some(ref logger) = *guard {
                logger.log_mode_transition(from, to);
            }
        }
    }
}

pub fn log_agent_running_change(from: bool, to: bool) {
    if let Some(logger_arc) = get_logger() {
        if let Ok(guard) = logger_arc.lock() {
            if let Some(ref logger) = *guard {
                logger.log_agent_running_change(from, to);
            }
        }
    }
}

pub fn log_submit(text_preview: &str) {
    if let Some(logger_arc) = get_logger() {
        if let Ok(guard) = logger_arc.lock() {
            if let Some(ref logger) = *guard {
                logger.log_submit(text_preview);
            }
        }
    }
}

pub fn log_permission_decision(tool: &str, decision: &str) {
    if let Some(logger_arc) = get_logger() {
        if let Ok(guard) = logger_arc.lock() {
            if let Some(ref logger) = *guard {
                logger.log_permission_decision(tool, decision);
            }
        }
    }
}

pub fn log_palette_command(command: &str) {
    if let Some(logger_arc) = get_logger() {
        if let Ok(guard) = logger_arc.lock() {
            if let Some(ref logger) = *guard {
                logger.log_palette_command(command);
            }
        }
    }
}

pub fn log_key_event(key: &str) {
    if let Some(logger_arc) = get_logger() {
        if let Ok(guard) = logger_arc.lock() {
            if let Some(ref logger) = *guard {
                logger.log_key_event(key);
            }
        }
    }
}

pub fn log_paste(text_len: usize) {
    if let Some(logger_arc) = get_logger() {
        if let Ok(guard) = logger_arc.lock() {
            if let Some(ref logger) = *guard {
                logger.log_paste(text_len);
            }
        }
    }
}

pub fn log_resize(w: u16, h: u16) {
    if let Some(logger_arc) = get_logger() {
        if let Ok(guard) = logger_arc.lock() {
            if let Some(ref logger) = *guard {
                logger.log_resize(w, h);
            }
        }
    }
}

pub fn log_error(context: &str, error: &str) {
    if let Some(logger_arc) = get_logger() {
        if let Ok(guard) = logger_arc.lock() {
            if let Some(ref logger) = *guard {
                logger.log_error(context, error);
            }
        }
    }
}

pub fn log_agent_start(provider: &str, model: &str) {
    if let Some(logger_arc) = get_logger() {
        if let Ok(guard) = logger_arc.lock() {
            if let Some(ref logger) = *guard {
                logger.log_agent_start(provider, model);
            }
        }
    }
}

pub fn log_llm_event(event_type: &str, detail: &str) {
    if let Some(logger_arc) = get_logger() {
        if let Ok(guard) = logger_arc.lock() {
            if let Some(ref logger) = *guard {
                logger.log_llm_event(event_type, detail);
            }
        }
    }
}

pub fn log_tool_call(tool: &str, args_preview: &str) {
    if let Some(logger_arc) = get_logger() {
        if let Ok(guard) = logger_arc.lock() {
            if let Some(ref logger) = *guard {
                logger.log_tool_call(tool, args_preview);
            }
        }
    }
}

pub fn log_tool_result(tool: &str, result_len: usize) {
    if let Some(logger_arc) = get_logger() {
        if let Ok(guard) = logger_arc.lock() {
            if let Some(ref logger) = *guard {
                logger.log_tool_result(tool, result_len);
            }
        }
    }
}

pub fn log_turn_end(turn: usize) {
    if let Some(logger_arc) = get_logger() {
        if let Ok(guard) = logger_arc.lock() {
            if let Some(ref logger) = *guard {
                logger.log_turn_end(turn);
            }
        }
    }
}

pub fn log_agent_end(total_turns: usize) {
    if let Some(logger_arc) = get_logger() {
        if let Ok(guard) = logger_arc.lock() {
            if let Some(ref logger) = *guard {
                logger.log_agent_end(total_turns);
            }
        }
    }
}

pub fn log_provider_created(provider: &str, model: &str, success: bool) {
    if let Some(logger_arc) = get_logger() {
        if let Ok(guard) = logger_arc.lock() {
            if let Some(ref logger) = *guard {
                logger.log_provider_created(provider, model, success);
            }
        }
    }
}

pub fn log_agent_spawned() {
    if let Some(logger_arc) = get_logger() {
        if let Ok(guard) = logger_arc.lock() {
            if let Some(ref logger) = *guard {
                logger.log_agent_spawned();
            }
        }
    }
}

pub fn log_agent_completed() {
    if let Some(logger_arc) = get_logger() {
        if let Ok(guard) = logger_arc.lock() {
            if let Some(ref logger) = *guard {
                logger.log_agent_completed();
            }
        }
    }
}

pub fn log_agent_error(error: &str) {
    if let Some(logger_arc) = get_logger() {
        if let Ok(guard) = logger_arc.lock() {
            if let Some(ref logger) = *guard {
                logger.log_agent_error(error);
            }
        }
    }
}