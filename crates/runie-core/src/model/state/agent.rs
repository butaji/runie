//! Agent turn state — a read-only projection of `TurnState`.
//!
//! `AgentState` lives in `AppState` and is the UI-read slice of the authoritative
//! `TurnState` owned by `TurnActor`. Construction is via `From<&TurnState>`.
//!
//! Fields are public for test setup; production code should use the `From<&TurnState>`
//! conversion. Mutable accessors are provided for test compatibility.

use crate::actors::turn::SpeedWindow;
use crate::actors::turn::TurnState;
use crate::model::QueuedMessage;
use crate::streaming_buffer::StreamingBuffer;

/// Agent turn state — a read-only projection of `TurnState`.
///
/// All authoritative fields are copied from `TurnState` via `From<&TurnState>`.
/// UI-only display fields (`tokens_in_display`, etc.) are kept here as they are
/// only needed for the rendering layer.
#[derive(Clone, Default)]
pub struct AgentState {
    // ── Queue management (mirrors TurnState) ───────────────────────────────────
    pub request_queue: std::collections::VecDeque<(String, String)>,
    pub message_queue: Vec<QueuedMessage>,

    // ── Turn lifecycle (mirrors TurnState) ─────────────────────────────────────
    pub current_request_id: Option<String>,
    pub turn_started_at: Option<std::time::Instant>,
    pub turn_active: bool,
    pub inflight: usize,

    // ── Tool execution (mirrors TurnState) ─────────────────────────────────────
    pub current_tool_name: Option<String>,
    pub tool_started_at: Option<std::time::Instant>,
    pub intermediate_step_count: usize,

    // ── Token accounting (mirrors TurnState) ────────────────────────────────────
    pub tokens_in: usize,
    pub tokens_out: usize,
    pub turn_tokens_out: usize,
    pub token_tracker: crate::tokens::TokenTracker,

    // ── Speed tracking (mirrors TurnState) ──────────────────────────────────────
    pub speed_tps: f64,
    pub speed_window: SpeedWindow,
    pub last_speed_update: Option<std::time::Instant>,
    pub tokens_at_last_speed: usize,

    // ── Streaming (mirrors TurnState) ───────────────────────────────────────────
    pub streaming: bool,
    pub streaming_buffer: StreamingBuffer,

    // ── IDs and counters (mirrors TurnState) ──────────────────────────────────
    pub next_id: u64,
    pub current_action: Option<String>,
    pub thought_seq: u64,
    pub last_assistant_index: Option<usize>,
    pub thinking_started_at: Option<std::time::Instant>,

    // ── UI display helpers ─────────────────────────────────────────────────────
    /// Animated display value for tokens_in (smooth interpolation).
    pub tokens_in_display: f64,
    /// Animated display value for tokens_out (smooth interpolation).
    pub tokens_out_display: f64,
    /// Previous token_in value for detecting changes.
    pub tokens_in_prev: usize,
    /// Previous token_out value for detecting changes.
    pub tokens_out_prev: usize,
}

impl From<&TurnState> for AgentState {
    /// Construct a read-only projection of `TurnState`.
    fn from(ts: &TurnState) -> Self {
        Self {
            request_queue: ts.request_queue.clone(),
            message_queue: ts.message_queue.clone(),
            current_request_id: ts.current_request_id.clone(),
            turn_started_at: ts.turn_started_at,
            turn_active: ts.turn_active,
            inflight: ts.inflight,
            current_tool_name: ts.current_tool_name.clone(),
            tool_started_at: ts.tool_started_at,
            intermediate_step_count: ts.intermediate_step_count,
            tokens_in: ts.tokens_in,
            tokens_out: ts.tokens_out,
            turn_tokens_out: ts.turn_tokens_out,
            token_tracker: ts.token_tracker.clone(),
            speed_tps: ts.speed_tps,
            speed_window: ts.speed_window.clone(),
            last_speed_update: ts.last_speed_update,
            tokens_at_last_speed: ts.tokens_at_last_speed,
            streaming: ts.streaming,
            streaming_buffer: ts.streaming_buffer.clone(),
            next_id: ts.next_id,
            current_action: ts.current_action.clone(),
            thought_seq: ts.thought_seq,
            last_assistant_index: ts.last_assistant_index,
            thinking_started_at: ts.thinking_started_at,
            tokens_in_display: ts.tokens_in_display,
            tokens_out_display: ts.tokens_out_display,
            tokens_in_prev: ts.tokens_in_prev,
            tokens_out_prev: ts.tokens_out_prev,
        }
    }
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
