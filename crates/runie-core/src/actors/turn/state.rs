//! Turn state owned by TurnActor.
//!
//! This module defines the authoritative turn state that was previously scattered
//! across `AgentState`. The turn state includes:
//! - Turn lifecycle flags (active, streaming)
//! - Request/message queues
//! - Token accounting and speed tracking
//! - Streaming buffer

use std::collections::VecDeque;

use crate::model::QueuedMessage;
use crate::streaming_buffer::StreamingBuffer;

pub use super::speed_window::SpeedWindow;


/// Authoritative turn state owned by TurnActor.
#[derive(Clone, Default)]
pub struct TurnState {
    pub request_queue: VecDeque<(String, String)>,
    pub message_queue: Vec<QueuedMessage>,
    pub current_request_id: Option<String>,
    pub turn_started_at: Option<std::time::Instant>,
    pub turn_active: bool,
    pub inflight: usize,
    pub current_tool_name: Option<String>,
    pub tool_started_at: Option<std::time::Instant>,
    pub tokens_in: usize,
    pub tokens_out: usize,
    pub turn_tokens_out: usize,
    pub speed_tps: f64,
    pub speed_window: SpeedWindow,
    pub last_speed_update: Option<std::time::Instant>,
    pub tokens_at_last_speed: usize,
    pub streaming: bool,
    pub next_id: u64,
    pub intermediate_step_count: usize,
    pub current_action: Option<String>,
    pub thought_seq: u64,
    pub last_assistant_index: Option<usize>,
    pub thinking_started_at: Option<std::time::Instant>,
    pub streaming_buffer: StreamingBuffer,
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
