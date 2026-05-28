# Runie Feed Synthesis: Cross-Agent Pattern Analysis

**Date**: 2026-05-28
**Sources**: Codex (CLI agent), OpenCode (web UI), Pi (terminal UI)
**Purpose**: Identify patterns missing from Runie that should be adopted

---

## 1. Feed Elements We Should Add

### 1.1 Message Separators with Metrics

**Pattern from Codex**: Full-width separator cell displaying runtime metrics.

```rust
struct SeparatorCell {
    content: String,          // e.g., "7.2s · 4 tool calls · 1,247 tokens"
    height: usize,
}
```

**Implementation for Runie**:
```rust
pub struct MessageSeparator {
    pub runtime_ms: u64,
    pub tool_call_count: usize,
    pub token_count: usize,
}

impl HistoryCell for MessageSeparator {
    fn display_lines(&self, _width: usize) -> Vec<Line> {
        let metrics = format!(
            "{} · {} tool calls · {} tokens",
            format_duration(self.runtime_ms),
            self.tool_call_count,
            self.token_count
        );
        vec![Line::from(format!("─{}─", metrics.center(width - 2, '─')))]
    }
}
```

### 1.2 Thinking/Reasoning Blocks

**Pattern from Pi**: Reasoning blocks with distinct styling and `●` prefix for headers.

```rust
struct ThinkingBlock {
    content: String,
    collapsed: bool,
}
```

**Runie implementation**:
```rust
pub struct ReasoningCell {
    content: String,
    // Renders as:
    // ● Reasoning
    //   (content with indentation)
}

impl HistoryCell for ReasoningCell {
    fn display_lines(&self, width: usize) -> Vec<Line> {
        let mut lines = vec![Line::from(styled("● Reasoning", Theme::Accent))];
        for chunk in self.content.wrap(width - 4) {
            lines.push(Line::from(format!("  {}", chunk)));
        }
        lines
    }
}
```

### 1.3 Tool Execution Progress Gauge

**Pattern from Codex**: Visual progress indicator for running tools.

```rust
struct ToolProgressGauge {
    tool_name: String,
    status: ToolStatus,  // Running, Success, Failed
    spinner_frame: usize,
}
```

**Runie implementation**:
```rust
#[derive(Clone)]
pub enum ToolStatus {
    Running,
    Success,
    Failed,
}

pub struct ToolProgressCell {
    name: String,
    status: ToolStatus,
    frame: usize,
}

const BRAILLE_FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

impl HistoryCell for ToolProgressCell {
    fn display_lines(&self, width: usize) -> Vec<Line> {
        let (icon, color) = match self.status {
            ToolStatus::Running => (BRAILLE_FRAMES[self.frame % 10], Theme::Muted),
            ToolStatus::Success => ("✓", Theme::Success),
            ToolStatus::Failed => ("✗", Theme::Error),
        };
        let line = format!("{} {}...", icon, self.name);
        vec![Line::from(styled(line, color))]
    }
}
```

### 1.4 Message Actions

**Pattern from OpenCode**: Copy icon that toggles to checkmark with 2s auto-revert.

```rust
struct MessageActions {
    message_id: Uuid,
    copied: bool,
}
```

**Runie implementation**:
```rust
pub struct MessageActionsCell {
    id: Uuid,
    actions: Vec<MessageAction>,  // Copy, Retry, Delete
}

#[derive(Clone)]
pub enum MessageAction {
    Copy,
    Retry,
}

impl HistoryCell for MessageActionsCell {
    fn display_lines(&self, _width: usize) -> Vec<Line> {
        // Renders as: [Copy] [Retry]
        // Click/toggle behavior handled by input layer
        vec![Line::from("[Copy] [Retry]")]
    }
}
```

### 1.5 Runtime Metrics Separator

**Pattern from Codex**: Full-width separator showing aggregate metrics for a response turn.

```rust
struct MetricsSeparator {
    elapsed_ms: u64,
    input_tokens: usize,
    output_tokens: usize,
    tool_calls: usize,
}
```

### 1.6 Session Header Cell

**Pattern from Pi**: Header showing session info at top of transcript.

```rust
struct SessionHeader {
    session_id: String,
    model: String,
    started_at: DateTime<Utc>,
}
```

---

## 2. Status System Improvements

### 2.1 Live Status Row with Elapsed Timer

**Pattern from Codex**: StatusIndicatorWidget displays live status above composer.

```rust
pub struct StatusIndicatorWidget {
    status: StatusState,
    elapsed: Duration,
    details: String,
    timer: Interval,
}

#[derive(Clone)]
pub enum StatusState {
    Idle,
    Thinking,
    Working,
    Done,
    Error(String),
}
```

**Runie adoption**:

```rust
pub struct RunieStatusBar {
    state: StatusState,
    started_at: Instant,
    details: String,
}

impl RunieStatusBar {
    pub fn set_state(&mut self, state: StatusState) {
        self.state = state;
        self.started_at = Instant::now();
        self.request_render();
    }

    pub fn display_lines(&self, width: usize) -> Vec<Line> {
        let elapsed = self.started_at.elapsed();
        let (indicator, color) = match &self.state {
            StatusState::Thinking => ("⠋", Theme::Accent),
            StatusState::Working => ("⠙", Theme::Accent),
            StatusState::Done => ("✓", Theme::Success),
            StatusState::Error(msg) => ("✗", Theme::Error),
            StatusState::Idle => (" ", Theme::Muted),
        };

        let status_text = format!(
            "{} {:>10}  {}",
            indicator,
            format_elapsed(elapsed),
            self.details
        );

        vec![Line::from(styled(status_text, color))]
    }
}
```

### 2.2 Status State Machine

**Transitions**:

```
Idle → Thinking (user submitted)
Thinking → Working (first token received)
Working → Done (stream complete)
Any → Error (on error)
Done → Idle (after 3s or on new input)
Error → Idle (on acknowledge)
```

### 2.3 Configurable Status Line Items

```rust
pub struct StatusConfig {
    show_elapsed: bool,
    show_token_count: bool,
    show_model: bool,
    show_tool_status: bool,
}
```

### 2.4 Error Display in Status

```rust
impl StatusState {
    fn is_error(&self) -> bool {
        matches!(self, StatusState::Error(_))
    }
}

// Status bar turns red with error when Error state
```

---

## 3. Interaction Improvements

### 3.1 Input History (Up/Down in Input)

**Pattern from Pi**: Input history navigable via Up/Down arrows, 100 entries.

```rust
pub struct InputHistory {
    entries: Vec<String>,
    position: isize,
    max_size: usize,
}

impl InputHistory {
    pub fn push(&mut self, input: String) {
        if self.entries.last() != Some(&input) {
            self.entries.push(input);
            if self.entries.len() > self.max_size {
                self.entries.remove(0);
            }
        }
        self.position = self.entries.len() as isize;
    }

    pub fn navigate_up(&mut self) -> Option<String> {
        if self.position > 0 {
            self.position -= 1;
        }
        self.entries.get(self.position as usize).cloned()
    }

    pub fn navigate_down(&mut self) -> Option<String> {
        if self.position < self.entries.len() as isize - 1 {
            self.position += 1;
        } else {
            self.position = self.entries.len() as isize;
            return Some(String::new());
        }
        self.entries.get(self.position as usize).cloned()
    }
}
```

**Key binding integration**:

```rust
match key {
    Key::Up if self.composer.is_focused() => {
        if let Some(history) = self.input_history.navigate_up() {
            self.composer.set_content(history);
        }
    }
    Key::Down if self.composer.is_focused() => {
        if let Some(history) = self.input_history.navigate_down() {
            self.composer.set_content(history);
        }
    }
}
```

### 3.2 Better Keyboard Navigation

**PageUp/PageDown for Feed**:

```rust
match key {
    Key::PageUp => self.feed.scroll_by(-self.page_size),
    Key::PageDown => self.feed.scroll_by(self.page_size),
    Key::Home => self.feed.scroll_to_top(),
    Key::End => self.feed.scroll_to_bottom(),
}
```

### 3.3 Copy Last Response Shortcut

```rust
match key_sequence {
    "C-c C-c" => self.copy_last_response(),
    "M-c" => self.copy_last_response(),
}
```

### 3.4 /commands with Autocomplete

**Pattern from Pi**: Command system with SelectList autocomplete.

```rust
pub struct CommandRegistry {
    commands: Vec<Command>,
}

pub struct Command {
    name: &'static str,
    description: &'static str,
    handler: Box<dyn Fn(&mut App)>,
}

impl CommandRegistry {
    pub fn get_matches(&self, input: &str) -> Vec<&Command> {
        self.commands
            .iter()
            .filter(|c| c.name.starts_with(input.trim_start_matches('/')))
            .collect()
    }
}

// Built-in commands
// /clear     - Clear feed
// /retry     - Retry last message
// /copy      - Copy last response
// /model     - Switch model
// /help      - Show help
```

### 3.5 Interrupt Handling Improvements

**Pattern from Pi**: Ctrl+C/Esc propagates to agent streaming + tool calls.

```rust
match key {
    Key::Ctrl('c') => {
        self.abort_signal.send(AbortReason::UserInterrupt);
        self.abort_controller.abort();
    }
    Key::Esc => {
        if self.composer.is_focused() {
            self.composer.blur();
            self.feed.focus();
        } else {
            self.abort_signal.send(AbortReason::UserInterrupt);
        }
    }
}
```

---

## 4. Scrolling Improvements

### 4.1 Smooth Scrolling with Acceleration

```rust
pub struct SmoothScrollController {
    target_offset: f64,
    current_offset: f64,
    velocity: f64,
}

impl SmoothScrollController {
    pub fn scroll_to(&mut self, target: f64) {
        self.target_offset = target;
    }

    pub fn tick(&mut self) {
        let diff = self.target_offset - self.current_offset;
        self.velocity += diff * 0.15;  // Spring factor
        self.velocity *= 0.85;          // Damping
        self.current_offset += self.velocity;

        if diff.abs() < 0.5 && self.velocity.abs() < 0.1 {
            self.current_offset = self.target_offset;
            self.velocity = 0.0;
        }
    }
}
```

### 4.2 Follow-Bottom Behavior

```rust
pub struct ScrollAnchor {
    auto_follow: bool,
    user_scrolled: bool,
}

impl ScrollAnchor {
    pub fn on_content_added(&mut self) {
        if !self.user_scrolled {
            self.scroll_to_bottom();
        }
    }

    pub fn on_user_scroll(&mut self) {
        self.user_scrolled = true;
    }
}
```

### 4.3 Independent Transcript Overlay

**Pattern from Pi**: PagerView with scroll_offset independent from main viewport.

```rust
pub struct TranscriptPager {
    feed: Vec<Box<dyn HistoryCell>>,
    scroll_offset: usize,
    visible_height: usize,
}
```

### 4.4 Scroll Indicators

```rust
pub struct ScrollIndicator {
    total_lines: usize,
    visible_lines: usize,
    scroll_position: usize,
}

impl ScrollIndicator {
    pub fn display_lines(&self, width: usize) -> Vec<Line> {
        if self.total_lines <= self.visible_lines {
            return vec![];
        }

        let scrollbar_height = self.visible_lines * self.visible_lines / self.total_lines;
        let thumb_position = self.scroll_position * self.visible_lines / self.total_lines;

        // Render: ████░░░░░░ (simplified)
        let indicator = format!("{}/{}", self.scroll_position, self.total_lines);
        vec![Line::from(indicator)]
    }
}
```

---

## 5. Streaming Improvements

### 5.1 Queue-Based Emission with Timestamps

**Pattern from Codex**: Two-region streaming with stable content + tail.

```rust
pub struct StreamingQueue {
    stable: Vec<ContentChunk>,    // Committed content
    tail: Vec<ContentChunk>,      // In-flight content
    pending: Vec<ContentChunk>,    // Buffered but not yet emitted
}

#[derive(Clone)]
pub struct ContentChunk {
    pub text: String,
    pub timestamp: Instant,
    pub is_final: bool,
}

impl StreamingQueue {
    pub fn emit(&mut self) -> Vec<ContentChunk> {
        let mut ready = Vec::new();

        // Move pending to tail if older than 50ms
        let cutoff = Instant::now() - Duration::from_millis(50);
        while let Some(chunk) = self.pending.pop_front() {
            if chunk.timestamp < cutoff || chunk.is_final {
                self.tail.push(chunk);
            } else {
                self.pending.push_front(chunk);
                break;
            }
        }

        // Move all tail to stable
        ready.extend(self.tail.drain(..));
        ready
    }
}
```

### 5.2 Adaptive Chunking

**Pattern from OpenCode**: PacedMarkdown with 24ms character reveal.

```rust
pub enum ChunkingStrategy {
    /// Steady pace for normal streaming
    Smooth {
        chars_per_second: f64,  // Default: 42 (24ms per char)
    },
    /// Fast catchup for buffered content
    CatchUp {
        max_delay_ms: u64,       // Default: 3000
    },
    /// Instant for user-pasted content
    Instant,
}

pub struct PacedMarkdown {
    strategy: ChunkingStrategy,
    pending: Vec<char>,
    last_emit: Instant,
}

impl PacedMarkdown {
    pub fn tick(&mut self, available: usize) -> Option<String> {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_emit);

        let (delay, batch_size) = match &self.strategy {
            ChunkingStrategy::Smooth { chars_per_second } => {
                let delay = Duration::from_secs_f64(1.0 / chars_per_second);
                let batch_size = if elapsed >= delay { 1 } else { 0 };
                (delay, batch_size)
            }
            ChunkingStrategy::CatchUp { max_delay_ms } => {
                let delay = Duration::from_millis(*max_delay_ms);
                let batch_size = if elapsed >= delay { available.min(10) } else { 0 };
                (delay, batch_size)
            }
            ChunkingStrategy::Instant => {
                let batch_size = available;
                (Duration::ZERO, batch_size)
            }
        };

        if batch_size > 0 {
            let count = batch_size.min(self.pending.len());
            let chars: String = self.pending.drain(..count).collect();
            self.last_emit = now;
            Some(chars)
        } else {
            None
        }
    }
}
```

### 5.3 Table Holdback

```rust
pub struct TableHoldback {
    buffer: Vec<String>,
    column_widths: Vec<usize>,
    revealed_rows: usize,
}

impl TableHoldback {
    /// Don't emit table until header row complete
    pub fn add_line(&mut self, line: String) -> Option<Vec<String>> {
        self.buffer.push(line);

        if self.is_table_complete() {
            Some(self.buffer.drain(..).collect())
        } else {
            None
        }
    }

    fn is_table_complete(&self) -> bool {
        // Check if we've received delimiter row (---+---+---)
        self.buffer.last()
            .map(|l| l.chars().all(|c| c == '-' || c == '+' || c == ' '))
            .unwrap_or(false)
    }
}
```

### 5.4 Shimmer Text for Loading

**Pattern from OpenCode**: TextShimmer with blur + opacity animation.

```rust
pub struct ShimmerText {
    text: String,
    frame: usize,
}

impl ShimmerText {
    const SHIMMER_FRAMES: [&str; 4] = ["   ", ".  ", ".. ", "..."];

    pub fn display_lines(&self, width: usize) -> Vec<Line> {
        let shimmer = SHIMMER_FRAMES[self.frame % 4];
        let text = format!("{}{}", self.text, shimmer);
        vec![Line::from(styled(text, Theme::Muted))]
    }
}
```

---

## 6. Tool Display Improvements

### 6.1 Tool-Specific Renderers

**Pattern from OpenCode**: Each tool type has custom renderer.

```rust
pub trait ToolRenderer: Send + Sync {
    fn tool_name(&self) -> &str;
    fn render_summary(&self, tool: &ToolCall) -> CellContent;
    fn render_result(&self, result: &ToolResult) -> CellContent;
}

pub struct BashRenderer;
pub struct ReadRenderer;
pub struct EditRenderer;
pub struct GrepRenderer;

impl ToolRenderer for BashRenderer {
    fn tool_name(&self) -> &str { "Bash" }

    fn render_summary(&self, tool: &ToolCall) -> CellContent {
        let cmd = tool.args.get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let truncated = if cmd.len() > 60 { &cmd[..60] } else { cmd };
        CellContent::Lines(vec![
            Line::from(styled("▶ Bash", Theme::Tool)),
            Line::from(format!("  $ {}", truncated)),
        ])
    }

    fn render_result(&self, result: &ToolResult) -> CellContent {
        // Render with exit code, stdout/stderr sections
    }
}
```

### 6.2 Collapsible Tool Output

```rust
pub struct CollapsibleTool {
    header: Box<dyn ToolRenderer>,
    expanded: bool,
    result: ToolResult,
}

impl CollapsibleTool {
    pub fn render(&self, width: usize) -> Vec<Line> {
        let summary = self.header.render_summary(&self.tool_call);

        if self.expanded {
            vec![summary]
                .into_iter()
                .chain(self.result.lines(width))
                .collect()
        } else {
            // Only header with expand indicator
            vec![summary, Line::from("  ▶展开".into())]
        }
    }
}
```

### 6.3 Context Tool Grouping

**Pattern from OpenCode**: Groups read/list/grep with aggregate summary.

```rust
pub struct ContextToolGroup {
    tools: Vec<ToolCall>,
    results: Vec<ToolResult>,
}

impl ContextToolGroup {
    pub fn summary(&self) -> String {
        let reads = self.tools.iter().filter(|t| t.name == "Read").count();
        let writes = self.tools.iter().filter(|t| t.name == "Write").count();

        match (reads, writes) {
            (n, 0) if n > 0 => format!("Read {} files", n),
            (0, n) if n > 0 => format!("Wrote {} files", n),
            (r, w) => format!("Read {} files, wrote {} files", r, w),
        }
    }
}
```

### 6.4 Tool Execution Progress

```rust
pub struct ToolProgress {
    tool_id: Uuid,
    tool_name: String,
    status: ToolStatus,
    start_time: Instant,
    elapsed_ms: u64,
}

impl ToolProgress {
    pub fn tick(&mut self) {
        if matches!(self.status, ToolStatus::Running) {
            self.elapsed_ms = self.start_time.elapsed().as_millis() as u64;
        }
    }

    pub fn format_duration(ms: u64) -> String {
        if ms < 1000 {
            format!("{}ms", ms)
        } else {
            format!("{:.1}s", ms as f64 / 1000.0)
        }
    }
}
```

---

## 7. Implementation Priority

### P0: Quick Wins (1-2 days each)

| Improvement | Effort | Impact | Notes |
|------------|--------|--------|-------|
| Message separator with metrics | 0.5d | High | Single cell type, clear spec |
| Error cells (red ■) | 0.25d | Medium | Already partially exists |
| Status state machine | 1d | High | Improves UX feedback significantly |
| Input history (Up/Down) | 1d | High | Core UX improvement |
| PageUp/PageDown scrolling | 0.5d | Medium | Expected behavior |
| Copy last response shortcut | 0.25d | Medium | Common workflow |

### P1: Medium Effort (3-5 days each)

| Improvement | Effort | Impact | Notes |
|------------|--------|--------|-------|
| Live status row with elapsed | 2d | High | Requires timer tick integration |
| Tool progress gauge | 2d | High | Animated cell type |
| Shimmer text for loading | 1d | Medium | Animated placeholder |
| Follow-bottom behavior | 1d | Medium | Scroll anchor logic |
| Thinking/reasoning blocks | 2d | Medium | New cell type, collapsible |
| Copy icon with toggle | 1d | Medium | State management for UI |

### P2: Large Effort (1-2 weeks each)

| Improvement | Effort | Impact | Notes |
|------------|--------|--------|-------|
| Virtualized scrolling | 1w | Very High | Performance critical for long sessions |
| Queue-based streaming | 1w | High | Core architecture change |
| Tool-specific renderers | 1w | High | Requires registry + all renderers |
| Smooth scrolling | 3d | Medium | Animation system |
| Command system with autocomplete | 3d | Medium | SelectList component needed |
| Table holdback | 2d | Medium | Streaming parser changes |

---

## 8. Architecture Recommendations

### 8.1 Unified Cell Trait

```rust
pub trait FeedCell: Send + Sync {
    fn height(&self, width: usize) -> usize;
    fn render(&self, width: usize, theme: &Theme) -> Vec<Line>;
    fn id(&self) -> Option<Uuid>;
}

pub trait AnimatedCell: FeedCell {
    fn tick(&mut self) -> bool;  // Returns true if render needed
    fn is_done(&self) -> bool;
}
```

### 8.2 Event Bus for Status

```rust
pub enum FeedEvent {
    MessageReceived(Message),
    StreamingStarted(Uuid),
    StreamingChunk(Uuid, String),
    StreamingComplete(Uuid),
    ToolStarted(Uuid, ToolCall),
    ToolProgress(Uuid, ToolStatus),
    ToolComplete(Uuid, ToolResult),
    ErrorOccurred(Error),
}

pub type EventBus = Broker<FeedEvent>;
```

### 8.3 Abort Signal Propagation

```rust
pub struct AbortController {
    inner: Arc<AtomicBool>,
}

impl AbortController {
    pub fn abort(&self) {
        self.inner.store(true, Ordering::SeqCst);
    }

    pub fn is_aborted(&self) -> bool {
        self.inner.load(Ordering::SeqCst)
    }
}

// Tool calls check this before/during execution
// Streaming checks this on each chunk
```

---

## 9. Detailed Implementation: Status State Machine

```rust
use std::time::{Duration, Instant};

#[derive(Clone, Debug, PartialEq)]
pub enum StatusPhase {
    Idle,
    Thinking,
    Working,
    Done,
    Error(String),
}

pub struct StatusStateMachine {
    phase: StatusPhase,
    started_at: Option<Instant>,
    phase_started_at: Option<Instant>,
}

impl StatusStateMachine {
    pub fn new() -> Self {
        Self {
            phase: StatusPhase::Idle,
            started_at: None,
            phase_started_at: None,
        }
    }

    pub fn transition_to(&mut self, new_phase: StatusPhase) {
        let now = Instant::now();
        self.phase = new_phase.clone();
        self.phase_started_at = Some(now);

        if matches!(new_phase, StatusPhase::Thinking) && self.started_at.is_none() {
            self.started_at = Some(now);
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.started_at
            .map(|s| s.elapsed())
            .unwrap_or_default()
    }

    pub fn phase_elapsed(&self) -> Duration {
        self.phase_started_at
            .map(|s| s.elapsed())
            .unwrap_or_default()
    }

    pub fn display_text(&self) -> String {
        match &self.phase {
            StatusPhase::Idle => "Ready".into(),
            StatusPhase::Thinking => {
                let elapsed = self.phase_elapsed();
                format!("Thinking {:.0}s", elapsed.as_secs_f64())
            }
            StatusPhase::Working => {
                let elapsed = self.phase_elapsed();
                format!("Working {:.0}s", elapsed.as_secs_f64())
            }
            StatusPhase::Done => {
                let total = self.elapsed();
                format!("Done in {:.1}s", total.as_secs_f64())
            }
            StatusPhase::Error(msg) => format!("Error: {}", msg),
        }
    }

    pub fn indicator_char(&self) -> &'static str {
        match &self.phase {
            StatusPhase::Idle => "·",
            StatusPhase::Thinking => "⠋",
            StatusPhase::Working => "⠙",
            StatusPhase::Done => "✓",
            StatusPhase::Error => "✗",
        }
    }
}
```

---

## 10. Detailed Implementation: Streaming Cursor

**Pattern from Pi**: Blinking `▊` block at end of streaming tail.

```rust
pub struct StreamingCursor {
    visible: bool,
    timer: Interval,
}

impl StreamingCursor {
    pub fn tick(&mut self) {
        self.visible = !self.visible;
    }

    pub fn char(&self) -> &'static str {
        if self.visible { "▊" } else { " " }
    }
}
```

**Integration with streaming output**:

```rust
pub struct StreamingTextCell {
    stable_content: String,
    pending_chars: Vec<char>,
    cursor: StreamingCursor,
    pacing: PacedMarkdown,
}

impl AnimatedCell for StreamingTextCell {
    fn tick(&mut self) -> bool {
        self.cursor.tick();
        if let Some(chars) = self.pacing.emit_pending() {
            self.stable_content.push_str(&chars.collect::<String>());
            return true;
        }
        false
    }

    fn is_done(&self) -> bool {
        self.pending_chars.is_empty() && self.cursor.visible
    }
}
```

---

## 11. Keyboard Layering

**Pattern from Codex**: Layered key handling with priority.

```rust
pub enum KeyLayer {
    App,        // Global: Ctrl-C, Ctrl-Q, etc.
    Chat,       // Chat-level: arrow keys, PageUp/Down
    Composer,   // Input editing: Emacs-style bindings
    Editor,     // Code editor: VSCode-like
    Pager,      // Help/docs: Vim-style navigation
}

pub struct KeyDispatcher {
    handlers: HashMap<KeyLayer, Box<dyn KeyHandler>>,
    active_layer: KeyLayer,
}

impl KeyDispatcher {
    pub fn handle(&self, key: KeyEvent) -> bool {
        // Try active layer first
        if self.handlers[&self.active_layer].handle(key) {
            return true;
        }

        // Fall back to lower layers
        for layer in self.layers_below(self.active_layer) {
            if self.handlers[layer].handle(key) {
                return true;
            }
        }

        false
    }
}
```

---

## 12. Clipboard Backend Selection

**Pattern from Codex**: Multi-backend clipboard with fallback chain.

```rust
pub enum ClipboardBackend {
    Arboard,        // macOS/Linux
    Wsl,            // Windows Subsystem for Linux
    Osc52,          // Terminal escape sequence
    XSel,           // X11 selection
}

pub struct Clipboard {
    backend: ClipboardBackend,
}

impl Clipboard {
    pub fn new() -> Self {
        let backend = if cfg!(target_os = "windows") {
            ClipboardBackend::Wsl
        } else if std::env::var("TERM_PROGRAM").is_ok() {
            ClipboardBackend::Osc52
        } else {
            ClipboardBackend::Arboard
        };

        Self { backend }
    }

    pub fn write(&self, text: &str) -> Result<()> {
        match &self.backend {
            ClipboardBackend::Arboard => arboard::Clipboard::new()?.set_text(text),
            ClipboardBackend::Osc52 => self.write_osc52(text),
            ClipboardBackend::Wsl => self.write_wsl(text),
            ClipboardBackend::XSel => self.write_xsel(text),
        }
    }
}
```

---

## 13. Differential Rendering

**Pattern from Pi**: Only changed lines re-rendered.

```rust
pub struct RenderCache {
    cached_lines: Vec<Line>,
    cached_width: usize,
    dirty: bool,
}

impl RenderCache {
    pub fn get_or_render(&mut self, cell: &dyn FeedCell, width: usize) -> &[Line] {
        if self.dirty || self.cached_width != width {
            self.cached_lines = cell.render(width);
            self.cached_width = width;
            self.dirty = false;
        }
        &self.cached_lines
    }

    pub fn invalidate(&mut self) {
        self.dirty = true;
    }
}

pub struct DiffRenderer {
    cache: HashMap<Uuid, RenderCache>,
}

impl DiffRenderer {
    pub fn render_changed(&mut self, cells: &[Box<dyn FeedCell>], width: usize) -> Vec<(usize, Vec<Line>)> {
        let mut changes = Vec::new();

        for (i, cell) in cells.iter().enumerate() {
            let id = cell.id().unwrap_or(Uuid::from_u64(i as u64));
            let cell_cache = self.cache.entry(id).or_default();

            let new_lines = cell.render(width);
            if &cell_cache.cached_lines != &new_lines || cell_cache.cached_width != width {
                changes.push((i, new_lines.clone()));
                cell_cache.cached_lines = new_lines;
                cell_cache.cached_width = width;
            }
        }

        changes
    }
}
```

---

## 14. Summary

This synthesis identifies 25+ patterns across three agents that Runie should consider:

**High-priority gaps**:
1. Status state machine with live timer
2. Input history navigation
3. Message separators with metrics
4. Tool execution progress gauges
5. Queue-based streaming with timestamps

**Medium-priority gaps**:
6. Thinking/reasoning blocks
7. Smooth scrolling with follow-bottom
8. Command system with autocomplete
9. Shimmer text for loading states
10. Copy icon with toggle feedback

**Architecture changes needed**:
- Unified FeedCell trait with AnimatedCell extension
- Event bus for decoupled components
- Abort signal propagation throughout stack
- Differential rendering for performance
- Clipboard backend abstraction

The P0 items can be implemented in 1-2 weeks with significant UX impact. P2 items require 3-4 weeks but provide foundational improvements that enable future features.
