# Telemetry

**Status**: done
**Milestone**: R3
**Category**: Configuration

## Description

Opt-in telemetry for crash reporting and usage metrics.

## Architecture

```rust
pub struct Telemetry {
    enabled: bool,
    client: Option<reqwest::Client>,
}

impl Telemetry {
    pub fn track_event(&self, event: &str, props: HashMap<String, String>);
    pub fn track_crash(&self, panic_info: &str);
}
```

## Acceptance Criteria

- [x] Opt-in only — disabled by default
- [x] Config setting: `telemetry.enabled = true`
- [x] Tracks: startup, model switches, tool usage (anonymized)
- [x] Crash reports with stack trace
- [x] No messages/content ever sent
- [x] User can disable at any time

## Tests

### Layer 1
- [x] `telemetry_disabled_by_default` — no events sent
- [x] `telemetry_respects_opt_in` — only sends when enabled
- [x] `telemetry_never_sends_content` — messages not included
