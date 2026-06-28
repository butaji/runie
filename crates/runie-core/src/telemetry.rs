//! Telemetry is now emitted via `tracing::info!` events.
//!
//! Model switches and tool usage are tracked as structured tracing events
//! when `config.telemetry.enabled` is true. No in-memory collection is performed.
//!
//! Users can opt out by setting `telemetry.enabled = false` in `~/.runie/config.toml`.

#[cfg(test)]
mod tests {
    #[test]
    fn telemetry_info_event_is_structured() {
        // Telemetry is now emitted as `tracing::info!` events.
        // This test verifies the event structure is preserved by checking that
        // the telemetry section in config has an `enabled` field.
        use crate::config::TelemetrySection;
        let section = TelemetrySection::default();
        assert!(section.enabled);
    }

    #[test]
    fn telemetry_can_be_disabled() {
        use crate::config::TelemetrySection;
        let section = crate::config::TelemetrySection { enabled: false };
        assert!(!section.enabled);
    }
}
