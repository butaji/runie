//! Enhanced headless runtime with structured output (from Grok Build)

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Output format for headless mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    /// Human-readable plain text
    #[default]
    Plain,
    /// Structured JSON with usage/cost
    Json,
    /// Streaming JSON response
    StreamingJson,
}

impl OutputFormat {
    /// Get the content type for HTTP responses
    pub fn content_type(&self) -> &'static str {
        match self {
            OutputFormat::Plain => "text/plain",
            OutputFormat::Json | OutputFormat::StreamingJson => "application/json",
        }
    }

    /// Check if this format uses streaming
    pub fn is_streaming(&self) -> bool {
        matches!(self, OutputFormat::StreamingJson)
    }
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Plain => write!(f, "plain"),
            OutputFormat::Json => write!(f, "json"),
            OutputFormat::StreamingJson => write!(f, "stream"),
        }
    }
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "plain" | "text" => Ok(OutputFormat::Plain),
            "json" => Ok(OutputFormat::Json),
            "stream" | "streaming" => Ok(OutputFormat::StreamingJson),
            _ => Err(format!("Unknown output format: {}", s)),
        }
    }
}

/// Headless mode options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeadlessOptions {
    /// Session ID for resuming
    pub session_id: Option<String>,
    /// Resume from a previous session
    pub resume: Option<String>,
    /// YOLO mode (skip confirmations)
    pub yolo: bool,
    /// Trust mode (skip security prompts)
    pub trust: bool,
    /// Output format
    pub output_format: OutputFormat,
    /// JSON schema for structured output
    pub json_schema: Option<serde_json::Value>,
    /// Model override
    pub model: Option<String>,
    /// Rules/prompt to prepend
    pub rules: Option<String>,
    /// Maximum turns
    pub max_turns: Option<u32>,
    /// Permission mode
    pub permission_mode: Option<String>,
    /// Reasoning effort
    pub reasoning_effort: Option<String>,
    /// Self-verify responses
    pub self_verify: bool,
    /// Best of N sampling
    pub best_of_n: Option<u32>,
    /// Wait for background tasks
    pub wait_for_background: bool,
    /// Timeout for background wait
    pub background_wait_timeout: Duration,
    /// Allowed patterns (regex)
    pub allow_patterns: Vec<String>,
    /// Denied patterns (regex)
    pub deny_patterns: Vec<String>,
    /// Input file
    pub input_file: Option<PathBuf>,
    /// Output file
    pub output_file: Option<PathBuf>,
    /// Attachments
    pub attachments: Vec<PathBuf>,
    /// System prompt override
    pub system_prompt: Option<String>,
    /// Temperature
    pub temperature: Option<f32>,
    /// Max tokens
    pub max_tokens: Option<u32>,
    /// Verbose output
    pub verbose: bool,
    /// Dry run (don't execute tools)
    pub dry_run: bool,
}

impl Default for HeadlessOptions {
    fn default() -> Self {
        Self {
            session_id: None,
            resume: None,
            yolo: false,
            trust: false,
            output_format: OutputFormat::Plain,
            json_schema: None,
            model: None,
            rules: None,
            max_turns: None,
            permission_mode: None,
            reasoning_effort: None,
            self_verify: false,
            best_of_n: None,
            wait_for_background: true,
            background_wait_timeout: Duration::from_secs(300),
            allow_patterns: Vec::new(),
            deny_patterns: Vec::new(),
            input_file: None,
            output_file: None,
            attachments: Vec::new(),
            system_prompt: None,
            temperature: None,
            max_tokens: None,
            verbose: false,
            dry_run: false,
        }
    }
}

impl HeadlessOptions {
    /// Enable yolo mode
    pub fn yolo(mut self) -> Self {
        self.yolo = true;
        self
    }

    /// Enable trust mode
    pub fn trust(mut self) -> Self {
        self.trust = true;
        self
    }

    /// Set output format
    pub fn output_format(mut self, format: OutputFormat) -> Self {
        self.output_format = format;
        self
    }

    /// Set JSON schema
    pub fn json_schema(mut self, schema: serde_json::Value) -> Self {
        self.json_schema = Some(schema);
        self
    }

    /// Set model
    pub fn model(mut self, model: &str) -> Self {
        self.model = Some(model.to_string());
        self
    }

    /// Set max turns
    pub fn max_turns(mut self, turns: u32) -> Self {
        self.max_turns = Some(turns);
        self
    }

    /// Enable self-verify
    pub fn self_verify(mut self) -> Self {
        self.self_verify = true;
        self
    }

    /// Set best of N
    pub fn best_of_n(mut self, n: u32) -> Self {
        self.best_of_n = Some(n);
        self
    }

    /// Add an allow pattern
    pub fn allow_pattern(mut self, pattern: &str) -> Self {
        self.allow_patterns.push(pattern.to_string());
        self
    }

    /// Add a deny pattern
    pub fn deny_pattern(mut self, pattern: &str) -> Self {
        self.deny_patterns.push(pattern.to_string());
        self
    }

    /// Set input file
    pub fn input_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.input_file = Some(path.into());
        self
    }

    /// Set output file
    pub fn output_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.output_file = Some(path.into());
        self
    }

    /// Add an attachment
    pub fn attach(mut self, path: impl Into<PathBuf>) -> Self {
        self.attachments.push(path.into());
        self
    }

    /// Set system prompt
    pub fn system_prompt(mut self, prompt: &str) -> Self {
        self.system_prompt = Some(prompt.to_string());
        self
    }

    /// Set temperature
    pub fn temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp);
        self
    }

    /// Set max tokens
    pub fn max_tokens(mut self, tokens: u32) -> Self {
        self.max_tokens = Some(tokens);
        self
    }

    /// Enable verbose output
    pub fn verbose(mut self) -> Self {
        self.verbose = true;
        self
    }

    /// Enable dry run
    pub fn dry_run(mut self) -> Self {
        self.dry_run = true;
        self
    }

    /// Set permission mode
    pub fn permission_mode(mut self, mode: &str) -> Self {
        self.permission_mode = Some(mode.to_string());
        self
    }

    /// Check if auto-approve is enabled
    pub fn is_auto_approve(&self) -> bool {
        self.yolo || self.permission_mode.as_deref() == Some("auto")
    }

    /// Get effective permission mode
    pub fn effective_permission_mode(&self) -> &str {
        self.permission_mode
            .as_deref()
            .unwrap_or(if self.yolo { "auto" } else { "manual" })
    }
}

/// Structured output with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredOutput {
    /// Main text response
    pub text: String,
    /// Token usage information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<UsageInfo>,
    /// Estimated cost
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<f64>,
    /// Duration in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// Structured data from JSON schema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structured_data: Option<serde_json::Value>,
    /// Session ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// Model used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Stop reason
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
    /// Tool calls made
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCallInfo>>,
    /// Warnings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warnings: Option<Vec<String>>,
}

impl StructuredOutput {
    /// Create a simple text output
    pub fn text<S: Into<String>>(text: S) -> Self {
        Self {
            text: text.into(),
            usage: None,
            cost: None,
            duration_ms: None,
            structured_data: None,
            session_id: None,
            model: None,
            stop_reason: None,
            tool_calls: None,
            warnings: None,
        }
    }

    /// Create from a plain response
    pub fn from_response(response: &str) -> Self {
        Self::text(response)
    }

    /// Add usage info
    pub fn with_usage(mut self, usage: UsageInfo) -> Self {
        self.usage = Some(usage);
        self
    }

    /// Add cost
    pub fn with_cost(mut self, cost: f64) -> Self {
        self.cost = Some(cost);
        self
    }

    /// Add duration
    pub fn with_duration(mut self, ms: u64) -> Self {
        self.duration_ms = Some(ms);
        self
    }

    /// Add structured data
    pub fn with_structured_data(mut self, data: serde_json::Value) -> Self {
        self.structured_data = Some(data);
        self
    }

    /// Format as plain text
    pub fn to_plain(&self) -> String {
        if let Some(ref data) = self.structured_data {
            serde_json::to_string_pretty(data).unwrap_or_else(|_| self.text.clone())
        } else {
            self.text.clone()
        }
    }

    /// Format as JSON string
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| r#"{"error": "serialization failed"}"#.to_string())
    }
}

/// Token usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageInfo {
    /// Input tokens
    pub input_tokens: u32,
    /// Output tokens
    pub output_tokens: u32,
    /// Reasoning tokens (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_tokens: Option<u32>,
    /// Total tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<u32>,
}

impl UsageInfo {
    /// Create usage info from values
    pub fn new(input: u32, output: u32) -> Self {
        Self {
            input_tokens: input,
            output_tokens: output,
            reasoning_tokens: None,
            total_tokens: Some(input.saturating_add(output)),
        }
    }

    /// Create with reasoning tokens
    pub fn with_reasoning(input: u32, output: u32, reasoning: u32) -> Self {
        Self {
            input_tokens: input,
            output_tokens: output,
            reasoning_tokens: Some(reasoning),
            total_tokens: Some(input.saturating_add(output).saturating_add(reasoning)),
        }
    }
}

/// Tool call information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallInfo {
    /// Tool name
    pub name: String,
    /// Tool arguments
    pub arguments: serde_json::Value,
    /// Whether the tool call succeeded
    pub success: bool,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Headless session state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Session is starting
    Starting,
    /// Session is running
    Running,
    /// Session is waiting for input
    WaitingForInput,
    /// Session is waiting for tool completion
    WaitingForTool,
    /// Session is complete
    Complete,
    /// Session was aborted
    Aborted,
    /// Session encountered an error
    Error,
}

impl SessionState {
    /// Check if session is active
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            SessionState::Running
                | SessionState::WaitingForInput
                | SessionState::WaitingForTool
        )
    }

    /// Get a human-readable label
    pub fn label(&self) -> &'static str {
        match self {
            SessionState::Starting => "Starting",
            SessionState::Running => "Running",
            SessionState::WaitingForInput => "Waiting for input",
            SessionState::WaitingForTool => "Running tools",
            SessionState::Complete => "Complete",
            SessionState::Aborted => "Aborted",
            SessionState::Error => "Error",
        }
    }
}

/// Headless session handle
#[derive(Debug, Clone)]
pub struct HeadlessSession {
    /// Session ID
    pub id: String,
    /// Session state
    pub state: SessionState,
    /// Options used for this session
    pub options: HeadlessOptions,
    /// Created at timestamp
    pub created_at: std::time::Instant,
}

impl HeadlessSession {
    /// Get session age
    pub fn age(&self) -> Duration {
        self.created_at.elapsed()
    }

    /// Check if session is still active
    pub fn is_active(&self) -> bool {
        self.state.is_active()
    }
}

/// Permission mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PermissionMode {
    /// Manual approval for all operations
    #[default]
    Manual,
    /// Automatic approval based on classifier
    Auto,
    /// Automatic approval with dangerous pattern blocking
    SafeAuto,
    /// Approve all operations (dangerous)
    BypassAll,
}

impl PermissionMode {
    /// Parse from string
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "manual" | "ask" => Some(PermissionMode::Manual),
            "auto" | "automatic" => Some(PermissionMode::Auto),
            "safe-auto" | "safe" => Some(PermissionMode::SafeAuto),
            "bypass" | "bypass-all" | "yes" => Some(PermissionMode::BypassAll),
            _ => None,
        }
    }

    /// Check if approvals should be automatic
    pub fn is_auto(&self) -> bool {
        !matches!(self, PermissionMode::Manual)
    }

    /// Check if dangerous patterns should be blocked
    pub fn blocks_dangerous(&self) -> bool {
        matches!(self, PermissionMode::Manual | PermissionMode::SafeAuto)
    }
}

impl std::fmt::Display for PermissionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PermissionMode::Manual => write!(f, "manual"),
            PermissionMode::Auto => write!(f, "auto"),
            PermissionMode::SafeAuto => write!(f, "safe-auto"),
            PermissionMode::BypassAll => write!(f, "bypass-all"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_format_parse() {
        assert_eq!("plain".parse::<OutputFormat>().unwrap(), OutputFormat::Plain);
        assert_eq!("json".parse::<OutputFormat>().unwrap(), OutputFormat::Json);
        assert_eq!("stream".parse::<OutputFormat>().unwrap(), OutputFormat::StreamingJson);
        assert!("invalid".parse::<OutputFormat>().is_err());
    }

    #[test]
    fn test_output_format_content_type() {
        assert_eq!(OutputFormat::Plain.content_type(), "text/plain");
        assert_eq!(OutputFormat::Json.content_type(), "application/json");
        assert_eq!(OutputFormat::StreamingJson.content_type(), "application/json");
    }

    #[test]
    fn test_headless_options_builder() {
        let opts = HeadlessOptions::default()
            .yolo()
            .model("claude-3-opus")
            .max_turns(10)
            .self_verify()
            .allow_pattern("ls")
            .deny_pattern("rm -rf");

        assert!(opts.yolo);
        assert_eq!(opts.model, Some("claude-3-opus".to_string()));
        assert_eq!(opts.max_turns, Some(10));
        assert!(opts.self_verify);
        assert!(opts.allow_patterns.contains(&"ls".to_string()));
        assert!(opts.deny_patterns.contains(&"rm -rf".to_string()));
    }

    #[test]
    fn test_structured_output() {
        let output = StructuredOutput::text("Hello, world!")
            .with_usage(UsageInfo::new(100, 50))
            .with_cost(0.001)
            .with_duration(500);

        assert_eq!(output.text, "Hello, world!");
        assert!(output.usage.is_some());
        assert!(output.cost.is_some());
        assert_eq!(output.duration_ms, Some(500));
    }

    #[test]
    fn test_usage_info() {
        let usage = UsageInfo::new(100, 50);
        assert_eq!(usage.input_tokens, 100);
        assert_eq!(usage.output_tokens, 50);
        assert_eq!(usage.total_tokens, Some(150));

        let usage_with_reasoning = UsageInfo::with_reasoning(100, 50, 200);
        assert_eq!(usage_with_reasoning.reasoning_tokens, Some(200));
        assert_eq!(usage_with_reasoning.total_tokens, Some(350));
    }

    #[test]
    fn test_session_state() {
        assert!(SessionState::Running.is_active());
        assert!(SessionState::WaitingForInput.is_active());
        assert!(!SessionState::Complete.is_active());
        assert!(!SessionState::Aborted.is_active());

        assert_eq!(SessionState::Running.label(), "Running");
    }

    #[test]
    fn test_permission_mode() {
        assert_eq!(PermissionMode::parse("manual"), Some(PermissionMode::Manual));
        assert_eq!(PermissionMode::parse("auto"), Some(PermissionMode::Auto));
        assert_eq!(PermissionMode::parse("safe-auto"), Some(PermissionMode::SafeAuto));
        assert_eq!(PermissionMode::parse("bypass"), Some(PermissionMode::BypassAll));

        assert!(PermissionMode::Auto.is_auto());
        assert!(!PermissionMode::Manual.is_auto());
        assert!(PermissionMode::Manual.blocks_dangerous());
        assert!(!PermissionMode::BypassAll.blocks_dangerous());
    }

    #[test]
    fn test_is_auto_approve() {
        let opts = HeadlessOptions::default();
        assert!(!opts.is_auto_approve());

        let opts = HeadlessOptions::default().yolo();
        assert!(opts.is_auto_approve());

        let opts = HeadlessOptions::default().permission_mode("auto");
        assert!(opts.is_auto_approve());
    }
}
