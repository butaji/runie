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

pub use crate::scroll_flush::{
    ScrollFinalize, ScrollFlush, ScrollFlushState, DEFAULT_SCROLL_FLUSH_CADENCE_MS,
    MIN_SCROLL_FLUSH_LINES,
};

/// Converts raw events into whole-line deltas without terminal or clock state.
/// Grok's default is three lines per three raw events; retaining the remainder
/// makes non-default profiles deterministic as well.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollNormalizer {
    events_per_tick: i32,
    trackpad_events_per_tick: i32,
    trackpad_detect_max_interval_ms: u64,
    lines_per_tick: i32,
    trackpad_lines_per_tick: i32,
    accel_fast_interval_ms: u64,
    accel_medium_interval_ms: u64,
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
// Multiple reports below this interval can represent one physical wheel notch
// in terminal batching, rather than a faster gesture.
const ACCEL_MIN_INTERVAL_MS: u64 = 6;

impl Default for ScrollNormalizer {
    fn default() -> Self {
        Self::new(3, 3)
    }
}

impl ScrollNormalizer {
    const fn acceleration_multiplier(&self, interval: u64) -> i32 {
        if interval < ACCEL_MIN_INTERVAL_MS {
            BASE_MULTIPLIER
        } else if interval <= self.accel_fast_interval_ms {
            FAST_MULTIPLIER
        } else if interval <= self.accel_medium_interval_ms {
            let span = self.accel_medium_interval_ms - self.accel_fast_interval_ms;
            let elapsed = interval - self.accel_fast_interval_ms;
            FAST_MULTIPLIER
                - ((elapsed as i32 * (FAST_MULTIPLIER - MEDIUM_MULTIPLIER)) / span as i32)
        } else {
            BASE_MULTIPLIER
        }
    }

    pub const fn new(events_per_tick: i32, lines_per_tick: i32) -> Self {
        let lines_per_tick = if lines_per_tick > 0 {
            lines_per_tick
        } else {
            1
        };
        Self {
            events_per_tick: if events_per_tick > 0 {
                events_per_tick
            } else {
                1
            },
            trackpad_events_per_tick: if events_per_tick > 0 {
                events_per_tick
            } else {
                1
            },
            trackpad_detect_max_interval_ms: 30,
            lines_per_tick,
            trackpad_lines_per_tick: lines_per_tick,
            accel_fast_interval_ms: 8,
            accel_medium_interval_ms: 20,
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
        let trackpad_lines_per_tick =
            if matches!(brand.as_str(), "vscode" | "cursor" | "windsurf") && !remuxed {
                15
            } else {
                3
            };
        let mut normalizer = Self::new(events_per_tick, lines_per_tick);
        normalizer.trackpad_lines_per_tick = trackpad_lines_per_tick;
        normalizer.trackpad_events_per_tick = 3;
        normalizer.trackpad_detect_max_interval_ms =
            if matches!(brand.as_str(), "vscode" | "cursor" | "windsurf") && !remuxed {
                60
            } else {
                30
            };
        if matches!(brand.as_str(), "vscode" | "cursor" | "windsurf") && !remuxed {
            normalizer.accel_fast_interval_ms = 25;
            normalizer.accel_medium_interval_ms = 50;
        }
        normalizer
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
        let lines_per_tick = if self.stream_trackpad {
            self.trackpad_lines_per_tick
        } else {
            self.lines_per_tick
        };
        self.pending_units += sign * lines_per_tick * multiplier * self.speed_tenths;
        let events_per_tick = if self.stream_trackpad {
            self.trackpad_events_per_tick
        } else {
            self.events_per_tick
        };
        let denominator = events_per_tick * FIXED_POINT * 10;
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
            ScrollMode::Auto if self.events_per_tick == 1 => {
                self.stream_trackpad
                    || (self.stream_events > 2
                        && self.stream_elapsed_ms / (self.stream_events.saturating_sub(1) as u64)
                            < self.trackpad_detect_max_interval_ms)
            }
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
            } else {
                self.acceleration_multiplier(interval)
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
    use super::{
        ScrollDirection, ScrollMode, ScrollNormalizer, BASE_MULTIPLIER, FAST_MULTIPLIER,
        MEDIUM_MULTIPLIER,
    };

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
        let (_, fast) = normalizer.push_at(7, ScrollDirection::Down);
        assert_eq!(base, 1);
        assert_eq!(medium, 2);
        assert_eq!(fast, 2);
    }

    #[test]
    fn acceleration_interpolates_between_grok_speed_bands() {
        let normalizer = ScrollNormalizer::default();
        assert_eq!(normalizer.acceleration_multiplier(8), FAST_MULTIPLIER);
        assert_eq!(normalizer.acceleration_multiplier(14), 21);
        assert_eq!(normalizer.acceleration_multiplier(20), MEDIUM_MULTIPLIER);
        assert_eq!(normalizer.acceleration_multiplier(21), BASE_MULTIPLIER);
    }

    #[test]
    fn sub_six_millisecond_terminal_batches_do_not_accelerate() {
        let normalizer = ScrollNormalizer::default();
        let (normalizer, _) = normalizer.push_at(0, ScrollDirection::Down);
        let (_, delta) = normalizer.push_at(5, ScrollDirection::Down);
        assert_eq!(delta, 1);
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
    fn vscode_profile_prices_trackpad_streams_like_grok() {
        let normalizer =
            ScrollNormalizer::for_terminal_context("vscode", false).with_mode(ScrollMode::Trackpad);
        let (normalizer, first) = normalizer.push_at(0, ScrollDirection::Down);
        let (_, second) = normalizer.push_at(20, ScrollDirection::Down);
        assert_eq!(first, 5);
        assert_eq!(second, 5);
    }

    #[test]
    fn vscode_profile_uses_grok_trackpad_detection_window() {
        let vscode = ScrollNormalizer::for_terminal_context("cursor", false);
        let unknown = ScrollNormalizer::for_terminal_context("xterm", false);
        assert_eq!(vscode.trackpad_detect_max_interval_ms, 60);
        assert_eq!(unknown.trackpad_detect_max_interval_ms, 30);
        assert_eq!(vscode.accel_fast_interval_ms, 25);
        assert_eq!(vscode.accel_medium_interval_ms, 50);
        assert_eq!(unknown.accel_fast_interval_ms, 8);
        assert_eq!(unknown.accel_medium_interval_ms, 20);
    }

    #[test]
    fn ept_one_profiles_wait_for_three_events_before_trackpad_promotion() {
        let normalizer = ScrollNormalizer::for_terminal_context("wezterm", false);
        let (normalizer, first) = normalizer.push_at(0, ScrollDirection::Down);
        let (normalizer, second) = normalizer.push_at(10, ScrollDirection::Down);
        let (normalizer, third) = normalizer.push_at(20, ScrollDirection::Down);
        assert_eq!((first, second), (1, 2));
        assert_eq!(third, 1);
        assert!(normalizer.stream_trackpad);
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
    fn zero_delivery_flush_is_a_cadence_noop() {
        let state = super::ScrollFlushState::new(ScrollNormalizer::default(), 20);
        let (state, flush) = state.flush_at(16);
        assert_eq!(flush.lines, 0);
        assert_eq!(state.last_flush_ms(), None);
        assert!(state.flush_due(16));
    }

    #[test]
    fn auto_mode_ignores_a_sub_six_millisecond_batching_interval() {
        let normalizer = ScrollNormalizer::default();
        let (normalizer, _) = normalizer.push_at(0, ScrollDirection::Down);
        let (_, delta) = normalizer.push_at(5, ScrollDirection::Down);
        assert_eq!(delta, 1);
    }
}
