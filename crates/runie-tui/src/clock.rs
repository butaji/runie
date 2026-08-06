//! Clock boundary for live timestamps and deterministic parity captures.

use std::time::{SystemTime, UNIX_EPOCH};

const PARITY_TIMESTAMP_ENV: &str = "RUNIE_PARITY_TIMESTAMP";
const MILLIS_PER_SECOND: u128 = 1_000;

/// Return the event timestamp used by the live prompt path.
///
/// Production runs use wall-clock time. Capture/replay tools may provide a
/// fixed Unix timestamp through `RUNIE_PARITY_TIMESTAMP`, keeping the clock
/// input outside the pure event projections.
pub fn unix_timestamp_seconds() -> i64 {
    if let Some(timestamp) = configured_timestamp(std::env::var(PARITY_TIMESTAMP_ENV).ok()) {
        return timestamp;
    }
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() / MILLIS_PER_SECOND)
        .unwrap_or_default() as i64
}

fn configured_timestamp(value: Option<String>) -> Option<i64> {
    value.and_then(|value| value.parse::<i64>().ok())
}

#[cfg(test)]
mod tests {
    use super::configured_timestamp;

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
}
