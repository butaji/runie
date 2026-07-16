//! Mouse scroll normalization (from Grok Build)
//!
//! Handles cross-platform scroll inconsistencies.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Scroll event from the terminal
#[derive(Debug, Clone)]
pub struct ScrollEvent {
    /// Number of lines scrolled (negative = up, positive = down)
    pub delta_lines: i32,
    /// When the event was received
    pub timestamp: Instant,
    /// Whether this came from a mouse wheel
    pub is_wheel: bool,
    /// Horizontal delta (for trackpads)
    pub delta_x: i32,
}

impl ScrollEvent {
    /// Create a new scroll event
    pub fn new(delta_lines: i32) -> Self {
        Self {
            delta_lines,
            timestamp: Instant::now(),
            is_wheel: true,
            delta_x: 0,
        }
    }

    /// Create from wheel event
    pub fn wheel(delta_lines: i32, delta_x: i32) -> Self {
        Self {
            delta_lines,
            timestamp: Instant::now(),
            is_wheel: true,
            delta_x,
        }
    }

    /// Create from button press
    pub fn button(delta_lines: i32) -> Self {
        Self {
            delta_lines,
            timestamp: Instant::now(),
            is_wheel: false,
            delta_x: 0,
        }
    }

    /// Check if this is an upward scroll
    pub fn is_up(&self) -> bool {
        self.delta_lines < 0
    }

    /// Check if this is a downward scroll
    pub fn is_down(&self) -> bool {
        self.delta_lines > 0
    }

    /// Get absolute value of scroll
    pub fn abs_delta(&self) -> i32 {
        self.delta_lines.abs()
    }
}

/// A scroll stream - sequence of events treated as one gesture
#[derive(Debug, Clone)]
pub struct ScrollStream {
    /// When the stream started
    pub start_time: Instant,
    /// Direction of the scroll
    pub direction: ScrollDirection,
    /// Events in this stream
    pub events: VecDeque<ScrollEvent>,
    /// Total accumulated delta
    pub total_delta: i32,
}

impl ScrollStream {
    /// Create a new stream from an initial event
    pub fn new(event: ScrollEvent) -> Self {
        let direction = if event.delta_lines < 0 {
            ScrollDirection::Up
        } else {
            ScrollDirection::Down
        };

        let mut events = VecDeque::new();
        events.push_back(event.clone());

        Self {
            start_time: event.timestamp,
            direction,
            events,
            total_delta: event.delta_lines,
        }
    }

    /// Add an event to the stream
    pub fn push(&mut self, event: ScrollEvent) {
        self.events.push_back(event.clone());
        self.total_delta += event.delta_lines;
    }

    /// Check if an event continues this stream
    pub fn can_extend(&self, event: &ScrollEvent, gap_threshold: Duration) -> bool {
        let direction_matches = match self.direction {
            ScrollDirection::Up => event.delta_lines <= 0,
            ScrollDirection::Down => event.delta_lines >= 0,
        };

        let gap = event.timestamp - self.events.back().map(|e| e.timestamp).unwrap_or(self.start_time);
        let gap_ok = gap <= gap_threshold;

        direction_matches && gap_ok
    }

    /// Get duration of the stream
    pub fn duration(&self) -> Duration {
        self.events
            .back()
            .map(|e| e.timestamp - self.start_time)
            .unwrap_or(Duration::ZERO)
    }

    /// Get event count
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

/// Scroll direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollDirection {
    Up,
    Down,
}

impl ScrollDirection {
    /// Invert the direction
    pub fn invert(self) -> Self {
        match self {
            ScrollDirection::Up => ScrollDirection::Down,
            ScrollDirection::Down => ScrollDirection::Up,
        }
    }

    /// Get a label for this direction
    pub fn label(&self) -> &'static str {
        match self {
            ScrollDirection::Up => "up",
            ScrollDirection::Down => "down",
        }
    }
}

/// Terminal scroll profile
#[derive(Debug, Clone, Copy, Default)]
pub enum TerminalProfile {
    /// iTerm2, WezTerm, Kitty: 1 event per tick
    OnePerTick,
    /// Most terminals: 3 events per tick
    #[default]
    ThreePerTick,
    /// tmux, screen: 1 event per tick
    TmuxLike,
    /// Alacritty: variable
    AlacrittyLike,
}

impl TerminalProfile {
    /// Detect profile from TERM environment variable
    pub fn detect() -> Self {
        let term = std::env::var("TERM").unwrap_or_default();

        if term.contains("tmux") || term.contains("screen") {
            TerminalProfile::TmuxLike
        } else if term.contains("xterm-kitty") || term.contains("wezterm") || term.contains("iter") {
            TerminalProfile::OnePerTick
        } else if term.contains("alacritty") {
            TerminalProfile::AlacrittyLike
        } else {
            TerminalProfile::ThreePerTick
        }
    }

    /// Get events per tick for this profile
    pub fn events_per_tick(&self) -> i32 {
        match self {
            TerminalProfile::OnePerTick => 1,
            TerminalProfile::ThreePerTick => 3,
            TerminalProfile::TmuxLike => 1,
            TerminalProfile::AlacrittyLike => 1,
        }
    }
}

/// Scroll normalization state
#[derive(Debug, Clone)]
pub struct ScrollState {
    /// Active streams
    streams: VecDeque<ScrollStream>,
    /// Last flush time
    last_flush: Instant,
    /// Terminal profile
    terminal_profile: TerminalProfile,
    /// Accumulated gestures pending emission
    pending_gestures: VecDeque<ScrollGesture>,
}

impl Default for ScrollState {
    fn default() -> Self {
        Self::new()
    }
}

impl ScrollState {
    /// Create new scroll state
    pub fn new() -> Self {
        Self {
            streams: VecDeque::new(),
            last_flush: Instant::now(),
            terminal_profile: TerminalProfile::detect(),
            pending_gestures: VecDeque::new(),
        }
    }

    /// Create with specific profile
    pub fn with_profile(profile: TerminalProfile) -> Self {
        Self {
            streams: VecDeque::new(),
            last_flush: Instant::now(),
            terminal_profile: profile,
            pending_gestures: VecDeque::new(),
        }
    }

    /// Process a raw scroll event
    /// Returns a gesture if one is complete, None if still accumulating
    pub fn push(&mut self, event: ScrollEvent) -> Option<ScrollGesture> {
        // Check if this continues an existing stream
        if let Some(stream) = self.streams.back_mut() {
            let gap = event.timestamp - stream.events.back()?.timestamp;

            if stream.can_extend(&event, Duration::from_millis(SCROLL_STREAM_GAP_MS)) {
                // Continue existing stream
                stream.push(event);

                // Check if stream is complete (gap too large or direction change)
                if gap > Duration::from_millis(SCROLL_STREAM_GAP_MS) {
                    return self.flush_stream(stream);
                }

                return None;
            }
        }

        // Start new stream
        let stream = ScrollStream::new(event);
        self.streams.push_back(stream);
        None
    }

    /// Flush a stream and return the gesture
    fn flush_stream(&mut self, stream: &mut ScrollStream) -> Option<ScrollGesture> {
        if stream.is_empty() {
            return None;
        }

        let gesture = self.normalize_stream(stream);
        Some(gesture)
    }

    /// Flush and normalize all accumulated streams
    pub fn flush(&mut self) -> Vec<ScrollGesture> {
        let now = Instant::now();
        let mut gestures = Vec::new();

        // Flush streams that are ready
        while let Some(mut stream) = self.streams.pop_front() {
            let duration = stream.duration();
            let len = stream.len();

            // Stream is complete if:
            // - Enough time has passed since start
            // - Enough events have accumulated
            // - There's a gap between this and next stream
            if duration > Duration::from_millis(SCROLL_FLUSH_INTERVAL_MS) || len >= 5 {
                let gesture = self.normalize_stream(&stream);
                gestures.push(gesture);
            } else {
                // Put it back and stop
                self.streams.push_front(stream);
                break;
            }
        }

        self.last_flush = now;
        gestures
    }

    /// Normalize a stream into a gesture
    fn normalize_stream(&self, stream: &ScrollStream) -> ScrollGesture {
        let events_per_tick = self.terminal_profile.events_per_tick();

        // Divide by events per tick to get actual lines
        let line_delta = stream.total_delta / events_per_tick;

        // Clamp to reasonable bounds
        let line_delta = line_delta.clamp(-20, 20);

        let duration_ms = stream
            .events
            .back()
            .map(|e| (e.timestamp - stream.start_time).as_millis() as u64)
            .unwrap_or(0);

        ScrollGesture {
            direction: stream.direction,
            line_delta,
            duration_ms,
            event_count: stream.len() as u32,
        }
    }

    /// Get the terminal profile
    pub fn terminal_profile(&self) -> TerminalProfile {
        self.terminal_profile
    }

    /// Set the terminal profile
    pub fn set_terminal_profile(&mut self, profile: TerminalProfile) {
        self.terminal_profile = profile;
    }

    /// Check if there are pending gestures
    pub fn has_pending(&self) -> bool {
        !self.pending_gestures.is_empty()
    }

    /// Pop a pending gesture
    pub fn pop_pending(&mut self) -> Option<ScrollGesture> {
        self.pending_gestures.pop_front()
    }

    /// Clear all state
    pub fn clear(&mut self) {
        self.streams.clear();
        self.pending_gestures.clear();
        self.last_flush = Instant::now();
    }
}

/// A complete scroll gesture
#[derive(Debug, Clone)]
pub struct ScrollGesture {
    /// Direction of scroll
    pub direction: ScrollDirection,
    /// Number of lines scrolled
    pub line_delta: i32,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Number of raw events
    pub event_count: u32,
}

impl ScrollGesture {
    /// Create a new gesture
    pub fn new(direction: ScrollDirection, line_delta: i32) -> Self {
        Self {
            direction,
            line_delta,
            duration_ms: 0,
            event_count: 1,
        }
    }

    /// Check if this is an upward scroll
    pub fn is_up(&self) -> bool {
        self.direction == ScrollDirection::Up
    }

    /// Check if this is a downward scroll
    pub fn is_down(&self) -> bool {
        self.direction == ScrollDirection::Down
    }

    /// Get velocity (lines per second)
    pub fn velocity(&self) -> f64 {
        if self.duration_ms == 0 {
            return 0.0;
        }
        self.line_delta.abs() as f64 / (self.duration_ms as f64 / 1000.0)
    }

    /// Get the number of "pages" (assuming 3 lines per page)
    pub fn pages(&self) -> f64 {
        self.line_delta as f64 / 3.0
    }

    /// Check if this is a "flick" gesture (fast, short duration)
    pub fn is_flick(&self) -> bool {
        self.duration_ms < 100 && self.line_delta.abs() >= 3
    }

    /// Check if this is a "drag" gesture (slow, long duration)
    pub fn is_drag(&self) -> bool {
        self.duration_ms > 200 && self.line_delta.abs() <= 2
    }
}

/// Scroll constants
pub const SCROLL_STREAM_GAP_MS: u64 = 80;
pub const SCROLL_FLUSH_INTERVAL_MS: u64 = 16;

/// Mouse button for scroll
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    ScrollUp,
    ScrollDown,
}

impl MouseButton {
    /// Parse from button number
    pub fn from_button_number(n: u8) -> Option<Self> {
        match n {
            0 => Some(MouseButton::Left),
            1 => Some(MouseButton::Middle),
            2 => Some(MouseButton::Right),
            4 => Some(MouseButton::ScrollUp),
            5 => Some(MouseButton::ScrollDown),
            _ => None,
        }
    }
}

/// Mouse event
#[derive(Debug, Clone)]
pub struct MouseEvent {
    /// Button that triggered the event
    pub button: MouseButton,
    /// X position
    pub x: u16,
    /// Y position
    pub y: u16,
    /// Modifiers held
    pub modifiers: MouseModifiers,
    /// Timestamp
    pub timestamp: Instant,
}

impl MouseEvent {
    /// Check if this is a scroll event
    pub fn is_scroll(&self) -> bool {
        matches!(
            self.button,
            MouseButton::ScrollUp | MouseButton::ScrollDown
        )
    }

    /// Get scroll delta (None if not a scroll event)
    pub fn scroll_delta(&self) -> Option<i32> {
        match self.button {
            MouseButton::ScrollUp => Some(-1),
            MouseButton::ScrollDown => Some(1),
            _ => None,
        }
    }
}

/// Mouse modifiers
#[derive(Debug, Clone, Copy, Default)]
pub struct MouseModifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

impl MouseModifiers {
    /// Check if any modifier is held
    pub fn has_any(&self) -> bool {
        self.shift || self.ctrl || self.alt
    }
}

/// Parse SGR mouse sequence
pub fn parse_sgr_mouse(data: &str) -> Option<MouseEvent> {
    // SGR format: CSI M Pb Px Py
    // or: CSI < Pb ; Px ; Py M
    let data = data.trim();

    if data.len() < 4 {
        return None;
    }

    // Check for SGR release event (CSI M ...m)
    let is_release = data.ends_with('m');

    // Extract button and coordinates
    // Format: CSI M <Cb>;<Px>;<Py> or CSI <Cb>;<Px>;<Py>M
    let data = data.strip_prefix("\x1b[M")?;

    let parts: Vec<&str> = data.trim_end_matches(|c| c == 'M' || c == 'm').split(';').collect();
    if parts.len() < 3 {
        return None;
    }

    let button_code: u8 = parts[0].parse().ok()?;
    let x: u16 = parts[1].parse().ok()?;
    let y: u16 = parts[2].parse().ok()?;

    let button = if is_release {
        // Release events use button + 32
        MouseButton::from_button_number(button_code.saturating_sub(32))
    } else {
        MouseButton::from_button_number(button_code)
    }?;

    let modifiers = MouseModifiers {
        shift: button_code & 4 != 0,
        ctrl: button_code & 8 != 0,
        alt: button_code & 16 != 0,
    };

    Some(MouseEvent {
        button,
        x: x.saturating_sub(1), // Convert to 0-indexed
        y: y.saturating_sub(1),
        modifiers,
        timestamp: Instant::now(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scroll_event() {
        let event = ScrollEvent::new(-3);
        assert!(event.is_up());
        assert!(!event.is_down());
        assert_eq!(event.abs_delta(), 3);
    }

    #[test]
    fn test_scroll_stream() {
        let mut stream = ScrollStream::new(ScrollEvent::new(-1));
        assert_eq!(stream.len(), 1);
        assert_eq!(stream.direction, ScrollDirection::Up);

        stream.push(ScrollEvent::new(-1));
        assert_eq!(stream.len(), 2);
        assert_eq!(stream.total_delta, -2);
    }

    #[test]
    fn test_terminal_profile_detection() {
        // Without TERM set, should default to ThreePerTick
        std::env::remove_var("TERM");
        let profile = TerminalProfile::detect();
        assert_eq!(profile, TerminalProfile::ThreePerTick);
    }

    #[test]
    fn test_scroll_state_normalization() {
        let mut state = ScrollState::with_profile(TerminalProfile::ThreePerTick);

        // Simulate 3 events (typical terminal sends 3 per scroll tick)
        state.push(ScrollEvent::new(-1));
        state.push(ScrollEvent::new(-1));
        state.push(ScrollEvent::new(-1));

        // Should not produce gesture yet
        let gestures = state.flush();
        assert!(!gestures.is_empty());

        let gesture = &gestures[0];
        assert_eq!(gesture.direction, ScrollDirection::Up);
        assert_eq!(gesture.line_delta, -1); // -3 / 3 = -1
    }

    #[test]
    fn test_scroll_gesture() {
        let gesture = ScrollGesture::new(ScrollDirection::Down, 3);

        assert!(gesture.is_down());
        assert!(!gesture.is_up());
        assert!(gesture.is_flick());
        assert!(!gesture.is_drag());

        // Set duration for velocity calculation
        let slow_gesture = ScrollGesture {
            direction: ScrollDirection::Down,
            line_delta: 2,
            duration_ms: 400,
            event_count: 3,
        };

        assert!(slow_gesture.is_drag());
        assert_eq!(slow_gesture.velocity(), 5.0);
    }

    #[test]
    fn test_mouse_button_parse() {
        assert_eq!(MouseButton::from_button_number(0), Some(MouseButton::Left));
        assert_eq!(MouseButton::from_button_number(1), Some(MouseButton::Middle));
        assert_eq!(MouseButton::from_button_number(2), Some(MouseButton::Right));
        assert_eq!(MouseButton::from_button_number(4), Some(MouseButton::ScrollUp));
        assert_eq!(MouseButton::from_button_number(5), Some(MouseButton::ScrollDown));
        assert_eq!(MouseButton::from_button_number(99), None);
    }

    #[test]
    fn test_mouse_event_scroll() {
        let event = MouseEvent {
            button: MouseButton::ScrollDown,
            x: 10,
            y: 20,
            modifiers: MouseModifiers::default(),
            timestamp: Instant::now(),
        };

        assert!(event.is_scroll());
        assert_eq!(event.scroll_delta(), Some(1));
    }

    #[test]
    fn test_scroll_direction() {
        assert_eq!(ScrollDirection::Up.invert(), ScrollDirection::Down);
        assert_eq!(ScrollDirection::Down.invert(), ScrollDirection::Up);
        assert_eq!(ScrollDirection::Up.label(), "up");
        assert_eq!(ScrollDirection::Down.label(), "down");
    }
}
