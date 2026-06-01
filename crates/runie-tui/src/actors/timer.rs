//! TimerActor — emits periodic tick messages for animations.
//!
//! Used for animation frames (cursor blink, streaming cursor, etc.)

use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;

use super::Actor;

/// Timer message — emitted periodically
#[derive(Debug, Clone, Copy)]
pub enum TimerMsg {
    /// Animation tick
    Tick,
}

/// TimerActor emits periodic tick messages.
pub struct TimerActor {
    interval_ms: u64,
}

impl TimerActor {
    /// Create a new TimerActor with the given interval in milliseconds.

    #[must_use]
    #[must_use]
    pub fn new(interval_ms: u64) -> Self {
        Self { interval_ms }
    }
}

impl Actor for TimerActor {
    type Msg = TimerMsg;

    fn name(&self) -> &'static str {
        "timer"
    }

    async fn run(self, msg_tx: mpsc::Sender<TimerMsg>, cancel: CancellationToken) {
        let mut interval = interval(Duration::from_millis(self.interval_ms));

        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    break;
                }
                _ = interval.tick() => {
                    if msg_tx.try_send(TimerMsg::Tick).is_err() {
                        // Channel full, skip this tick
                        tracing::debug!(target: "runie", "[ACTOR:timer] channel full, skipping tick");
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::actors::spawn_actor;

    use super::*;

    #[tokio::test]
    async fn test_timer_actor_emits_ticks() {
        let (handle, mut rx) = spawn_actor(TimerActor::new(10));

        // Wait for at least 2 ticks
        let _ = rx.recv().await;
        let _ = rx.recv().await;

        handle.shutdown();
    }
}
