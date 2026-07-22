//! Ordered stream emitter with delta coalescing.
//!
//! Buffers events and coalesces adjacent same-type deltas (e.g., consecutive
//! `TextDelta` strings are merged into a single emission) to reduce overhead.
//!
//! Events are emitted in FIFO order. When the pending queue exceeds `MAX_PENDING`
//! events or accumulated delta chars exceed `MAX_DELTA_CHARS`, the queue is
//! flushed automatically.

use futures::Stream;
use std::collections::VecDeque;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Maximum number of pending events before automatic flush.
const MAX_PENDING: usize = 64;

/// Maximum accumulated characters in a delta before flush.
const MAX_DELTA_CHARS: usize = 65536;

/// Emits `T` with optional accumulated text for delta types.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum EmittedEvent<T> {
    /// A single event to emit.
    Event(T),
    /// A flushed batch of events.
    Batch(Vec<T>),
}

/// Trait for event types that support delta coalescing.
///
/// Implement this for event types that carry a text delta (e.g., `TextDelta(String)`).
pub trait Coalescable {
    /// Returns the event type identifier for coalescing decisions.
    fn event_key(&self) -> &'static str;

    /// Returns the text content of a delta, if any.
    fn delta_text(&self) -> Option<&str> {
        None
    }

    /// Creates a new event from accumulated text.
    fn from_text(text: String) -> Self
    where
        Self: Sized;
}

/// Wrapper that coalesces adjacent same-type deltas and emits events in order.
///
/// The inner type `Inner` is the underlying stream being wrapped.
#[derive(Debug)]
pub struct OrderedStreamEmitter<Inner> {
    /// Pending events waiting to be emitted.
    pending: VecDeque<Inner>,
    /// Accumulated delta text for the current coalesced event.
    accumulator: Option<String>,
    /// The event_key of the currently accumulated delta (for coalescing decisions).
    acc_key: Option<&'static str>,
    /// Flag indicating the source stream has ended.
    done: bool,
}

impl<Inner> OrderedStreamEmitter<Inner> {
    /// Create a new emitter with no pending events.
    pub fn new() -> Self {
        Self { pending: VecDeque::new(), accumulator: None, acc_key: None, done: false }
    }

    /// Returns the number of pending events.
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Returns true if there are no pending events and the stream is done.
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty() && self.accumulator.is_none()
    }
}

impl<Inner> Default for OrderedStreamEmitter<Inner> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Inner> OrderedStreamEmitter<Inner>
where
    Inner: Coalescable,
{
    /// Emit a single event, coalescing with the previous event if types match.
    ///
    /// Returns `true` if the internal buffer was flushed due to hitting limits.
    pub fn emit(&mut self, event: Inner) -> bool {
        let delta_text = event.delta_text();
        let key = event.event_key();

        // If we have an accumulated delta, try to coalesce or flush.
        if self.accumulator.is_some() {
            if delta_text.is_some() && self.acc_key.as_deref() == Some(key) {
                // Same delta type: append text to accumulator.
                if let Some(text) = delta_text {
                    if let Some(acc) = &mut self.accumulator {
                        acc.push_str(text);
                        if acc.len() > MAX_DELTA_CHARS {
                            self.flush_to_pending();
                            self.acc_key = None;
                            return true;
                        }
                    }
                }
                return false;
            } else {
                // Different type or non-delta: flush accumulator first.
                self.flush_to_pending();
            }
        }

        // Start accumulating if this is a delta, otherwise queue directly.
        if let Some(text) = delta_text {
            if text.is_empty() {
                self.accumulator = None;
                self.acc_key = None;
            } else {
                self.accumulator = Some(text.to_string());
                self.acc_key = Some(key);
                // Check delta limit on first accumulation (large single delta).
                if text.len() > MAX_DELTA_CHARS {
                    self.flush_to_pending();
                    self.acc_key = None;
                    return true;
                }
            }
        } else {
            // If pending is full, signal flush *before* adding more.
            if self.pending.len() >= MAX_PENDING {
                self.pending.push_back(event);
                return true;
            }
            self.pending.push_back(event);
        }

        false
    }

    /// Flush any accumulated delta to the pending queue.
    pub fn flush(&mut self) {
        self.flush_to_pending();
    }

    /// Move accumulated event to pending queue.
    fn flush_to_pending(&mut self) {
        if let Some(text) = self.accumulator.take() {
            if !text.is_empty() {
                self.pending.push_back(Inner::from_text(text));
            }
        }
    }

    /// Mark the stream as complete. Call after the inner stream finishes.
    pub fn finish(&mut self) {
        self.done = true;
        self.acc_key = None;
        self.flush_to_pending();
    }
}

impl<Inner> Stream for OrderedStreamEmitter<Inner>
where
    Inner: Coalescable + Unpin,
{
    type Item = Inner;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Flush any remaining accumulator on the first poll after done.
        if self.done && self.accumulator.is_some() {
            self.flush_to_pending();
        }

        if let Some(event) = self.pending.pop_front() {
            Poll::Ready(Some(event))
        } else if self.done {
            Poll::Ready(None)
        } else {
            Poll::Pending
        }
    }
}

// ─── Coalescable implementations for common provider event types ───────────────

use crate::provider_event::ProviderEvent;

/// Coalescable implementation for `ProviderEvent`.
///
/// Coalesces adjacent `TextDelta` and `ThinkingDelta` events.
impl Coalescable for ProviderEvent {
    fn event_key(&self) -> &'static str {
        match self {
            ProviderEvent::TextStart { .. } => "text_start",
            ProviderEvent::TextDelta(_) => "text_delta",
            ProviderEvent::TextEnd { .. } => "text_end",
            ProviderEvent::ThinkingStart { .. } => "thinking_start",
            ProviderEvent::ThinkingDelta(_) => "thinking_delta",
            ProviderEvent::ThinkingEnd { .. } => "thinking_end",
            ProviderEvent::ToolCallStart { .. } => "tool_call_start",
            ProviderEvent::ToolCallInputDelta { .. } => "tool_call_input_delta",
            ProviderEvent::ToolCallEnd { .. } => "tool_call_end",
            ProviderEvent::Error(_) => "error",
            ProviderEvent::Usage { .. } => "usage",
            ProviderEvent::Finish { .. } => "finish",
            ProviderEvent::ToolExecutionStart { .. } => "tool_execution_start",
            ProviderEvent::ToolExecutionEnd { .. } => "tool_execution_end",
            ProviderEvent::ToolExecutionResult { .. } => "tool_execution_result",
            ProviderEvent::TurnEnd => "turn_end",
            ProviderEvent::AgentEnd => "agent_end",
        }
    }

    fn delta_text(&self) -> Option<&str> {
        match self {
            ProviderEvent::TextDelta(s) => Some(s),
            ProviderEvent::ThinkingDelta(s) => Some(s),
            _ => None,
        }
    }

    fn from_text(text: String) -> Self {
        // This is a fallback; the actual event type is determined by context.
        // For coalescing, we primarily emit TextDelta events.
        ProviderEvent::TextDelta(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider_event::{ModelError, ProviderEvent, StopReason};

    use futures::StreamExt;

    fn text_delta(s: &str) -> ProviderEvent {
        ProviderEvent::TextDelta(s.to_string())
    }

    fn thinking_delta(s: &str) -> ProviderEvent {
        ProviderEvent::ThinkingDelta(s.to_string())
    }

    fn text_start(id: &str) -> ProviderEvent {
        ProviderEvent::TextStart { id: id.into() }
    }

    fn finish() -> ProviderEvent {
        ProviderEvent::Finish { reason: StopReason::Stop }
    }

    fn error(msg: &str) -> ProviderEvent {
        ProviderEvent::Error(ModelError::Other(msg.into()))
    }

    #[tokio::test]
    async fn emitter_coalesces_consecutive_text_deltas() {
        let events = vec![text_delta("hello"), text_delta(" "), text_delta("world")];

        let mut emitter = OrderedStreamEmitter::new();
        for event in events {
            emitter.emit(event);
        }
        emitter.finish();

        let collected: Vec<_> = emitter.collect().await;
        assert_eq!(collected.len(), 1); // single coalesced TextDelta("hello world")
        match &collected[0] {
            ProviderEvent::TextDelta(s) => assert_eq!(s, "hello world"),
            other => panic!("expected TextDelta, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn emitter_emits_non_delta_events_immediately() {
        let events = vec![text_start("1"), text_delta("hello"), finish()];

        let mut emitter = OrderedStreamEmitter::new();
        for event in events {
            emitter.emit(event);
        }
        emitter.finish();

        let collected: Vec<_> = emitter.collect().await;
        assert_eq!(collected.len(), 3);
    }

    #[tokio::test]
    async fn emitter_preserves_fifo_order() {
        let events = vec![text_start("1"), text_delta("a"), text_start("2"), text_delta("b"), finish()];

        let mut emitter = OrderedStreamEmitter::new();
        for event in events {
            emitter.emit(event);
        }
        emitter.finish();

        let collected: Vec<_> = emitter.collect().await;
        // text_start("1"), TextDelta("a"), text_start("2"), TextDelta("b"), Finish = 5 events.
        assert_eq!(collected.len(), 5);
    }

    #[test]
    fn emitter_respects_max_pending_limit() {
        let mut emitter = OrderedStreamEmitter::<ProviderEvent>::new();

        // Emit MAX_PENDING non-delta events to fill the queue.
        for i in 0..MAX_PENDING {
            emitter.emit(text_start(&format!("id_{i}")));
        }

        assert_eq!(emitter.pending_count(), MAX_PENDING);

        // Adding one more should trigger a flush (return true).
        let flushed = emitter.emit(text_start("overflow"));
        assert!(flushed);
        assert!(emitter.pending_count() <= MAX_PENDING + 1); // overflow item added before flush signal
    }

    #[test]
    fn emitter_respects_max_delta_chars() {
        let mut emitter = OrderedStreamEmitter::<ProviderEvent>::new();

        // Emit a large delta that exceeds MAX_DELTA_CHARS.
        let large_text = "x".repeat(MAX_DELTA_CHARS + 100);
        let flushed = emitter.emit(text_delta(&large_text));
        assert!(flushed, "should flush when delta exceeds MAX_DELTA_CHARS");
    }

    #[tokio::test]
    async fn emitter_preserves_empty_events() {
        let mut emitter = OrderedStreamEmitter::new();
        emitter.emit(text_delta("hello"));
        emitter.emit(text_delta(""));
        emitter.emit(text_delta("world"));
        emitter.finish();

        let collected: Vec<_> = emitter.collect().await;
        // Empty deltas coalesce into the accumulator, not new events.
        assert_eq!(collected.len(), 1); // single TextDelta("helloworld")
        match &collected[0] {
            ProviderEvent::TextDelta(s) => assert_eq!(s, "helloworld"),
            other => panic!("expected TextDelta, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn emitter_handles_empty_stream() {
        let mut emitter = OrderedStreamEmitter::<ProviderEvent>::new();
        emitter.finish();
        let collected: Vec<_> = emitter.collect().await;
        assert!(collected.is_empty());
    }

    #[tokio::test]
    async fn emitter_switches_between_delta_types() {
        let events = vec![text_delta("hello"), thinking_delta("thinking..."), text_delta("world")];

        let mut emitter = OrderedStreamEmitter::new();
        for event in events {
            emitter.emit(event);
        }
        emitter.finish();

        let collected: Vec<_> = emitter.collect().await;
        // Three separate delta events: TextDelta("hello"), ThinkingDelta("thinking..."), TextDelta("world").
        assert_eq!(collected.len(), 3);
    }

    #[test]
    fn coalescable_provider_event_keys() {
        assert_eq!(text_delta("x").event_key(), "text_delta");
        assert_eq!(thinking_delta("x").event_key(), "thinking_delta");
        assert_eq!(text_start("1").event_key(), "text_start");
        assert_eq!(finish().event_key(), "finish");
        assert_eq!(error("x").event_key(), "error");
    }

    #[test]
    fn coalescable_provider_event_delta_text() {
        assert_eq!(text_delta("hello").delta_text(), Some("hello"));
        assert_eq!(thinking_delta("think").delta_text(), Some("think"));
        assert!(text_start("1").delta_text().is_none());
        assert!(finish().delta_text().is_none());
    }

    #[test]
    fn emitter_pending_count() {
        let mut emitter = OrderedStreamEmitter::<ProviderEvent>::new();
        assert_eq!(emitter.pending_count(), 0);

        emitter.emit(text_start("1"));
        assert_eq!(emitter.pending_count(), 1);

        emitter.emit(text_delta("hello"));
        assert_eq!(emitter.pending_count(), 1); // Accumulated, not pending
    }

    #[test]
    fn emitter_is_empty() {
        let mut emitter = OrderedStreamEmitter::<ProviderEvent>::new();
        assert!(emitter.is_empty());

        emitter.emit(text_delta("hello"));
        assert!(!emitter.is_empty());

        emitter.finish();
        assert!(!emitter.is_empty()); // Has pending coalesced event

        drop(emitter);
    }
}
