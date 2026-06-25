use std::collections::VecDeque;

use crate::model::QueuedMessage;
use crate::streaming_buffer::StreamingBuffer;

/// Rolling window for speed calculation - tracks last N tokens' arrival times.
#[derive(Clone)]
pub struct SpeedWindow {
    /// Token arrival events: (timestamp, cumulative_token_count_at_arrival)
    /// Using VecDeque for O(1) pop_front.
    events: std::collections::VecDeque<(std::time::Instant, usize)>,
    /// Maximum tokens to track in window
    window_tokens: usize,
}

impl Default for SpeedWindow {
    fn default() -> Self {
        // Default to 1000 token window
        Self {
            events: std::collections::VecDeque::new(),
            window_tokens: 1000,
        }
    }
}

impl SpeedWindow {
    /// Create a new window tracking up to `window_tokens` tokens.
    pub fn new(window_tokens: usize) -> Self {
        Self {
            events: std::collections::VecDeque::new(),
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
        // Find oldest event within window_tokens of current count
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

#[derive(Clone, Default)]
pub struct AgentState {
    pub request_queue: VecDeque<(String, String)>,
    pub message_queue: Vec<QueuedMessage>,
    pub current_request_id: Option<String>,
    pub turn_started_at: Option<std::time::Instant>,
    pub turn_active: bool,
    pub inflight: usize,
    pub current_tool_name: Option<String>,
    pub tool_started_at: Option<std::time::Instant>,
    /// Cumulative input tokens sent to LLM (all turns).
    pub tokens_in: usize,
    /// Cumulative output tokens received from LLM (all turns).
    pub tokens_out: usize,
    /// Output tokens in the current turn (for speed calculation).
    pub turn_tokens_out: usize,
    /// Current streaming speed in tokens/sec (rolling window).
    pub speed_tps: f64,
    /// Rolling window for speed calculation.
    pub speed_window: SpeedWindow,
    /// Last time speed was updated.
    pub last_speed_update: Option<std::time::Instant>,
    /// Token count snapshot at last speed update.
    pub tokens_at_last_speed: usize,
    /// Animated display value for tokens_in (smooth interpolation).
    pub tokens_in_display: f64,
    /// Animated display value for tokens_out (smooth interpolation).
    pub tokens_out_display: f64,
    /// Previous token_in value for detecting changes.
    pub tokens_in_prev: usize,
    /// Previous token_out value for detecting changes.
    pub tokens_out_prev: usize,
    /// Token estimation/cost tracker configured for the active model.
    pub token_tracker: crate::tokens::TokenTracker,
    pub streaming: bool,
    pub next_id: u64,
    pub intermediate_step_count: usize,
    pub current_action: Option<String>,
    pub thought_seq: u64,
    pub last_assistant_index: Option<usize>,
    pub thinking_started_at: Option<std::time::Instant>,
    /// Buffer for streaming response deltas (stable content + mutable tail).
    pub streaming_buffer: StreamingBuffer,
}
