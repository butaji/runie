//! Shared speed-window type used by both TurnState and AgentState.
//!
//! Uses `ringbuffer::AllocRingBuffer` for efficient O(1) push/pop without
//! reallocation. Stores `(elapsed_ns, token_count)` tuples; `Instant` is
//! converted to/from `u64` nanoseconds at the record boundary.

use ringbuffer::{AllocRingBuffer, RingBuffer};

/// Rolling window for speed calculation — tracks last N tokens' arrival times.
#[derive(Clone)]
pub struct SpeedWindow {
    /// Token arrival events: (elapsed_ns_since_first_event, cumulative_token_count_at_arrival).
    /// Using `AllocRingBuffer` for O(1) enqueue/dequeue without reallocation.
    events: AllocRingBuffer<(u64, usize)>,
    /// Maximum tokens to track in window (configurable).
    window_tokens: usize,
    /// First recorded instant, used as the reference for elapsed_ns.
    first_instant: Option<std::time::Instant>,
}

impl Default for SpeedWindow {
    fn default() -> Self {
        Self::new(1000)
    }
}

impl SpeedWindow {
    /// Create a new window tracking up to `window_tokens` tokens.
    /// Uses a generous internal capacity of 4096 to handle bursts.
    pub fn new(window_tokens: usize) -> Self {
        Self {
            // 4096 is large enough for any reasonable burst while staying bounded.
            events: AllocRingBuffer::new(4096),
            window_tokens,
            first_instant: None,
        }
    }

    /// Record tokens arriving at the current time.
    pub fn record(&mut self, token_count: usize) {
        let now = std::time::Instant::now();
        let elapsed_ns = if let Some(first) = self.first_instant {
            now.duration_since(first).as_nanos() as u64
        } else {
            self.first_instant = Some(now);
            0
        };
        self.events.enqueue((elapsed_ns, token_count));
        self.evict_old();
    }

    /// Remove events outside the window based on token count.
    fn evict_old(&mut self) {
        if self.events.len() <= 1 {
            return;
        }
        let Some((_, latest_token)) = self.events.back() else {
            return;
        };
        let cutoff = latest_token.saturating_sub(self.window_tokens);
        // Drain events from the front while they are below the cutoff.
        while self.events.len() > 1 {
            if let Some((_, count)) = self.events.front() {
                if *count < cutoff {
                    self.events.dequeue();
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
        let (start_ns, start_tok) = *self.events.get(0).unwrap();
        let (end_ns, end_tok) = *self.events.get_signed(-1).unwrap();
        if start_tok == end_tok {
            return 0.0;
        }
        let elapsed = (end_ns as f64 - start_ns as f64) / 1_000_000_000.0;
        if elapsed < 0.001 {
            return 0.0;
        }
        (end_tok - start_tok) as f64 / elapsed
    }

    /// Clear the window.
    pub fn clear(&mut self) {
        self.events.clear();
        self.first_instant = None;
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
