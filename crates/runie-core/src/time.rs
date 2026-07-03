//! Time helpers.

use std::time::{SystemTime, UNIX_EPOCH};

/// Returns seconds since UNIX epoch as f64.
pub fn unix_now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

/// Returns seconds since UNIX epoch as u64 (truncated).
pub fn unix_now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unix_now_is_positive() {
        let t = unix_now();
        assert!(t > 0.0, "unix_now should be positive, got {}", t);
    }

    #[test]
    fn unix_now_secs_is_positive() {
        let t = unix_now_secs();
        assert!(t > 0, "unix_now_secs should be positive, got {}", t);
    }

    #[test]
    fn unix_now_secs_matches_unix_now_truncated() {
        let f = unix_now();
        let i = unix_now_secs();
        assert_eq!(i, f as u64, "unix_now_secs should equal unix_now as u64");
    }
}
