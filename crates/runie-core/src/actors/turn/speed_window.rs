//! Shared speed-window type used by both TurnState and AgentState.

use std::collections::VecDeque;

/// Rolling window for speed calculation — tracks last N tokens' arrival times.
#[derive(Clone)]
pub struct SpeedWindow {
    /// Token arrival events: (timestamp, cumulative_token_count_at_arrival).
    /// Using VecDeque for O(1) pop_front.
    events: VecDeque<(std::time::Instant, usize)>,
    /// Maximum tokens to track in window.
    window_tokens: usize,
}

impl Default for SpeedWindow {
    fn default() -> Self {
        Self::new(1000)
    }
}

impl SpeedWindow {
    /// Create a new window tracking up to `window_tokens` tokens.
    pub fn new(window_tokens: usize) -> Self {
        Self {
            events: VecDeque::new(),
            window_tokens,
        }
    }

    /// Record tokens arriving at the current time.
    pub fn record(&mut self, token_count: usize) {
        let now = std::time::Instant::now();
        self.events.push_back((now, token_count));
        self.evict_old();
    }

    /// Remove events outside the window.
    fn evict_old(&mut self) {
        if self.events.len() <= 1 {
            return;
        }
        let Some((_, latest)) = self.events.back() else {
            return;
        };
        let cutoff = latest.saturating_sub(self.window_tokens);
        while self.events.len() > 1 {
            if let Some((_, count)) = self.events.front() {
                if *count < cutoff {
                    self.events.pop_front();
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }

    /// Calculate tokens/sec based on the rolling window.
    /// Returns 0.0 if not enough data.
    pub fn speed(&self) -> f64 {
        if self.events.len() < 2 {
            return 0.0;
        }
        let (start, start_tok) = self.events.front().unwrap();
        let (end, end_tok) = self.events.back().unwrap();
        if start_tok == end_tok {
            return 0.0;
        }
        let elapsed = end.duration_since(*start).as_secs_f64();
        if elapsed < 0.001 {
            return 0.0;
        }
        (end_tok - start_tok) as f64 / elapsed
    }

    /// Clear the window.
    pub fn clear(&mut self) {
        self.events.clear();
    }

    /// Number of events in window.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// True if window is empty.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}
