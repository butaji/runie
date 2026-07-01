//! Turn state owned by TurnActor.
//!
//! This module defines the authoritative turn state that was previously scattered
//! across `AgentState`. The turn state includes:
//! - Turn lifecycle flags (active, streaming)
//! - Request/message queues
//! - Token accounting and speed tracking
//! - Streaming buffer
//! - UI-only display fields (tokens_in_display, token_tracker, etc.)
//!
//! `AgentState` (in `AppState`) is a read-only projection of `TurnState`.
//! Use `From<&TurnState>` to construct it.

use std::collections::VecDeque;

use crate::model::QueuedMessage;
use crate::streaming_buffer::StreamingBuffer;
use crate::tokens::TokenTracker;

pub use super::speed_window::SpeedWindow;


/// Authoritative turn state owned by TurnActor.
///
/// Contains all fields needed for both authoritative turn lifecycle management
/// and the UI projection (`AgentState`). Fields are public for test setup;
/// production code should use the `From<&TurnState>` conversion for the UI projection.
#[derive(Clone)]
pub struct TurnState {
    // ── Queue management ────────────────────────────────────────────────────────
    pub request_queue: VecDeque<(String, String)>,
    pub message_queue: Vec<QueuedMessage>,

    // ── Turn lifecycle ─────────────────────────────────────────────────────────
    pub current_request_id: Option<String>,
    pub turn_started_at: Option<std::time::Instant>,
    pub turn_active: bool,
    pub inflight: usize,

    // ── Tool execution ─────────────────────────────────────────────────────────
    pub current_tool_name: Option<String>,
    pub tool_started_at: Option<std::time::Instant>,
    pub intermediate_step_count: usize,

    // ── Token accounting ───────────────────────────────────────────────────────
    /// Cumulative input tokens sent to LLM (all turns).
    pub tokens_in: usize,
    /// Cumulative output tokens received from LLM (all turns).
    pub tokens_out: usize,
    /// Output tokens in the current turn (for speed calculation).
    pub turn_tokens_out: usize,
    /// Token estimation/cost tracker configured for the active model.
    pub token_tracker: TokenTracker,

    // ── Speed tracking ─────────────────────────────────────────────────────────
    pub speed_tps: f64,
    pub speed_window: SpeedWindow,
    pub last_speed_update: Option<std::time::Instant>,
    pub tokens_at_last_speed: usize,

    // ── Streaming ─────────────────────────────────────────────────────────────
    pub streaming: bool,
    pub streaming_buffer: StreamingBuffer,

    // ── IDs and counters ──────────────────────────────────────────────────────
    pub next_id: u64,
    pub current_action: Option<String>,
    pub thought_seq: u64,
    pub last_assistant_index: Option<usize>,
    pub thinking_started_at: Option<std::time::Instant>,

    // ── UI display helpers (read by AgentState projection) ─────────────────────
    /// Animated display value for tokens_in (smooth interpolation).
    pub tokens_in_display: f64,
    /// Animated display value for tokens_out (smooth interpolation).
    pub tokens_out_display: f64,
    /// Previous token_in value for detecting changes.
    pub tokens_in_prev: usize,
    /// Previous token_out value for detecting changes.
    pub tokens_out_prev: usize,
}

impl Default for TurnState {
    fn default() -> Self {
        Self {
            request_queue: VecDeque::new(),
            message_queue: Vec::new(),
            current_request_id: None,
            turn_started_at: None,
            turn_active: false,
            inflight: 0,
            current_tool_name: None,
            tool_started_at: None,
            intermediate_step_count: 0,
            tokens_in: 0,
            tokens_out: 0,
            turn_tokens_out: 0,
            token_tracker: TokenTracker::new(),
            speed_tps: 0.0,
            speed_window: SpeedWindow::default(),
            last_speed_update: None,
            tokens_at_last_speed: 0,
            streaming: false,
            streaming_buffer: StreamingBuffer::default(),
            next_id: 0,
            current_action: None,
            thought_seq: 0,
            last_assistant_index: None,
            thinking_started_at: None,
            tokens_in_display: 0.0,
            tokens_out_display: 0.0,
            tokens_in_prev: 0,
            tokens_out_prev: 0,
        }
    }
}

impl TurnState {
    pub fn peek_queue(&self) -> Option<&(String, String)> {
        self.request_queue.front()
    }

    pub fn pop_queue(&mut self) -> Option<(String, String)> {
        self.request_queue.pop_front()
    }

    pub fn is_active(&self) -> bool {
        self.turn_active
    }

    pub fn start_turn(&mut self) {
        self.turn_active = true;
        self.inflight += 1;
    }

    pub fn stop_turn(&mut self) {
        self.turn_active = false;
        self.current_request_id = None;
        self.streaming = false;
        self.current_tool_name = None;
        self.current_action = None;
        self.inflight = self.inflight.saturating_sub(1);
        self.turn_started_at = None;
        self.thinking_started_at = None;
        self.tool_started_at = None;
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}
