//! Pure normalization of terminal wheel events before actor delivery.

/// Direction of one raw terminal wheel event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollDirection {
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollMode {
    Auto,
    Wheel,
    Trackpad,
}

/// Result of one explicit Grok cadence flush.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollFlush {
    pub lines: i32,
    pub backlog: i32,
}

/// Result of ending a stream without an uncapped catch-up burst.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollFinalize {
    pub flushed: i32,
    pub dropped: i32,
    pub backlog: i32,
}

/// Pure cadence-owned scroll state. Raw input accumulates whole-line movement;
/// explicit flush events apply Grok's viewport-scaled per-frame cap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollFlushState {
    normalizer: ScrollNormalizer,
    viewport_rows: u16,
    backlog: i32,
    last_flush_ms: Option<u64>,
}

pub const DEFAULT_SCROLL_FLUSH_CADENCE_MS: u64 = 16;
pub const MIN_SCROLL_FLUSH_LINES: i32 = 6;

impl ScrollFlushState {
    pub const fn new(normalizer: ScrollNormalizer, viewport_rows: u16) -> Self {
        Self {
            normalizer,
            viewport_rows,
            backlog: 0,
            last_flush_ms: None,
        }
    }

    pub const fn input_at(mut self, at_ms: u64, direction: ScrollDirection) -> (Self, i32) {
        let (normalizer, delta) = self.normalizer.push_at(at_ms, direction);
        self.normalizer = normalizer;
        self.backlog += delta;
        (self, delta)
    }

    pub const fn with_normalizer(mut self, normalizer: ScrollNormalizer) -> Self {
        self.normalizer = normalizer;
        self
    }

    pub const fn with_viewport_rows(mut self, viewport_rows: u16) -> Self {
        self.viewport_rows = viewport_rows;
        self
    }

    pub const fn flush_cap(self) -> i32 {
        let viewport_cap = (self.viewport_rows / 2) as i32;
        if viewport_cap > MIN_SCROLL_FLUSH_LINES {
            viewport_cap
        } else {
            MIN_SCROLL_FLUSH_LINES
        }
    }

    pub const fn flush_at(mut self, at_ms: u64) -> (Self, ScrollFlush) {
        let cap = self.flush_cap();
        let lines = if self.backlog < -cap {
            -cap
        } else if self.backlog > cap {
            cap
        } else {
            self.backlog
        };
        self.backlog -= lines;
        self.last_flush_ms = Some(at_ms);
        (
            self,
            ScrollFlush {
                lines,
                backlog: self.backlog,
            },
        )
    }

    pub const fn finalize(mut self) -> (Self, ScrollFinalize) {
        let dropped = self.backlog;
        self.backlog = 0;
        self.last_flush_ms = None;
        (
            self,
            ScrollFinalize {
                flushed: 0,
                dropped,
                backlog: 0,
            },
        )
    }

    pub const fn backlog(self) -> i32 {
        self.backlog
    }

    pub const fn last_flush_ms(self) -> Option<u64> {
        self.last_flush_ms
    }

    pub const fn flush_due(self, at_ms: u64) -> bool {
        match self.last_flush_ms {
            None => true,
            Some(last) => at_ms.saturating_sub(last) >= DEFAULT_SCROLL_FLUSH_CADENCE_MS,
        }
    }

    pub const fn normalizer_for_replay(self) -> ScrollNormalizer {
        self.normalizer
    }
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
    speed_tenths: i32,
    inverted: bool,
    mode: ScrollMode,
    pending_units: i32,
    stream_events: i32,
    stream_elapsed_ms: u64,
    stream_trackpad: bool,
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
            speed_tenths: 10,
            inverted: false,
            mode: ScrollMode::Auto,
            pending_units: 0,
            stream_events: 0,
            stream_elapsed_ms: 0,
            stream_trackpad: false,
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

    pub const fn with_speed(mut self, speed: u8) -> Self {
        let speed = if speed < 1 {
            1
        } else if speed > 100 {
            100
        } else {
            speed
        } as i32;
        self.speed_tenths = if speed <= 50 {
            1 + (speed - 1) * 9 / 49
        } else {
            10 + (speed - 50) * 50 / 50
        };
        self
    }

    pub const fn with_inversion(mut self, inverted: bool) -> Self {
        self.inverted = inverted;
        self
    }

    pub const fn with_mode(mut self, mode: ScrollMode) -> Self {
        self.mode = mode;
        self
    }

    pub const fn push(self, direction: ScrollDirection) -> (Self, i32) {
        self.push_with_multiplier(direction, BASE_MULTIPLIER)
    }

    const fn push_with_multiplier(
        mut self,
        direction: ScrollDirection,
        multiplier: i32,
    ) -> (Self, i32) {
        let mut sign = match direction {
            ScrollDirection::Up => -1,
            ScrollDirection::Down => 1,
        };
        if self.inverted {
            sign = -sign;
        }
        self.pending_units += sign * self.lines_per_tick * multiplier * self.speed_tenths;
        let denominator = self.events_per_tick * FIXED_POINT * 10;
        let delta = self.pending_units / denominator;
        self.pending_units %= denominator;
        (self, delta)
    }

    const fn classify_stream(mut self, interval: u64) -> Self {
        if self.stream_events == 0 {
            self.stream_events = 1;
        } else {
            self.stream_events += 1;
            self.stream_elapsed_ms += interval;
        }
        self.stream_trackpad = match self.mode {
            ScrollMode::Trackpad => true,
            ScrollMode::Wheel => false,
            ScrollMode::Auto if self.events_per_tick == 1 => interval <= 30 || self.stream_trackpad,
            ScrollMode::Auto => {
                self.stream_trackpad
                    || (self.stream_events >= self.events_per_tick && self.stream_elapsed_ms > 12)
            }
        };
        self
    }

    /// Push an event at an injected monotonic millisecond timestamp. A gap
    /// larger than Grok's stream boundary starts a fresh gesture; replay can
    /// exercise this without wall-clock access or test sleeps.
    pub const fn push_at(mut self, at_ms: u64, direction: ScrollDirection) -> (Self, i32) {
        if let Some(last) = self.last_event_ms {
            let interval = at_ms.saturating_sub(last);
            if interval > self.stream_gap_ms {
                self.pending_units = 0;
                self.stream_events = 0;
                self.stream_elapsed_ms = 0;
                self.stream_trackpad = false;
            }
            self = self.classify_stream(interval);
            let multiplier = if self.stream_trackpad {
                BASE_MULTIPLIER
            } else if interval < 8 {
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
        self.stream_events = 1;
        self.stream_elapsed_ms = 0;
        self.stream_trackpad = matches!(self.mode, ScrollMode::Trackpad);
        self.push_with_multiplier(direction, BASE_MULTIPLIER)
    }
}

#[cfg(test)]
mod tests {
    use super::{ScrollDirection, ScrollMode, ScrollNormalizer};

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

    #[test]
    fn speed_and_inversion_overrides_are_deterministic() {
        let slow = ScrollNormalizer::default().with_speed(1);
        let fast = ScrollNormalizer::default().with_speed(100);
        assert_eq!(slow.push(ScrollDirection::Down).1, 0);
        assert_eq!(fast.push(ScrollDirection::Down).1, 6);
        assert_eq!(
            ScrollNormalizer::default()
                .with_inversion(true)
                .push(ScrollDirection::Down)
                .1,
            -1
        );
    }

    #[test]
    fn explicit_trackpad_mode_disables_wheel_acceleration() {
        let normalizer = ScrollNormalizer::default().with_mode(ScrollMode::Trackpad);
        let (normalizer, _) = normalizer.push_at(0, ScrollDirection::Down);
        let (_, delta) = normalizer.push_at(5, ScrollDirection::Down);
        assert_eq!(delta, 1);
    }

    #[test]
    fn auto_mode_promotes_a_slow_multi_event_stream_to_trackpad_pricing() {
        let normalizer = ScrollNormalizer::default();
        let (normalizer, _) = normalizer.push_at(0, ScrollDirection::Down);
        let (normalizer, _) = normalizer.push_at(20, ScrollDirection::Down);
        let (_, delta) = normalizer.push_at(40, ScrollDirection::Down);
        assert_eq!(delta, 1);
    }

    #[test]
    fn explicit_flush_caps_backlog_and_preserves_the_tail() {
        let mut state = super::ScrollFlushState::new(ScrollNormalizer::new(1, 20), 20);
        for at_ms in 0..4 {
            let (next, _) = state.input_at(at_ms, ScrollDirection::Down);
            state = next;
        }
        assert_eq!(state.backlog(), 80);
        let (state, first) = state.flush_at(16);
        assert_eq!(
            first,
            super::ScrollFlush {
                lines: 10,
                backlog: 70
            }
        );
        let (state, second) = state.flush_at(32);
        assert_eq!(
            second,
            super::ScrollFlush {
                lines: 10,
                backlog: 60
            }
        );
        assert_eq!(state.last_flush_ms(), Some(32));
    }

    #[test]
    fn finalize_does_not_emit_an_uncapped_catch_up_burst() {
        let state = super::ScrollFlushState::new(ScrollNormalizer::new(1, 20), 20);
        let (state, _) = state.input_at(0, ScrollDirection::Down);
        let (state, _) = state.input_at(1, ScrollDirection::Down);
        let (state, result) = state.finalize();
        assert_eq!(
            result,
            super::ScrollFinalize {
                flushed: 0,
                dropped: 40,
                backlog: 0
            }
        );
        assert_eq!(state.backlog(), 0);
    }

    #[test]
    fn viewport_event_changes_the_next_flush_cap() {
        let mut state = super::ScrollFlushState::new(ScrollNormalizer::new(1, 20), 10);
        for at_ms in 0..4 {
            let (next, _) = state.input_at(at_ms, ScrollDirection::Down);
            state = next;
        }
        assert!(state.flush_due(16));
        state = state.with_viewport_rows(30);
        let (state, flush) = state.flush_at(16);
        assert_eq!(flush.lines, 15);
        assert_eq!(flush.backlog, 65);
        assert_eq!(state.last_flush_ms(), Some(16));
        assert!(!state.flush_due(20));
        assert!(state.flush_due(32));
    }

    #[test]
    fn auto_mode_keeps_a_fast_tick_in_wheel_acceleration_band() {
        let normalizer = ScrollNormalizer::default();
        let (normalizer, _) = normalizer.push_at(0, ScrollDirection::Down);
        let (_, delta) = normalizer.push_at(5, ScrollDirection::Down);
        assert_eq!(delta, 2);
    }
}
