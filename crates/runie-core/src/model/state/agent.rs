use std::collections::VecDeque;

use crate::actors::turn::SpeedWindow;
use crate::model::QueuedMessage;
use crate::streaming_buffer::StreamingBuffer;

/// Agent turn state — queues, tokens, streaming.
/// Fields are public for test setup; production code should use accessors.
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

impl AgentState {
    // Mutable accessors for tests
    pub fn streaming_mut(&mut self) -> &mut bool {
        &mut self.streaming
    }

    pub fn turn_active_mut(&mut self) -> &mut bool {
        &mut self.turn_active
    }

    pub fn turn_started_at_mut(&mut self) -> &mut Option<std::time::Instant> {
        &mut self.turn_started_at
    }

    pub fn current_request_id_mut(&mut self) -> &mut Option<String> {
        &mut self.current_request_id
    }

    pub fn thinking_started_at_mut(&mut self) -> &mut Option<std::time::Instant> {
        &mut self.thinking_started_at
    }
}
