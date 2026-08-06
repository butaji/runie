//! Clock boundary for live timestamps and deterministic parity captures.

use std::time::{SystemTime, UNIX_EPOCH};

const PARITY_TIMESTAMP_ENV: &str = "RUNIE_PARITY_TIMESTAMP";
const PARITY_ELAPSED_TICKS_ENV: &str = "RUNIE_PARITY_ELAPSED_TICKS";
const PARITY_THINKING_MS_ENV: &str = "RUNIE_PARITY_THINKING_MS";
const PARITY_CLOCK_ENV: &str = "RUNIE_PARITY_CLOCK";
const MILLIS_PER_SECOND: u128 = 1_000;

/// Return the event timestamp used by the live prompt path.
///
/// Production runs use wall-clock time. Capture/replay tools may provide a
/// fixed Unix timestamp through `RUNIE_PARITY_TIMESTAMP`, keeping the clock
/// input outside the pure event projections.
pub fn unix_timestamp_seconds() -> i64 {
    if let Some((timestamp, _)) = parity_clock() {
        return timestamp;
    }
    if let Some(timestamp) = configured_timestamp(std::env::var(PARITY_TIMESTAMP_ENV).ok()) {
        return timestamp;
    }
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() / MILLIS_PER_SECOND)
        .unwrap_or_default() as i64
}

/// Optional deterministic 20Hz status duration for parity captures.
pub fn parity_elapsed_ticks() -> Option<u64> {
    if let Some((_, elapsed_ticks)) = parity_clock() {
        return Some(elapsed_ticks);
    }
    configured_elapsed_ticks(std::env::var(PARITY_ELAPSED_TICKS_ENV).ok())
}

/// Optional deterministic thinking duration for provider replay/capture
/// adapters. The value enters the assistant stream event at the boundary;
/// reducers and views remain unaware of the environment variable.
pub fn parity_thinking_elapsed_ms() -> Option<u64> {
    configured_thinking_elapsed_ms(std::env::var(PARITY_THINKING_MS_ENV).ok())
}

fn parity_clock() -> Option<(i64, u64)> {
    let value = std::env::var(PARITY_CLOCK_ENV).ok()?;
    let (timestamp, elapsed_ticks) = value.split_once(',')?;
    Some((timestamp.parse().ok()?, elapsed_ticks.parse().ok()?))
}

fn configured_timestamp(value: Option<String>) -> Option<i64> {
    value.and_then(|value| value.parse::<i64>().ok())
}

fn configured_elapsed_ticks(value: Option<String>) -> Option<u64> {
    value.and_then(|value| value.parse::<u64>().ok())
}

fn configured_thinking_elapsed_ms(value: Option<String>) -> Option<u64> {
    value.and_then(|value| value.parse::<u64>().ok())
}

#[cfg(test)]
mod tests {
    use super::{configured_elapsed_ticks, configured_thinking_elapsed_ms, configured_timestamp};

    #[test]
    fn parity_timestamp_accepts_only_complete_unix_values() {
        const EXPECTED_TIMESTAMP: i64 = 1_722_988_800;
        assert_eq!(configured_timestamp(Some("1_234".into())), None);
        assert_eq!(
            configured_timestamp(Some(EXPECTED_TIMESTAMP.to_string())),
            Some(EXPECTED_TIMESTAMP)
        );
        assert_eq!(configured_timestamp(Some("not-a-timestamp".into())), None);
        assert_eq!(configured_timestamp(None), None);
    }

    #[test]
    fn parity_elapsed_ticks_rejects_invalid_values() {
        assert_eq!(configured_elapsed_ticks(Some("38".into())), Some(38));
        assert_eq!(configured_elapsed_ticks(Some("-1".into())), None);
        assert_eq!(configured_elapsed_ticks(None), None);
    }

    #[test]
    fn parity_thinking_duration_is_optional_and_numeric() {
        assert_eq!(
            configured_thinking_elapsed_ms(Some("800".into())),
            Some(800)
        );
        assert_eq!(
            configured_thinking_elapsed_ms(Some("not-a-duration".into())),
            None
        );
    }
}
