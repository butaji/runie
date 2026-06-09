# Telemetry

**Status**: todo
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

- [ ] Opt-in only — disabled by default
- [ ] Config setting: `telemetry.enabled = true`
- [ ] Tracks: startup, model switches, tool usage (anonymized)
- [ ] Crash reports with stack trace
- [ ] No messages/content ever sent
- [ ] User can disable at any time

## Tests

### Layer 1
- [ ] `telemetry_disabled_by_default` — no events sent
- [ ] `telemetry_respects_opt_in` — only sends when enabled
- [ ] `telemetry_never_sends_content` — messages not included
