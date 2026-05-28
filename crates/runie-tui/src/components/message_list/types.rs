/// Braille spinner frames (10 frames)
pub const BRAILLE_FRAMES: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

/// Braille spinner frames (10 frames) - counter-clockwise (rewind)
pub const REVERSE_BRAILLE_FRAMES: [char; 10] = ['⠏', '⠇', '⠧', '⠦', '⠴', '⠼', '⠸', '⠹', '⠙', '⠋'];

/// Plan step status
#[derive(Debug, Clone, PartialEq)]
pub enum PlanStatus {
    Pending,
    Active,
    Complete,
}

#[derive(Clone)]
pub struct MessageList {
    pub messages: Vec<MessageItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MessageItem {
    User { text: String, model: Option<String>, timestamp: Option<String> },
    Assistant { text: String, model: Option<String>, timestamp: Option<String> },
    Thought { duration_secs: f32 },
    /// Separator between conversation turns showing elapsed time and metrics
    Separator { elapsed_secs: u64, tool_calls: usize, tokens_used: Option<usize> },
    ToolCall { name: String, args: String, result: Option<String>, is_error: bool },
    Edit { filename: String, diff: Option<String> },
    System { text: String },
    // P2-1: Structured error with recoverable flag for better error presentation
    Error { message: String, recoverable: bool },
    ToolRunning { name: String, args: String, duration_ms: u64 },
    ToolComplete { name: String, result: String, lines: Option<usize> },
    PlanStep { step: usize, text: String, status: PlanStatus },
    Interrupt,
    Rewind { steps: usize },
}

impl Default for MessageList {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
        }
    }
}
