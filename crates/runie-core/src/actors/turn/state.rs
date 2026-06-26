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

/// Rolling window for speed calculation.
#[derive(Clone)]
pub struct SpeedWindow {
    events: VecDeque<(std::time::Instant, usize)>,
    window_tokens: usize,
}

impl Default for SpeedWindow {
    fn default() -> Self {
        Self::new(1000)
    }
}

impl SpeedWindow {
    pub fn new(window_tokens: usize) -> Self {
        Self { events: VecDeque::new(), window_tokens }
    }

    pub fn record(&mut self, token_count: usize) {
        let now = std::time::Instant::now();
        self.events.push_back((now, token_count));
        self.evict_old();
    }

    fn evict_old(&mut self) {
        if self.events.len() <= 1 {
            return;
        }
        let Some((_, latest)) = self.events.back() else { return };
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

    pub fn clear(&mut self) {
        self.events.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

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
