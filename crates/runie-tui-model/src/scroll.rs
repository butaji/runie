//! Pure normalization of terminal wheel events before actor delivery.

/// Direction of one raw terminal wheel event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollDirection {
    Up,
    Down,
}

/// Converts raw events into whole-line deltas without terminal or clock state.
/// Grok's default is three lines per three raw events; retaining the remainder
/// makes non-default profiles deterministic as well.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollNormalizer {
    events_per_tick: i32,
    lines_per_tick: i32,
    stream_gap_ms: u64,
    last_event_ms: Option<u64>,
    pending_units: i32,
}

impl Default for ScrollNormalizer {
    fn default() -> Self {
        Self::new(3, 3)
    }
}

impl ScrollNormalizer {
    pub const fn new(events_per_tick: i32, lines_per_tick: i32) -> Self {
        Self {
            events_per_tick: if events_per_tick > 0 {
                events_per_tick
            } else {
                1
            },
            lines_per_tick: if lines_per_tick > 0 {
                lines_per_tick
            } else {
                1
            },
            stream_gap_ms: 80,
            last_event_ms: None,
            pending_units: 0,
        }
    }

    pub const fn push(mut self, direction: ScrollDirection) -> (Self, i32) {
        let sign = match direction {
            ScrollDirection::Up => -1,
            ScrollDirection::Down => 1,
        };
        self.pending_units += sign * self.lines_per_tick;
        let delta = self.pending_units / self.events_per_tick;
        self.pending_units %= self.events_per_tick;
        (self, delta)
    }

    /// Push an event at an injected monotonic millisecond timestamp. A gap
    /// larger than Grok's stream boundary starts a fresh gesture; replay can
    /// exercise this without wall-clock access or test sleeps.
    pub const fn push_at(mut self, at_ms: u64, direction: ScrollDirection) -> (Self, i32) {
        if let Some(last) = self.last_event_ms {
            if at_ms.saturating_sub(last) > self.stream_gap_ms {
                self.pending_units = 0;
            }
        }
        self.last_event_ms = Some(at_ms);
        self.push(direction)
    }
}

#[cfg(test)]
mod tests {
    use super::{ScrollDirection, ScrollNormalizer};

    #[test]
    fn grok_default_emits_three_lines_across_three_raw_events() {
        let mut normalizer = ScrollNormalizer::default();
        let mut deltas = Vec::new();
        for _ in 0..3 {
            let (next, delta) = normalizer.push(ScrollDirection::Down);
            normalizer = next;
            deltas.push(delta);
        }
        assert_eq!(deltas, [1, 1, 1]);
    }

    #[test]
    fn custom_ratio_preserves_fractional_remainder() {
        let mut normalizer = ScrollNormalizer::new(3, 5);
        let (next, first) = normalizer.push(ScrollDirection::Down);
        normalizer = next;
        let (next, second) = normalizer.push(ScrollDirection::Down);
        normalizer = next;
        let (next, third) = normalizer.push(ScrollDirection::Down);
        normalizer = next;
        assert_eq!([first, second, third], [1, 2, 2]);
        assert_eq!(normalizer.pending_units, 0);
    }

    #[test]
    fn stream_gap_resets_fractional_carry_without_sleeping() {
        let normalizer = ScrollNormalizer::new(3, 5);
        let (normalizer, _) = normalizer.push_at(0, ScrollDirection::Down);
        let (normalizer, _) = normalizer.push_at(20, ScrollDirection::Down);
        let (normalizer, after_gap) = normalizer.push_at(101, ScrollDirection::Down);
        assert_eq!(after_gap, 1);
        assert_eq!(normalizer.last_event_ms, Some(101));
    }
}
