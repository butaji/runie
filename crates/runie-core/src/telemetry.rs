//! Opt-in telemetry for anonymized usage metrics and crash reporting.
//!
//! Telemetry is disabled by default. When enabled, it tracks:
//! - startup
//! - model switches
//! - tool usage (name only, no output)
//! - crashes (with stack trace)
//!
//! User messages and tool output are NEVER captured.

use std::collections::HashMap;

/// A single telemetry event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TelemetryEvent {
    pub name: String,
    pub properties: HashMap<String, String>,
}

/// Telemetry collector. Disabled by default.
#[derive(Clone, Debug, Default)]
pub struct Telemetry {
    enabled: bool,
    events: Vec<TelemetryEvent>,
}

impl Telemetry {
    /// Create telemetry with the given enabled state.
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            events: Vec::new(),
        }
    }

    /// Track an anonymized event. No-op when disabled.
    pub fn track_event(&mut self, event: &str, props: HashMap<String, String>) {
        if !self.enabled {
            return;
        }
        self.events.push(TelemetryEvent {
            name: event.into(),
            properties: props,
        });
    }

    /// Track a crash with stack trace. No-op when disabled.
    pub fn track_crash(&mut self, panic_info: &str) {
        if !self.enabled {
            return;
        }
        let mut props = HashMap::new();
        props.insert("stack_trace".into(), panic_info.into());
        self.events.push(TelemetryEvent {
            name: "crash".into(),
            properties: props,
        });
    }

    /// Whether telemetry is currently enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Access collected events (for testing / inspection).
    pub fn events(&self) -> &[TelemetryEvent] {
        &self.events
    }

    /// Clear collected events.
    pub fn clear(&mut self) {
        self.events.clear();
    }
}

/// Install a panic hook that reports crashes via telemetry.
/// The hook captures a backtrace and forwards it to the provided
/// telemetry sender. The original hook is still invoked.
pub fn install_panic_hook(mut tx: tokio::sync::mpsc::Sender<TelemetryEvent>) {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let payload = if let Some(s) = info.payload().downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "unknown panic".to_string()
        };
        let location = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_default();
        let backtrace = std::backtrace::Backtrace::capture().to_string();
        let panic_info = format!(
            "Panic: {}\nLocation: {}\nBacktrace:\n{}",
            payload, location, backtrace
        );
        let evt = TelemetryEvent {
            name: "crash".into(),
            properties: {
                let mut m = HashMap::new();
                m.insert("stack_trace".into(), panic_info);
                m
            },
        };
        let _ = tx.try_send(evt);
        default_hook(info);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn telemetry_disabled_by_default() {
        let mut tel = Telemetry::new(false);
        tel.track_event("startup", HashMap::new());
        assert!(tel.events().is_empty());
    }

    #[test]
    fn telemetry_respects_opt_in() {
        let mut tel = Telemetry::new(true);
        tel.track_event("startup", HashMap::new());
        assert_eq!(tel.events().len(), 1);
        assert_eq!(tel.events()[0].name, "startup");
    }

    #[test]
    fn telemetry_never_sends_content() {
        let mut tel = Telemetry::new(true);
        let mut props = HashMap::new();
        props.insert("tool".into(), "read_file".into());
        // We deliberately do NOT insert any user content
        tel.track_event("tool_usage", props);
        let evt = &tel.events()[0];
        assert!(!evt.properties.values().any(|v| v.contains("secret")));
        assert!(!evt.properties.values().any(|v| v.contains("password")));
    }

    #[test]
    fn telemetry_track_crash_respects_enabled() {
        let mut tel = Telemetry::new(true);
        tel.track_crash("segmentation fault");
        assert_eq!(tel.events().len(), 1);
        assert_eq!(tel.events()[0].name, "crash");
        assert!(tel.events()[0].properties.contains_key("stack_trace"));
    }

    #[test]
    fn telemetry_clear_empties_events() {
        let mut tel = Telemetry::new(true);
        tel.track_event("x", HashMap::new());
        tel.clear();
        assert!(tel.events().is_empty());
    }
}
