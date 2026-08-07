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

const FIXED_POINT: i32 = 10;
const BASE_MULTIPLIER: i32 = 10;
const MEDIUM_MULTIPLIER: i32 = 16;
const FAST_MULTIPLIER: i32 = 25;

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

    /// Select Grok's conservative wheel profile from terminal metadata.
    /// Multiplexers re-encode mouse streams and therefore use one event per
    /// tick regardless of the outer terminal brand.
    pub fn for_terminal_context(brand: &str, remuxed: bool) -> Self {
        let brand = brand.to_ascii_lowercase();
        let events_per_tick = if remuxed
            || matches!(
                brand.as_str(),
                "wezterm" | "iterm.app" | "vscode" | "cursor" | "windsurf" | "zed"
            ) {
            1
        } else {
            3
        };
        let lines_per_tick = if events_per_tick == 1 { 1 } else { 3 };
        Self::new(events_per_tick, lines_per_tick)
    }

    pub const fn push(self, direction: ScrollDirection) -> (Self, i32) {
        self.push_with_multiplier(direction, BASE_MULTIPLIER)
    }

    const fn push_with_multiplier(
        mut self,
        direction: ScrollDirection,
        multiplier: i32,
    ) -> (Self, i32) {
        let sign = match direction {
            ScrollDirection::Up => -1,
            ScrollDirection::Down => 1,
        };
        self.pending_units += sign * self.lines_per_tick * multiplier;
        let denominator = self.events_per_tick * FIXED_POINT;
        let delta = self.pending_units / denominator;
        self.pending_units %= denominator;
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
            let interval = at_ms.saturating_sub(last);
            let multiplier = if interval < 8 {
                FAST_MULTIPLIER
            } else if interval < 20 {
                MEDIUM_MULTIPLIER
            } else {
                BASE_MULTIPLIER
            };
            self.last_event_ms = Some(at_ms);
            return self.push_with_multiplier(direction, multiplier);
        }
        self.last_event_ms = Some(at_ms);
        self.push_with_multiplier(direction, BASE_MULTIPLIER)
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

    #[test]
    fn injected_intervals_select_grok_acceleration_bands() {
        let normalizer = ScrollNormalizer::default();
        let (normalizer, base) = normalizer.push_at(0, ScrollDirection::Down);
        let (_, medium) = normalizer.push_at(10, ScrollDirection::Down);
        let normalizer = ScrollNormalizer::default();
        let (normalizer, _) = normalizer.push_at(0, ScrollDirection::Down);
        let (_, fast) = normalizer.push_at(5, ScrollDirection::Down);
        assert_eq!(base, 1);
        assert_eq!(medium, 1);
        assert_eq!(fast, 2);
    }

    #[test]
    fn terminal_profiles_follow_grok_event_density_defaults() {
        let remuxed = ScrollNormalizer::for_terminal_context("xterm", true);
        let wezterm = ScrollNormalizer::for_terminal_context("WezTerm", false);
        let unknown = ScrollNormalizer::for_terminal_context("xterm", false);
        assert_eq!(remuxed.push(ScrollDirection::Down).1, 1);
        assert_eq!(wezterm.push(ScrollDirection::Down).1, 1);
        assert_eq!(unknown.push(ScrollDirection::Down).1, 1);
    }
}
