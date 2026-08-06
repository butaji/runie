//! YAML-driven e2e test runner for `runie-tui`.
//!
//! Each YAML fixture under `tests/yaml_fixtures/*.yaml` is loaded, parsed into
//! a `Scenario`, then executed against a real `LoopActor` + `EventRenderer`.
//! The runner applies the fixture's assertions against the recorded events
//! and the rendered scrollback.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use crate::event_renderer::EventRenderer;
use crate::widgets::{FeedSnapshot, Line, LineKind, Scrollback, ScrollbackMsg, ToolBlock};
use parking_lot::Mutex;
use ratatui::buffer::Buffer;
use runie_core::events::EventBus;
use runie_core::provider::stream_fn::{AssistantMessageEventStream, StreamError, StreamFn};
use runie_core::provider::ProviderActor;
use runie_core::queues::{FollowUpQueueActor, SteeringQueueActor};
use runie_core::r#loop::{LoopActor, LoopDeps};
use runie_core::state::AgentStateActor;
use runie_core::tools::executor::ToolExecHooks;
use runie_core::tools::{ToolExecutorActor, ToolRegistry};
use runie_core::types::{
    AgentContext, AgentEvent, AgentMessage, AgentTool, AgentToolResult, AssistantMessage,
    AssistantMessageEvent, Model, SimpleStreamOptions, StopReason, ThinkingLevel,
    ToolExecutionMode, ToolResultContent, Usage, UserContent, UserMessage, WaitingReason,
};
use serde::Deserialize;
use tokio::sync::broadcast;
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Deserialize)]
pub struct Scenario {
    pub name: String,
    pub initial_prompt: Option<String>,
    #[serde(default)]
    pub follow_up: Vec<String>,
    #[serde(default)]
    pub steering_mode: Option<runie_core::types::QueueMode>,
    #[serde(default)]
    pub follow_up_mode: Option<runie_core::types::QueueMode>,
    #[serde(default)]
    pub tools: Vec<ToolSpec>,
    #[serde(default)]
    pub context: ContextSpec,
    pub events: Vec<EventSpec>,
    /// Capture the frame after tool execution while the next model request is
    /// still pending. This models Grok's stable waiting/feed boundary.
    #[serde(default)]
    pub capture_while_waiting: bool,
    /// Deterministic user-row clock for full-frame replay assertions.
    pub prompt_timestamp: Option<String>,
    #[serde(default)]
    pub assertions: Assertions,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct ContextSpec {
    #[serde(default)]
    pub system_prompt: String,
    #[serde(default)]
    pub messages: Vec<String>,
    /// Explicit Pi-style empty tool set; omission keeps registered defaults.
    #[serde(default)]
    pub disable_tools: bool,
}

impl Scenario {
    fn agent_context(&self) -> AgentContext {
        AgentContext {
            system_prompt: self.context.system_prompt.clone(),
            messages: self
                .context
                .messages
                .iter()
                .map(|text| {
                    AgentMessage::User(UserMessage {
                        content: vec![UserContent::Text { text: text.clone() }],
                        timestamp: 0,
                    })
                })
                .collect(),
            // Omitted in YAML: let the scenario's registered executor tools
            // supply the Pi-compatible default.
            tools: self.context.disable_tools.then_some(Vec::new()),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    #[serde(default = "default_tool_kind")]
    pub kind: String,
}

fn default_tool_kind() -> String {
    "echo".into()
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum EventSpec {
    Bare(String),
    TextDelta {
        text_delta: String,
    },
    TextStart {
        text_start: TextStartSpec,
    },
    TextEnd {
        text_end: TextEndSpec,
    },
    ThinkingDelta {
        thinking_delta: String,
    },
    ThinkingStart {
        thinking_start: ThinkingStartSpec,
    },
    ThinkingEnd {
        thinking_end: ThinkingEndSpec,
    },
    ToolCall {
        tool_call: ToolCallSpec,
    },
    ToolCallStart {
        tool_call_start: ToolCallSectionSpec,
    },
    ToolCallDelta {
        tool_call_delta: ToolCallSectionSpec,
    },
    ToolCallEnd {
        tool_call_end: ToolCallSectionSpec,
    },
    ToolUpdate {
        tool_update: ToolUpdateSpec,
    },
    ToolSeed {
        tool_seed: ToolSeedSpec,
    },
    Done {
        done: DoneSpec,
    },
    Error {
        error: String,
    },
    Waiting {
        waiting: String,
    },
    Theme {
        theme: String,
    },
    ThinkingLevel {
        thinking_level: String,
    },
    ToolMode {
        tool_mode: ToolModeSpec,
    },
    ToolFold {
        tool_fold: String,
    },
    ToolSelect {
        tool_select: String,
    },
    Scroll {
        scroll: i32,
    },
    FollowLatest {
        follow_latest: bool,
    },
    BackgroundStart {
        background_start: BackgroundStartSpec,
    },
    BackgroundProgress {
        background_progress: BackgroundProgressSpec,
    },
    BackgroundEnd {
        background_end: BackgroundEndSpec,
    },
    BackgroundCancel {
        background_cancel: BackgroundCancelSpec,
    },
    WorkflowStart {
        workflow_start: WorkflowStartSpec,
    },
    WorkflowProgress {
        workflow_progress: WorkflowProgressSpec,
    },
    WorkflowEnd {
        workflow_end: WorkflowEndSpec,
    },
}

#[derive(Debug, Deserialize, Clone)]
pub struct BackgroundStartSpec {
    pub work_id: String,
    pub description: String,
    #[serde(default)]
    pub background: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BackgroundProgressSpec {
    pub work_id: String,
    pub description: String,
    pub activity: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BackgroundEndSpec {
    pub work_id: String,
    pub description: String,
    #[serde(default)]
    pub is_error: bool,
    #[serde(default)]
    pub elapsed_ms: Option<u64>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BackgroundCancelSpec {
    pub work_id: String,
    pub description: String,
    #[serde(default)]
    pub elapsed_ms: Option<u64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WorkflowStartSpec {
    pub run_id: String,
    pub name: String,
    pub objective: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WorkflowProgressSpec {
    pub run_id: String,
    pub phase: String,
    pub state: String,
    #[serde(default)]
    pub active_agents: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WorkflowEndSpec {
    pub run_id: String,
    pub status: String,
    #[serde(default)]
    pub elapsed_ms: Option<u64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ToolModeSpec {
    pub tool_call_id: String,
    pub mode: runie_core::types::ToolDisplayMode,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ToolUpdateSpec {
    pub tool_call_id: String,
    pub tool_name: String,
    #[serde(default)]
    pub args: serde_json::Value,
    #[serde(default)]
    pub partial_result: serde_json::Value,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ToolSeedSpec {
    pub tool_call_id: String,
    pub header: String,
    /// Test-only lifecycle fact for inspecting a live card before completion.
    #[serde(default)]
    pub running: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ThinkingStartSpec {
    #[serde(default)]
    pub index: usize,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ThinkingEndSpec {
    #[serde(default)]
    pub index: usize,
    pub content: String,
    #[serde(default)]
    pub elapsed_ms: Option<u64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TextStartSpec {
    #[serde(default)]
    pub index: usize,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TextEndSpec {
    #[serde(default)]
    pub index: usize,
    pub content: String,
}

fn waiting_name(name: &str) -> WaitingReason {
    match name {
        "subagent" => WaitingReason::Subagent,
        "tasks_complete" => WaitingReason::TasksComplete,
        "sleep" => WaitingReason::Sleep,
        _ => WaitingReason::Model,
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct DoneSpec {
    #[serde(default)]
    pub stop_reason: StopReasonSpec,
    /// Provider usage is part of the terminal event and must be fixture-owned
    /// for deterministic footer parity; omitted usage keeps the zero default.
    #[serde(default)]
    pub usage: Usage,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ToolCallSpec {
    pub name: String,
    #[serde(default)]
    pub args: serde_json::Value,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ToolCallSectionSpec {
    #[serde(default)]
    pub index: usize,
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub arguments: serde_json::Value,
    #[serde(default)]
    pub delta: String,
}

#[derive(Debug, Default, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum StopReasonSpec {
    #[default]
    Stop,
    ToolUse,
    MaxTokens,
    Aborted,
}

impl From<&StopReasonSpec> for StopReason {
    fn from(s: &StopReasonSpec) -> Self {
        match s {
            StopReasonSpec::Stop => StopReason::Stop,
            StopReasonSpec::ToolUse => StopReason::ToolUse,
            StopReasonSpec::MaxTokens => StopReason::MaxTokens,
            StopReasonSpec::Aborted => StopReason::Aborted,
        }
    }
}

impl EventSpec {
    #[allow(
        clippy::too_many_lines,
        reason = "keeps the declarative assistant event mapping together"
    )]
    fn to_assistant_event(&self, index: usize) -> Option<AssistantMessageEvent> {
        match self {
            Self::Bare(s) if s == "start" => Some(AssistantMessageEvent::Start {
                partial: AssistantMessage::default(),
            }),
            Self::TextDelta { text_delta } => Some(AssistantMessageEvent::TextDelta {
                index: 0,
                delta: text_delta.clone(),
                partial: AssistantMessage::default(),
            }),
            Self::TextStart { text_start } => Some(AssistantMessageEvent::TextStart {
                index: text_start.index,
                partial: AssistantMessage::default(),
            }),
            Self::TextEnd { text_end } => Some(AssistantMessageEvent::TextEnd {
                index: text_end.index,
                content: text_end.content.clone(),
                partial: AssistantMessage::default(),
            }),
            Self::ThinkingDelta { thinking_delta } => Some(AssistantMessageEvent::ThinkingDelta {
                index: 1,
                delta: thinking_delta.clone(),
                partial: AssistantMessage::default(),
            }),
            Self::ThinkingStart { thinking_start } => Some(AssistantMessageEvent::ThinkingStart {
                index: thinking_start.index,
                partial: AssistantMessage::default(),
            }),
            Self::ThinkingEnd { thinking_end } => Some(AssistantMessageEvent::ThinkingEnd {
                index: thinking_end.index,
                content: thinking_end.content.clone(),
                elapsed_ms: thinking_end.elapsed_ms,
                partial: AssistantMessage::default(),
            }),
            Self::ToolCall { tool_call } => Some(AssistantMessageEvent::ToolCallDelta {
                index,
                delta: serde_json::to_string(&tool_call.args).unwrap_or_default(),
                partial: AssistantMessage::with_tool_call(runie_core::types::ToolCall {
                    id: format!("call-{index}"),
                    name: tool_call.name.clone(),
                    arguments: tool_call.args.clone(),
                    thought_signature: None,
                }),
            }),
            Self::ToolCallStart { tool_call_start } => Some(AssistantMessageEvent::ToolCallStart {
                index: tool_call_start.index,
                partial: AssistantMessage::with_tool_call(runie_core::types::ToolCall {
                    id: tool_call_start.id.clone(),
                    name: tool_call_start.name.clone(),
                    arguments: tool_call_start.arguments.clone(),
                    thought_signature: None,
                }),
            }),
            Self::ToolCallDelta { tool_call_delta } => Some(AssistantMessageEvent::ToolCallDelta {
                index: tool_call_delta.index,
                delta: tool_call_delta.delta.clone(),
                partial: AssistantMessage::with_tool_call(runie_core::types::ToolCall {
                    id: tool_call_delta.id.clone(),
                    name: tool_call_delta.name.clone(),
                    arguments: tool_call_delta.arguments.clone(),
                    thought_signature: None,
                }),
            }),
            Self::ToolCallEnd { tool_call_end } => Some(AssistantMessageEvent::ToolCallEnd {
                index: tool_call_end.index,
                tool_call: runie_core::types::ToolCall {
                    id: tool_call_end.id.clone(),
                    name: tool_call_end.name.clone(),
                    arguments: tool_call_end.arguments.clone(),
                    thought_signature: None,
                },
                partial: AssistantMessage::default(),
            }),
            Self::ToolUpdate { .. } | Self::ToolSeed { .. } => None,
            Self::Done { done } => Some(AssistantMessageEvent::Done {
                stop_reason: StopReason::from(&done.stop_reason),
                usage: done.usage.clone(),
                message: None,
            }),
            Self::Error { error } => Some(AssistantMessageEvent::Error {
                reason: StopReason::Error,
                error: AssistantMessage::with_error(StopReason::Error, error.clone()),
            }),
            Self::Waiting { .. } => None,
            Self::Theme { .. } => None,
            Self::ThinkingLevel { .. } => None,
            Self::ToolMode { .. } => None,
            Self::ToolFold { .. } => None,
            Self::ToolSelect { .. } => None,
            Self::Scroll { .. } => None,
            Self::FollowLatest { .. } => None,
            Self::BackgroundStart { .. }
            | Self::BackgroundProgress { .. }
            | Self::BackgroundEnd { .. }
            | Self::BackgroundCancel { .. }
            | Self::WorkflowStart { .. }
            | Self::WorkflowProgress { .. }
            | Self::WorkflowEnd { .. } => None,
            Self::Bare(other) => panic!("unknown event kind: {other:?}"),
        }
    }

    #[allow(
        clippy::too_many_lines,
        reason = "the declarative control-event projection stays in one table"
    )]
    fn waiting_event(&self) -> Option<AgentEvent> {
        match self {
            Self::Waiting { waiting } => Some(AgentEvent::Waiting {
                reason: waiting_name(waiting),
            }),
            Self::Theme { theme } => Some(AgentEvent::ThemeChanged {
                theme: parse_theme(theme),
            }),
            Self::ThinkingLevel { thinking_level } => Some(AgentEvent::ThinkingLevelChanged {
                level: parse_thinking_level(thinking_level),
            }),
            Self::ToolMode { tool_mode } => Some(AgentEvent::ToolDisplayModeChanged {
                tool_call_id: tool_mode.tool_call_id.clone(),
                mode: tool_mode.mode,
            }),
            Self::ToolUpdate { tool_update } => Some(AgentEvent::ToolExecutionUpdate {
                tool_call_id: tool_update.tool_call_id.clone(),
                tool_name: tool_update.tool_name.clone(),
                args: tool_update.args.clone(),
                partial_result: tool_update.partial_result.clone(),
            }),
            Self::ThinkingStart { .. } | Self::ThinkingEnd { .. } => None,
            Self::ToolFold { .. } => None,
            Self::ToolSelect { .. } => None,
            Self::BackgroundStart { background_start } => Some(AgentEvent::BackgroundWorkStarted {
                work_id: background_start.work_id.clone(),
                description: background_start.description.clone(),
                background: background_start.background,
            }),
            Self::BackgroundProgress {
                background_progress,
            } => Some(AgentEvent::BackgroundWorkProgress {
                work_id: background_progress.work_id.clone(),
                description: background_progress.description.clone(),
                activity: background_progress.activity.clone(),
            }),
            Self::BackgroundEnd { background_end } => Some(AgentEvent::BackgroundWorkFinished {
                work_id: background_end.work_id.clone(),
                description: background_end.description.clone(),
                is_error: background_end.is_error,
                elapsed_ms: background_end.elapsed_ms,
                error: background_end.error.clone(),
            }),
            Self::BackgroundCancel { background_cancel } => {
                Some(AgentEvent::BackgroundWorkCancelled {
                    work_id: background_cancel.work_id.clone(),
                    description: background_cancel.description.clone(),
                    elapsed_ms: background_cancel.elapsed_ms,
                })
            }
            Self::WorkflowStart { workflow_start } => Some(AgentEvent::WorkflowStarted {
                run_id: workflow_start.run_id.clone(),
                name: workflow_start.name.clone(),
                objective: workflow_start.objective.clone(),
            }),
            Self::WorkflowProgress { workflow_progress } => Some(AgentEvent::WorkflowProgress {
                run_id: workflow_progress.run_id.clone(),
                phase: workflow_progress.phase.clone(),
                state: workflow_progress.state.clone(),
                active_agents: workflow_progress.active_agents,
            }),
            Self::WorkflowEnd { workflow_end } => Some(AgentEvent::WorkflowFinished {
                run_id: workflow_end.run_id.clone(),
                status: workflow_end.status.clone(),
                elapsed_ms: workflow_end.elapsed_ms,
            }),
            _ => None,
        }
    }
}

fn parse_theme(theme: &str) -> runie_core::types::ThemeKind {
    match theme.to_ascii_lowercase().as_str() {
        "grok_day" | "grok-day" | "day" => runie_core::types::ThemeKind::GrokDay,
        "tokyo_night" | "tokyo-night" => runie_core::types::ThemeKind::TokyoNight,
        "rose_pine_moon" | "rose-pine-moon" => runie_core::types::ThemeKind::RosePineMoon,
        "oscura_midnight" | "oscura-midnight" => runie_core::types::ThemeKind::OscuraMidnight,
        "auto" => runie_core::types::ThemeKind::Auto,
        "terminal_native" | "terminal-native" | "native" => {
            runie_core::types::ThemeKind::TerminalNative
        }
        _ => runie_core::types::ThemeKind::GrokNight,
    }
}

fn parse_thinking_level(level: &str) -> ThinkingLevel {
    match level.to_ascii_lowercase().as_str() {
        "minimal" => ThinkingLevel::Minimal,
        "low" => ThinkingLevel::Low,
        "medium" => ThinkingLevel::Medium,
        "high" => ThinkingLevel::High,
        "xhigh" | "x-high" => ThinkingLevel::XHigh,
        "max" => ThinkingLevel::Max,
        _ => ThinkingLevel::Off,
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct Assertions {
    #[serde(default)]
    pub transcript_contains: Vec<String>,
    #[serde(default)]
    pub events: Vec<String>,
    /// When present, require the complete event vector in order. This keeps
    /// event-sequence/state tests declarative and recompilation-free.
    #[serde(default)]
    pub exact_events: Option<Vec<String>>,
    #[serde(default)]
    pub turn_starts: Option<usize>,
    #[serde(default)]
    pub scrollback_lines: Vec<LineAssertion>,
    /// Optional in-process visual check: drive the TUI App via TestBackend
    /// at the given viewport size, then assert substrings appear in / are
    /// excluded from the rendered screen.
    #[serde(default)]
    pub visual: Option<VisualAssertions>,
    #[serde(default)]
    pub state: Option<StateAssertions>,
}

/// Small assertion DSL used by the YAML projection oracle. It keeps the
/// fixture-facing field checks declarative while preserving one consistent
/// diagnostic shape; it has no state or rendering side effects.
macro_rules! assert_yaml_eq {
    ($expected:expr, $actual:expr, $field:literal) => {
        if let Some(expected) = $expected {
            let actual = $actual;
            if actual != expected {
                return Err(format!(
                    "state {} mismatch: expected {:?}, got {:?}",
                    $field, expected, actual
                ));
            }
        }
    };
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct StateAssertions {
    /// Renderer-independent status label (for example `thinking` or `ready`).
    pub status: Option<String>,
    pub is_streaming: Option<bool>,
    pub pending_tool_calls: Option<usize>,
    pub messages: Option<usize>,
    pub tool_count: Option<usize>,
    pub streaming_contains: Option<String>,
    pub error_contains: Option<String>,
    pub system_prompt_contains: Option<String>,
    pub tool_blocks: Option<usize>,
    pub tool_output_lines: Option<usize>,
    pub tool_modes: Option<Vec<runie_core::types::ToolDisplayMode>>,
    /// Ordered running-state projection for typed tool cards.
    pub tool_running: Option<Vec<bool>>,
    /// Ordered semantic headers for the projected Grok tool blocks.
    pub tool_headers: Option<Vec<String>>,
    /// Ordered reducer identities for semantic tool-header lines. `null`
    /// denotes a compatibility-seeded row; numeric values are opaque live
    /// reducer tokens.
    pub tool_header_row_ids: Option<Vec<Option<u64>>>,
    /// Ordered lifecycle eligibility for the same semantic header rows. This
    /// distinguishes a retained opaque identity from an active event target.
    pub tool_header_row_active: Option<Vec<bool>>,
    /// Ordered output rows for each projected tool block.
    pub tool_outputs: Option<Vec<Vec<String>>>,
    pub tool_kinds: Option<Vec<crate::widgets::ToolCardKind>>,
    pub selected_tool_id: Option<String>,
    pub selected_entry: Option<usize>,
    pub scroll_offset: Option<usize>,
    pub thinking_level: Option<ThinkingLevel>,
    pub reasoning_expanded: Option<bool>,
    pub activity_expanded: Option<bool>,
    pub follow_latest_user: Option<bool>,
    /// Exact actor-owned workflow projections keyed by their stable run id.
    /// YAML owns the expected state; the runner only performs generic field
    /// comparison so workflow fixtures stay recompilation-free.
    pub workflows: Option<BTreeMap<String, WorkflowStateAssertion>>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct WorkflowStateAssertion {
    pub name: Option<String>,
    pub objective: Option<String>,
    pub phase: Option<String>,
    pub state: Option<String>,
    pub active_agents: Option<u32>,
    pub status: Option<String>,
    pub elapsed_ms: Option<u64>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct VisualAssertions {
    #[serde(default = "default_visual_cols")]
    pub cols: u16,
    #[serde(default = "default_visual_rows")]
    pub rows: u16,
    #[serde(default)]
    pub screen_text: Vec<String>,
    #[serde(default)]
    pub screen_excludes: Vec<String>,
    /// Optional actor-owned command-palette state after `steps` settle.
    #[serde(default)]
    pub palette_open: Option<bool>,
    #[serde(default)]
    pub palette_query: Option<String>,
    #[serde(default)]
    pub palette_index: Option<usize>,
    /// Steps the TUI should perform before snapshotting the screen.
    /// Each step is a key event (e.g. "hello", "Enter", "Ctrl+C").
    #[serde(default)]
    pub steps: Vec<String>,
    /// Interaction steps applied after the declared event stream and initial
    /// prompt have settled. This is the deterministic phase for testing
    /// viewport behavior on real feed content.
    #[serde(default)]
    pub post_steps: Vec<String>,
    /// If true, also spawn the real `runie` binary in a pty and assert
    /// the same `screen_text` / `screen_excludes` substrings there.
    /// Requires `portable-pty` (currently a no-op stub).
    #[serde(default)]
    pub pty: bool,
    /// Render captured reasoning bodies instead of Grok's collapsed `Thought`
    /// summary. This keeps reasoning-fold scenarios declarative.
    #[serde(default)]
    pub reasoning_expanded: bool,
    /// Render grouped tool member rows instead of only the activity summary.
    #[serde(default)]
    pub activity_expanded: Option<bool>,
    #[serde(default)]
    pub header_meter: Option<String>,
    #[serde(default)]
    pub waiting_chrome: Option<String>,
    /// Expected adapter geometry for a complete terminal frame. This keeps
    /// wrapping/scrolling decisions inspectable without encoding Ratatui
    /// objects in YAML.
    #[serde(default)]
    pub layout: Option<LayoutAssertions>,
    /// Additional frozen geometries for the same event/state scenario.
    #[serde(default)]
    pub layout_matrix: Vec<LayoutMatrixCase>,
    /// Optional asciinema oracle. The runner selects the first terminal frame
    /// containing every marker and compares the requested row text with the
    /// Runie TestBackend frame. YAML owns the state/marker recipe; Rust only
    /// supplies the generic dump decoder.
    #[serde(default)]
    pub reference: Option<DumpReference>,
}

#[derive(Debug, Deserialize, Default, Clone, Copy)]
pub struct LayoutAssertions {
    pub header: RegionGeometry,
    pub scrollback: RegionGeometry,
    pub prompt: RegionGeometry,
    pub status: RegionGeometry,
    pub footer_badge: RegionGeometry,
}

#[derive(Debug, Deserialize, Default, Clone, Copy)]
pub struct RegionGeometry {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

#[derive(Debug, Deserialize, Default, Clone, Copy)]
pub struct LayoutMatrixCase {
    pub cols: u16,
    pub rows: u16,
    pub layout: LayoutAssertions,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DumpReference {
    pub cast: String,
    #[serde(default)]
    pub frame_contains: Vec<String>,
    /// Select the first frame after a marker phase has appeared. This is the
    /// YAML equivalent of the cast comparator's `--frames-after` mode.
    #[serde(default)]
    pub frame_after: Vec<String>,
    #[serde(default)]
    pub rows: Vec<DumpRowReference>,
    /// Compare every terminal cell in the selected frame, not only named rows.
    #[serde(default)]
    pub exact_screen: bool,
    /// Compare symbols, colors, and text attributes for every selected cell.
    #[serde(default)]
    pub exact_attributes: bool,
    /// Require the selected cast frame to contain truecolor cells. This keeps
    /// a terminal-default recording from being mistaken for a full-color
    /// parity oracle.
    #[serde(default)]
    pub require_truecolor: bool,
    /// Optional zero-based output-frame index. When present, this takes
    /// precedence over marker matching and makes dynamic casts phase-locked.
    #[serde(default)]
    pub frame_index: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DumpCell {
    symbol: String,
    width: u8,
    fg: String,
    bg: String,
    bold: bool,
    italic: bool,
    underline: bool,
    inverse: bool,
}

fn vt_color_key(color: vt100::Color) -> String {
    match color {
        vt100::Color::Default => "default".to_owned(),
        vt100::Color::Idx(index) => format!("idx:{index}"),
        vt100::Color::Rgb(red, green, blue) => format!("rgb:{red},{green},{blue}"),
    }
}

fn ratatui_color_key(color: ratatui::style::Color) -> String {
    use ratatui::style::Color;
    match color {
        Color::Reset => "default".to_owned(),
        Color::Black => "idx:0".to_owned(),
        Color::Red => "idx:1".to_owned(),
        Color::Green => "idx:2".to_owned(),
        Color::Yellow => "idx:3".to_owned(),
        Color::Blue => "idx:4".to_owned(),
        Color::Magenta => "idx:5".to_owned(),
        Color::Cyan => "idx:6".to_owned(),
        Color::Gray => "idx:7".to_owned(),
        Color::DarkGray => "idx:8".to_owned(),
        Color::LightRed => "idx:9".to_owned(),
        Color::LightGreen => "idx:10".to_owned(),
        Color::LightYellow => "idx:11".to_owned(),
        Color::LightBlue => "idx:12".to_owned(),
        Color::LightMagenta => "idx:13".to_owned(),
        Color::LightCyan => "idx:14".to_owned(),
        Color::White => "idx:15".to_owned(),
        Color::Indexed(index) => format!("idx:{index}"),
        Color::Rgb(red, green, blue) => format!("rgb:{red},{green},{blue}"),
    }
}

fn ratatui_cell_width(buffer: &Buffer, col: u16, row: u16) -> u8 {
    let cell = buffer.cell((col, row)).expect("Runie cell");
    let symbol = cell.symbol();
    if !symbol.is_empty() {
        return UnicodeWidthStr::width(symbol).min(2) as u8;
    }
    if col > 0
        && buffer
            .cell((col - 1, row))
            .is_some_and(|previous| UnicodeWidthStr::width(previous.symbol()) == 2)
    {
        0
    } else {
        1
    }
}

fn cell_symbol_key(symbol: &str) -> String {
    if symbol.is_empty() {
        " ".to_owned()
    } else {
        symbol.to_owned()
    }
}

fn dump_cells(screen: &vt100::Screen, cols: u16, rows: u16) -> Vec<DumpCell> {
    (0..rows)
        .flat_map(|row| {
            (0..cols).map(move |col| {
                let cell = screen.cell(row, col).expect("terminal cell");
                DumpCell {
                    symbol: cell_symbol_key(&cell.contents()),
                    width: if cell.is_wide_continuation() {
                        0
                    } else if cell.is_wide() {
                        2
                    } else {
                        1
                    },
                    fg: vt_color_key(cell.fgcolor()),
                    bg: vt_color_key(cell.bgcolor()),
                    bold: cell.bold(),
                    italic: cell.italic(),
                    underline: cell.underline(),
                    inverse: cell.inverse(),
                }
            })
        })
        .collect()
}

#[derive(Debug, Deserialize, Clone)]
pub struct DumpRowReference {
    pub contains: String,
    #[serde(default)]
    pub exact: bool,
    #[serde(default)]
    pub last: bool,
}

fn default_visual_cols() -> u16 {
    120
}
fn default_visual_rows() -> u16 {
    30
}

#[derive(Debug, Deserialize)]
pub struct LineAssertion {
    pub kind: LineKindName,
    pub contains: String,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum LineKindName {
    User,
    Assistant,
    Tool,
    ToolResult,
    ToolError,
    ToolOutput,
    System,
    Activity,
    Reasoning,
    ThinkingStatus,
}

impl From<LineKindName> for LineKind {
    fn from(k: LineKindName) -> Self {
        match k {
            LineKindName::User => LineKind::User,
            LineKindName::Assistant => LineKind::Assistant,
            LineKindName::Tool => LineKind::Tool,
            LineKindName::ToolResult => LineKind::ToolResult,
            LineKindName::ToolError => LineKind::ToolError,
            LineKindName::ToolOutput => LineKind::ToolOutput,
            LineKindName::System => LineKind::System,
            LineKindName::Activity => LineKind::Activity,
            LineKindName::Reasoning => LineKind::Reasoning,
            LineKindName::ThinkingStatus => LineKind::ThinkingStatus,
        }
    }
}

/// StreamFn impl driven by a `Vec<AssistantMessageEvent>`.
pub struct ScenarioStream {
    pub events: Vec<AssistantMessageEvent>,
    /// Number of `stream()` calls so far. The first call replays `events`;
    /// later calls (auto-continue after a tool batch) return a terminating
    /// `Done{stop}` so the loop does not replay the same script forever.
    pub calls: Mutex<usize>,
    pub pending_after_first: bool,
}

#[async_trait::async_trait]
impl StreamFn for ScenarioStream {
    async fn stream(
        &self,
        _model: &Model,
        _context: &AgentContext,
        _options: Option<SimpleStreamOptions>,
    ) -> Result<AssistantMessageEventStream, StreamError> {
        use futures::stream;
        let mut n = self.calls.lock();
        *n += 1;
        if *n > 1 && self.pending_after_first {
            return Ok(Box::pin(futures::stream::pending()));
        }
        if *n > 1 {
            return Ok(Box::pin(stream::iter(vec![AssistantMessageEvent::Done {
                stop_reason: StopReason::Stop,
                usage: Usage::default(),
                message: None,
            }])));
        }
        // YAML replay consumes the complete event log after the actor settles;
        // a synchronous deterministic stream avoids test-only scheduler races
        // between parallel tool dispatch and recorder completion.
        Ok(Box::pin(stream::iter(self.events.clone())))
    }
}

/// Echo tool that returns its args verbatim.
pub struct EchoTool;
#[async_trait::async_trait]
impl AgentTool for EchoTool {
    fn name(&self) -> &str {
        "echo"
    }
    fn label(&self) -> &str {
        "Echo"
    }
    fn description(&self) -> &str {
        "Echoes args."
    }
    async fn execute(
        &self,
        _id: &str,
        args: serde_json::Value,
        _signal: Option<tokio_util::sync::CancellationToken>,
        _on_update: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
    ) -> Result<AgentToolResult, String> {
        Ok(AgentToolResult {
            content: vec![ToolResultContent::Text {
                text: args.to_string(),
            }],
            details: serde_json::Value::Null,
            usage: None,
            added_tool_names: vec![],
            terminate: false,
        })
    }
}

/// Deterministic named tool used by strict visual replays. Its output is
/// fixed so the TestBackend frame never depends on the host filesystem.
pub struct ReplayTool {
    name: String,
    output: String,
    error: bool,
}

impl ReplayTool {
    fn new(name: &str, output: &str) -> Self {
        Self {
            name: name.into(),
            output: output.into(),
            error: false,
        }
    }

    fn failing(name: &str, output: &str) -> Self {
        Self {
            name: name.into(),
            output: output.into(),
            error: true,
        }
    }

    fn structured(name: &str, output: &str) -> Self {
        Self {
            name: name.into(),
            output: output.into(),
            error: false,
        }
    }
}

#[async_trait::async_trait]
impl AgentTool for ReplayTool {
    fn name(&self) -> &str {
        &self.name
    }
    fn label(&self) -> &str {
        &self.name
    }
    fn description(&self) -> &str {
        "Deterministic visual replay tool."
    }
    async fn execute(
        &self,
        _id: &str,
        _args: serde_json::Value,
        _signal: Option<tokio_util::sync::CancellationToken>,
        on_update: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
    ) -> Result<AgentToolResult, String> {
        if self.error {
            return Err(self.output.clone());
        }
        if let Some(on_update) = on_update {
            on_update(serde_json::json!({"output": self.output}));
        }
        Ok(AgentToolResult {
            content: vec![ToolResultContent::Text {
                text: self.output.clone(),
            }],
            details: serde_json::Value::Null,
            usage: None,
            added_tool_names: vec![],
            terminate: false,
        })
    }
}

#[derive(Clone)]
pub struct ScenarioOutcome {
    pub events: Vec<AgentEvent>,
    pub feed: FeedSnapshot,
    pub scrollback: Vec<Line>,
    pub tool_blocks: Vec<ToolBlock>,
    pub selected_tool_id: Option<String>,
    pub selected_entry: Option<usize>,
    pub scroll_offset: usize,
    pub state: runie_core::state::AgentStateSnapshot,
    pub status: crate::widgets::StatusSnapshot,
}

pub struct ScenarioError(pub String);

impl std::fmt::Display for ScenarioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

pub async fn run_scenario(scenario: &Scenario) -> Result<ScenarioOutcome, ScenarioError> {
    let (bus, actor) = build_scenario_loop(scenario)?;

    let actor_snapshot = actor.clone();
    let mut events_from_task = record_and_run_scenario(actor, bus, scenario).await;
    let declared_events = declared_control_events(scenario);
    events_from_task.extend(declared_events.iter().cloned());
    for event in &declared_events {
        actor_snapshot.apply_event(event).await;
    }

    let (scrollback, status) = replay_scenario_events(
        &events_from_task,
        scenario.initial_prompt.is_none(),
        scenario,
    )
    .await;
    let feed = scrollback.model_snapshot();
    Ok(ScenarioOutcome {
        events: events_from_task,
        scrollback: feed.lines.clone(),
        tool_blocks: feed.tool_blocks.clone(),
        selected_tool_id: feed.selected_tool_id.clone(),
        selected_entry: feed.selected_entry,
        scroll_offset: feed.scroll_offset,
        feed,
        state: actor_snapshot.state_snapshot(),
        status,
    })
}

fn declared_control_events(scenario: &Scenario) -> Vec<AgentEvent> {
    scenario
        .events
        .iter()
        .filter_map(EventSpec::waiting_event)
        .collect()
}

#[allow(
    clippy::cognitive_complexity,
    reason = "the YAML replay keeps seed, event, and navigation reduction in one deterministic pass"
)]
async fn replay_scenario_events(
    events: &[AgentEvent],
    emit_welcome: bool,
    scenario: &Scenario,
) -> (Scrollback, crate::widgets::StatusSnapshot) {
    let scrollback_actor = crate::ScrollbackActor::new();
    let status_actor = crate::StatusActor::new();
    let mut renderer =
        EventRenderer::with_actors(scrollback_actor.clone(), status_actor.clone(), emit_welcome);
    for message in declared_tool_seeds(scenario) {
        scrollback_actor.apply(message).await;
    }
    for event in events {
        renderer.apply_actor_event(event.clone()).await;
    }
    for tool_call_id in declared_tool_folds(scenario) {
        scrollback_actor
            .apply(ScrollbackMsg::ToggleToolMode(tool_call_id))
            .await;
    }
    for message in declared_tool_selections(scenario) {
        scrollback_actor.apply(message).await;
    }
    for message in declared_scrolls(scenario) {
        scrollback_actor.apply(message).await;
    }
    (scrollback_actor.snapshot(), status_actor.model_snapshot())
}

fn declared_tool_folds(scenario: &Scenario) -> Vec<String> {
    scenario
        .events
        .iter()
        .filter_map(|event| match event {
            EventSpec::ToolFold { tool_fold } => Some(tool_fold.clone()),
            _ => None,
        })
        .collect()
}

fn declared_tool_seeds(scenario: &Scenario) -> Vec<ScrollbackMsg> {
    scenario
        .events
        .iter()
        .filter_map(|event| match event {
            EventSpec::ToolSeed { tool_seed } => Some(ScrollbackMsg::Append(
                Line::new(
                    if tool_seed.running {
                        LineKind::ToolRunning
                    } else {
                        LineKind::Tool
                    },
                    tool_seed.header.clone(),
                )
                .for_tool(tool_seed.tool_call_id.clone()),
            )),
            _ => None,
        })
        .collect()
}

fn declared_tool_selections(scenario: &Scenario) -> Vec<ScrollbackMsg> {
    scenario
        .events
        .iter()
        .filter_map(|event| match event {
            EventSpec::ToolSelect { tool_select } if tool_select == "next" => {
                Some(ScrollbackMsg::SelectNextTool)
            }
            EventSpec::ToolSelect { tool_select } if tool_select == "previous" => {
                Some(ScrollbackMsg::SelectPreviousTool)
            }
            EventSpec::ToolSelect { tool_select } if tool_select == "entry_next" => {
                Some(ScrollbackMsg::SelectNextEntry)
            }
            EventSpec::ToolSelect { tool_select } if tool_select == "entry_previous" => {
                Some(ScrollbackMsg::SelectPreviousEntry)
            }
            _ => None,
        })
        .collect()
}

fn declared_scrolls(scenario: &Scenario) -> Vec<ScrollbackMsg> {
    scenario
        .events
        .iter()
        .filter_map(|event| match event {
            EventSpec::Scroll { scroll } => Some(ScrollbackMsg::ScrollBy(*scroll)),
            EventSpec::FollowLatest { follow_latest } => {
                Some(ScrollbackMsg::SetFollowLatestUser(*follow_latest))
            }
            _ => None,
        })
        .collect()
}

async fn record_and_run_scenario(
    actor: LoopActor,
    bus: EventBus,
    scenario: &Scenario,
) -> Vec<AgentEvent> {
    let mut rec_rx = bus.subscribe();
    let (rec_stop_tx, mut rec_stop_rx) = tokio::sync::oneshot::channel::<()>();
    // OWNER: YAML replay recorder; joined before the scenario returns.
    let rec_handle = tokio::spawn(async move {
        let mut captured = Vec::new();
        loop {
            tokio::select! {
                biased;
                _ = &mut rec_stop_rx => break,
                result = rec_rx.recv() => {
                    match result {
                        Ok(ev) => {
                            let finished = matches!(ev, AgentEvent::AgentEnd { .. });
                            captured.push(ev);
                            if finished {
                                break;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
            }
        }
        captured
    });

    submit_scenario(actor.clone(), scenario).await;
    actor.wait_for_idle().await;

    // Signal the recorder to stop (the bus is still held by LoopActor,
    // so it doesn't close — we use a dedicated oneshot to break the loop).
    let _ = rec_stop_tx.send(());
    let events_from_task = rec_handle.await.unwrap_or_default();

    events_from_task
}

async fn submit_scenario(actor: LoopActor, scenario: &Scenario) {
    for text in &scenario.follow_up {
        actor
            .follow_up(AgentMessage::User(UserMessage {
                content: vec![UserContent::Text { text: text.clone() }],
                timestamp: 0,
            }))
            .await;
    }
    let prompts = scenario
        .initial_prompt
        .as_ref()
        .map(|text| {
            vec![AgentMessage::User(UserMessage {
                content: vec![UserContent::Text { text: text.clone() }],
                timestamp: 1,
            })]
        })
        .unwrap_or_default();
    if let Err(error) = actor.prompt(prompts, scenario.agent_context()).await {
        eprintln!("[yaml_runner] prompt error: {error:?}");
    }
}

fn build_scenario_loop(scenario: &Scenario) -> Result<(EventBus, LoopActor), ScenarioError> {
    let bus = EventBus::new();
    let mut registry = ToolRegistry::new();
    for tool in &scenario.tools {
        register_scenario_tool(&mut registry, tool)?;
    }
    let provider = ProviderActor::new(Arc::new(ScenarioStream {
        events: scenario
            .events
            .iter()
            .enumerate()
            .filter_map(|(index, event)| event.to_assistant_event(index))
            .collect(),
        calls: Mutex::new(0),
        // `run_scenario` waits for a terminal outcome to assert state; the
        // pending capture mode is exercised by `render_visual_buffer`, which
        // snapshots before joining the deliberately pending continuation.
        pending_after_first: false,
    }));
    let deps = LoopDeps {
        state: AgentStateActor::new(),
        steering: SteeringQueueActor::new(),
        follow_up: FollowUpQueueActor::new(),
        tool_executor: ToolExecutorActor::new(Arc::new(registry)),
        provider,
        bus: bus.clone(),
        subscribers: runie_core::events::SubscriberRegistry::new(),
        hooks: ToolExecHooks::default(),
        turn_hooks: runie_core::hooks::TurnHooks::default(),
        transform_context: None,
        api_key_resolver: None,
        convert_to_llm: None,
        stream_options: Default::default(),
        abort: None,
        tool_execution_mode: ToolExecutionMode::Parallel,
        steering_mode: scenario.steering_mode.unwrap_or_default(),
        follow_up_mode: scenario.follow_up_mode.unwrap_or_default(),
    };
    Ok((bus, LoopActor::new(deps)))
}

#[allow(
    clippy::too_many_lines,
    reason = "the YAML tool registry keeps declarative replay variants together"
)]
fn register_scenario_tool(
    registry: &mut ToolRegistry,
    tool: &ToolSpec,
) -> Result<(), ScenarioError> {
    match tool.kind.as_str() {
        "echo" => registry.register(Arc::new(EchoTool)),
        "list_dir" => registry.register(Arc::new(ReplayTool::new(
            &tool.name,
            "Cargo.toml\nsrc\ncrates",
        ))),
        "read" => registry.register(Arc::new(ReplayTool::new(
            &tool.name,
            "# runie\n\nThis is **Runie**.",
        ))),
        "edit" => registry.register(Arc::new(ReplayTool::new(
            &tool.name,
            "@@ -1 +1 @@\n-old\n+new",
        ))),
        "bash" => registry.register(Arc::new(ReplayTool::new(
            &tool.name,
            "cargo test completed",
        ))),
        "subagent" => {
            registry.register(Arc::new(ReplayTool::new(&tool.name, "subagent completed")))
        }
        "memory_search" => registry.register(Arc::new(ReplayTool::new(
            &tool.name,
            "memory hit one\nmemory hit two",
        ))),
        "workflow" => registry.register(Arc::new(ReplayTool::new(&tool.name, "workflow done"))),
        "web_fetch" => registry.register(Arc::new(ReplayTool::new(
            &tool.name,
            "status: 200\ncontent_type: text/html\nsize: 14.2 KB\nbody",
        ))),
        "web_search" => registry.register(Arc::new(ReplayTool::new(
            &tool.name,
            "https://docs.rs/runie\nhttps://docs.rs/ratatui\nhttps://rust-lang.org/learn",
        ))),
        "error" => registry.register(Arc::new(ReplayTool::failing(&tool.name, "tool failed"))),
        "structured_update" => registry.register(Arc::new(ReplayTool::structured(
            &tool.name,
            "first\nsecond",
        ))),
        other => return Err(ScenarioError(format!("unknown tool kind: {other}"))),
    }
    Ok(())
}

pub fn assert_scenario(outcome: &ScenarioOutcome, scenario: &Scenario) -> Result<(), String> {
    // Reuse the current tokio runtime if any (the e2e binary runs each
    // scenario inside `#[tokio::main]`); otherwise spin up a fresh one.
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => handle.block_on(assert_scenario_async(outcome, scenario)),
        Err(_) => {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| format!("build tokio runtime: {e}"))?;
            rt.block_on(assert_scenario_async(outcome, scenario))
        }
    }
}

pub async fn assert_scenario_async(
    outcome: &ScenarioOutcome,
    scenario: &Scenario,
) -> Result<(), String> {
    assert_event_expectations(outcome, scenario)?;
    assert_state_expectations(outcome, scenario)?;
    assert_transcript_expectations(outcome, scenario)?;
    if let Some(visual) = &scenario.assertions.visual {
        assert_visual_expectations(scenario, visual).await?;
    }
    Ok(())
}

#[allow(
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    reason = "state assertion diagnostics stay grouped by projection field"
)]
fn assert_state_expectations(outcome: &ScenarioOutcome, scenario: &Scenario) -> Result<(), String> {
    let Some(expected) = &scenario.assertions.state else {
        return Ok(());
    };
    let actual = &outcome.state;
    if let Some(expected) = &expected.status {
        let actual_status = outcome.status.state.label();
        if actual_status != *expected {
            return Err(format!(
                "state status mismatch: expected {expected:?}, got {actual_status:?}"
            ));
        }
    }
    assert_yaml_eq!(expected.is_streaming, actual.is_streaming, "is_streaming");
    assert_yaml_eq!(
        expected.thinking_level,
        actual.thinking_level,
        "thinking_level"
    );
    assert_yaml_eq!(
        expected.pending_tool_calls,
        actual.pending_tool_calls.len(),
        "pending_tool_calls"
    );
    assert_yaml_eq!(expected.messages, actual.messages.len(), "messages");
    assert_yaml_eq!(expected.tool_count, actual.tools.len(), "tool_count");
    if let Some(needle) = &expected.streaming_contains {
        let text = actual
            .streaming_message
            .as_ref()
            .map(message_text)
            .unwrap_or_default();
        if !text.contains(needle) {
            return Err(format!("streaming state missing {needle:?}: {text:?}"));
        }
    }
    if let Some(needle) = &expected.error_contains {
        let error = actual.error_message.as_deref().unwrap_or_default();
        if !error.contains(needle) {
            return Err(format!("state error missing {needle:?}: {error:?}"));
        }
    }
    if let Some(needle) = &expected.system_prompt_contains {
        if !actual.system_prompt.contains(needle) {
            return Err(format!(
                "state system prompt missing {needle:?}: {:?}",
                actual.system_prompt
            ));
        }
    }
    assert_tool_block_expectations(outcome, expected)?;
    if let Some(expected) = &expected.selected_tool_id {
        if outcome.feed.selected_tool_id.as_deref() != Some(expected.as_str()) {
            return Err(format!(
                "state selected_tool_id mismatch: expected {expected:?}, got {:?}",
                outcome.feed.selected_tool_id
            ));
        }
    }
    if let Some(expected) = expected.selected_entry {
        if outcome.feed.selected_entry != Some(expected) {
            return Err(format!(
                "state selected_entry mismatch: expected {expected:?}, got {:?}",
                outcome.feed.selected_entry
            ));
        }
    }
    if let Some(expected) = expected.scroll_offset {
        if outcome.feed.scroll_offset != expected {
            return Err(format!(
                "state scroll_offset mismatch: expected {expected}, got {}",
                outcome.feed.scroll_offset
            ));
        }
    }
    assert_feed_state_expectations(outcome, expected)?;
    assert_workflow_expectations(outcome, expected)?;
    Ok(())
}

fn assert_feed_state_expectations(
    outcome: &ScenarioOutcome,
    expected: &StateAssertions,
) -> Result<(), String> {
    if let Some(expected) = expected.reasoning_expanded {
        if outcome.feed.reasoning_expanded != expected {
            return Err(format!(
                "state reasoning_expanded mismatch: expected {expected}, got {}",
                outcome.feed.reasoning_expanded
            ));
        }
    }
    if let Some(expected) = expected.activity_expanded {
        if outcome.feed.activity_expanded != expected {
            return Err(format!(
                "state activity_expanded mismatch: expected {expected}, got {}",
                outcome.feed.activity_expanded
            ));
        }
    }
    if let Some(expected) = expected.follow_latest_user {
        if outcome.feed.follow_latest_user != expected {
            return Err(format!(
                "state follow_latest_user mismatch: expected {expected}, got {}",
                outcome.feed.follow_latest_user
            ));
        }
    }
    Ok(())
}

fn assert_workflow_expectations(
    outcome: &ScenarioOutcome,
    expected: &StateAssertions,
) -> Result<(), String> {
    let Some(expected) = &expected.workflows else {
        return Ok(());
    };
    if outcome.state.workflows.len() != expected.len() {
        return Err(format!(
            "state workflows mismatch: expected keys {:?}, got {:?}",
            expected.keys().collect::<Vec<_>>(),
            outcome.state.workflows.keys().collect::<Vec<_>>()
        ));
    }
    for (run_id, assertion) in expected {
        let Some(actual) = outcome.state.workflows.get(run_id) else {
            return Err(format!("state workflow missing run_id {run_id:?}"));
        };
        assert_workflow_fields(run_id, assertion, actual)?;
    }
    Ok(())
}

fn assert_workflow_fields(
    run_id: &str,
    assertion: &WorkflowStateAssertion,
    actual: &runie_core::state::WorkflowSnapshot,
) -> Result<(), String> {
    assert_optional_eq(&assertion.name, &actual.name, "workflow name", run_id)?;
    assert_optional_eq(
        &assertion.objective,
        &actual.objective,
        "workflow objective",
        run_id,
    )?;
    assert_optional_option_eq(&assertion.phase, &actual.phase, "workflow phase", run_id)?;
    assert_optional_option_eq(&assertion.state, &actual.state, "workflow state", run_id)?;
    if let Some(expected) = assertion.active_agents {
        if actual.active_agents != expected {
            return Err(format!(
                "state workflow {run_id:?} active_agents mismatch: expected {expected}, got {}",
                actual.active_agents
            ));
        }
    }
    assert_optional_eq(&assertion.status, &actual.status, "workflow status", run_id)?;
    if let Some(expected) = assertion.elapsed_ms {
        if actual.elapsed_ms != Some(expected) {
            return Err(format!(
                "state workflow {run_id:?} elapsed_ms mismatch: expected {expected:?}, got {:?}",
                actual.elapsed_ms
            ));
        }
    }
    Ok(())
}

fn assert_optional_eq<T: PartialEq + std::fmt::Debug>(
    expected: &Option<T>,
    actual: &T,
    field: &str,
    run_id: &str,
) -> Result<(), String> {
    if let Some(expected) = expected {
        if expected != actual {
            return Err(format!(
                "state workflow {run_id:?} {field} mismatch: expected {expected:?}, got {actual:?}"
            ));
        }
    }
    Ok(())
}

fn assert_optional_option_eq<T: PartialEq + std::fmt::Debug>(
    expected: &Option<T>,
    actual: &Option<T>,
    field: &str,
    run_id: &str,
) -> Result<(), String> {
    if let Some(expected) = expected {
        if actual.as_ref() != Some(expected) {
            return Err(format!(
                "state workflow {run_id:?} {field} mismatch: expected {expected:?}, got {actual:?}"
            ));
        }
    }
    Ok(())
}

#[allow(
    clippy::too_many_lines,
    reason = "the declarative tool-card fields are kept together for fixture diagnostics"
)]
fn assert_tool_block_expectations(
    outcome: &ScenarioOutcome,
    expected: &StateAssertions,
) -> Result<(), String> {
    if let Some(value) = expected.tool_blocks {
        assert_equal(value, actual_tool_blocks(outcome), "tool_blocks")?;
    }
    if let Some(value) = expected.tool_output_lines {
        let actual = outcome
            .feed
            .tool_blocks
            .iter()
            .map(|block| block.output.len())
            .sum::<usize>();
        assert_equal(value, actual, "tool_output_lines")?;
    }
    if let Some(value) = &expected.tool_modes {
        assert_vec_equal(
            value,
            &outcome
                .feed
                .tool_blocks
                .iter()
                .map(|block| block.mode)
                .collect::<Vec<_>>(),
            "tool_modes",
        )?;
    }
    if let Some(value) = &expected.tool_running {
        assert_vec_equal(
            value,
            &outcome
                .feed
                .tool_blocks
                .iter()
                .map(|block| block.is_running)
                .collect::<Vec<_>>(),
            "tool_running",
        )?;
    }
    if let Some(value) = &expected.tool_headers {
        assert_vec_equal(
            value,
            &outcome
                .feed
                .tool_blocks
                .iter()
                .map(|block| block.header.clone())
                .collect::<Vec<_>>(),
            "tool_headers",
        )?;
    }
    if let Some(value) = &expected.tool_header_row_ids {
        let actual = outcome
            .feed
            .lines
            .iter()
            .filter(|line| {
                matches!(
                    line.kind,
                    LineKind::Tool | LineKind::ToolRunning | LineKind::ToolError
                )
            })
            .map(|line| line.tool_row_id)
            .collect::<Vec<_>>();
        assert_vec_equal(value, &actual, "tool_header_row_ids")?;
    }
    if let Some(value) = &expected.tool_header_row_active {
        let actual = outcome
            .feed
            .lines
            .iter()
            .filter(|line| {
                matches!(
                    line.kind,
                    LineKind::Tool | LineKind::ToolRunning | LineKind::ToolError
                )
            })
            .map(|line| line.is_tool_row_active())
            .collect::<Vec<_>>();
        assert_vec_equal(value, &actual, "tool_header_row_active")?;
    }
    if let Some(value) = &expected.tool_outputs {
        assert_vec_equal(
            value,
            &outcome
                .feed
                .tool_blocks
                .iter()
                .map(|block| block.output.clone())
                .collect::<Vec<_>>(),
            "tool_outputs",
        )?;
    }
    if let Some(value) = &expected.tool_kinds {
        assert_vec_equal(
            value,
            &outcome
                .feed
                .tool_blocks
                .iter()
                .map(|block| block.kind)
                .collect::<Vec<_>>(),
            "tool_kinds",
        )?;
    }
    Ok(())
}

fn assert_equal(expected: usize, actual: usize, field: &str) -> Result<(), String> {
    if expected == actual {
        Ok(())
    } else {
        Err(format!(
            "state {field} mismatch: expected {expected}, got {actual}"
        ))
    }
}

fn assert_vec_equal<T: std::fmt::Debug + PartialEq>(
    expected: &[T],
    actual: &[T],
    field: &str,
) -> Result<(), String> {
    if expected == actual {
        Ok(())
    } else {
        Err(format!(
            "state {field} mismatch: expected {expected:?}, got {actual:?}"
        ))
    }
}

fn actual_tool_blocks(outcome: &ScenarioOutcome) -> usize {
    outcome.feed.tool_blocks.len()
}

fn message_text(message: &runie_core::types::AgentMessage) -> String {
    match message {
        runie_core::types::AgentMessage::Assistant(message) => message
            .content
            .iter()
            .filter_map(|block| match block {
                runie_core::types::AssistantContent::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect(),
        _ => String::new(),
    }
}

fn assert_event_expectations(outcome: &ScenarioOutcome, scenario: &Scenario) -> Result<(), String> {
    let kinds = outcome.events.iter().map(event_kind).collect::<Vec<_>>();
    for expected in &scenario.assertions.events {
        if !kinds.contains(&expected.as_str()) {
            return Err(format!("expected event kind {expected:?} not in {kinds:?}"));
        }
    }
    if let Some(expected) = &scenario.assertions.exact_events {
        if kinds != expected.iter().map(String::as_str).collect::<Vec<_>>() {
            return Err(format!(
                "exact event sequence mismatch: expected {:?}, got {:?}",
                expected, kinds
            ));
        }
    }
    if let Some(expected) = scenario.assertions.turn_starts {
        let actual = kinds.iter().filter(|kind| **kind == "turn_start").count();
        if actual != expected {
            return Err(format!(
                "expected {expected} turn_start events, got {actual}"
            ));
        }
    }
    Ok(())
}

fn event_kind(event: &runie_core::types::AgentEvent) -> &'static str {
    use runie_core::types::AgentEvent::*;
    match event {
        AgentStart => "agent_start",
        AgentEnd { .. } => "agent_end",
        Error { .. } => "error",
        ThinkingLevelChanged { .. } => "thinking_level_changed",
        Reset => "reset",
        TurnStart => "turn_start",
        Waiting { .. } => "waiting",
        ThemeChanged { .. } => "theme_changed",
        ToolDisplayModeChanged { .. } => "tool_display_mode_changed",
        TurnEnd { .. } => "turn_end",
        MessageStart { .. } => "message_start",
        MessageUpdate { .. } => "message_update",
        MessageEnd { .. } => "message_end",
        ToolExecutionStart { .. } => "tool_execution_start",
        ToolExecutionUpdate { .. } => "tool_execution_update",
        ToolExecutionEnd { .. } => "tool_execution_end",
        BackgroundWorkStarted { .. } => "background_work_started",
        BackgroundWorkProgress { .. } => "background_work_progress",
        BackgroundWorkFinished { .. } => "background_work_finished",
        BackgroundWorkCancelled { .. } => "background_work_cancelled",
        WorkflowStarted { .. } => "workflow_started",
        WorkflowProgress { .. } => "workflow_progress",
        WorkflowFinished { .. } => "workflow_finished",
    }
}

fn assert_transcript_expectations(
    outcome: &ScenarioOutcome,
    scenario: &Scenario,
) -> Result<String, String> {
    let haystack = outcome
        .scrollback
        .iter()
        .map(|line| line.text.clone())
        .collect::<Vec<_>>()
        .join("\n");
    for needle in &scenario.assertions.transcript_contains {
        if !haystack.contains(needle) {
            return Err(format!(
                "transcript missing {needle:?}; full haystack:\n{haystack}"
            ));
        }
    }
    for assertion in &scenario.assertions.scrollback_lines {
        let kind: LineKind = assertion.kind.into();
        if !outcome
            .scrollback
            .iter()
            .any(|line| line.kind == kind && line.text.contains(&assertion.contains))
        {
            return Err(format!(
                "expected {kind:?} line containing {:?}; scrollback:\n{haystack}",
                assertion.contains
            ));
        }
    }
    Ok(haystack)
}

async fn assert_visual_expectations(
    scenario: &Scenario,
    visual: &VisualAssertions,
) -> Result<(), String> {
    let buffer = render_visual_buffer(scenario, visual).await?;
    if let Some(expected) = visual.layout {
        assert_layout_expectations(visual.cols, visual.rows, expected)?;
    }
    for case in &visual.layout_matrix {
        let mut matrix_visual = visual.clone();
        matrix_visual.cols = case.cols;
        matrix_visual.rows = case.rows;
        render_visual_buffer(scenario, &matrix_visual).await?;
        assert_layout_expectations(case.cols, case.rows, case.layout)?;
    }
    let screen = buffer_to_screen(&buffer);
    for needle in &visual.screen_text {
        if !screen.contains(needle) {
            return Err(format!("screen missing {needle:?}\nscreen:\n{screen}"));
        }
    }
    for needle in &visual.screen_excludes {
        if screen.contains(needle) {
            return Err(format!(
                "screen unexpectedly contains {needle:?}\nscreen:\n{screen}"
            ));
        }
    }
    if let Some(reference) = &visual.reference {
        assert_dump_reference(&buffer, reference)?;
    }
    if visual.pty {
        eprintln!("[visual] pty assertion requested but not yet implemented");
    }
    Ok(())
}

fn assert_layout_expectations(
    cols: u16,
    rows: u16,
    expected: LayoutAssertions,
) -> Result<(), String> {
    let layout = crate::layout::chat_layout_with_prompt_height(
        ratatui::layout::Rect::new(0, 0, cols, rows),
        crate::layout::PROMPT_HEIGHT,
    );
    let actual = [
        ("header", layout.header, expected.header),
        ("scrollback", layout.scrollback, expected.scrollback),
        ("prompt", layout.prompt, expected.prompt),
        ("status", layout.status, expected.status),
        ("footer_badge", layout.footer_badge, expected.footer_badge),
    ];
    for (name, region, expected) in actual {
        let actual = (region.x, region.y, region.width, region.height);
        let expected = (expected.x, expected.y, expected.width, expected.height);
        if actual != expected {
            return Err(format!(
                "layout {name} mismatch: expected {expected:?}, got {actual:?}"
            ));
        }
    }
    Ok(())
}

#[allow(
    clippy::too_many_lines,
    reason = "the generic dump oracle keeps decode, frame selection, and row diagnostics together"
)]
#[allow(clippy::cognitive_complexity)]
#[allow(clippy::too_many_arguments)]
fn assert_dump_reference(buffer: &Buffer, reference: &DumpReference) -> Result<(), String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../artifacts")
        .join(&reference.cast);
    let dump = std::fs::read_to_string(&path)
        .map_err(|error| format!("read dump {}: {error}", path.display()))?;
    let mut lines = dump.lines();
    let header: serde_json::Value = serde_json::from_str(
        lines
            .next()
            .ok_or_else(|| format!("dump {} has no header", path.display()))?,
    )
    .map_err(|error| format!("parse dump header {}: {error}", path.display()))?;
    let cols = header["term"]["cols"]
        .as_u64()
        .ok_or_else(|| format!("dump {} has no terminal width", path.display()))?
        as u16;
    let rows = header["term"]["rows"]
        .as_u64()
        .ok_or_else(|| format!("dump {} has no terminal height", path.display()))?
        as u16;
    let mut parser = vt100::Parser::new(rows, cols, 0);
    let mut selected = None;
    let mut selected_cells = None;
    let mut selected_frame_index = None;
    let mut output_frame = 0usize;
    let mut after_armed = reference.frame_after.is_empty();
    for line in lines {
        let event: serde_json::Value = serde_json::from_str(line)
            .map_err(|error| format!("parse dump event {}: {error}", path.display()))?;
        if event[1].as_str() != Some("o") {
            continue;
        }
        parser.process(
            event[2]
                .as_str()
                .ok_or_else(|| format!("dump {} has invalid output event", path.display()))?
                .as_bytes(),
        );
        let contents = parser.screen().contents();
        let marker_match = reference
            .frame_contains
            .iter()
            .all(|marker| contents.contains(marker));
        let after_match = reference
            .frame_after
            .iter()
            .all(|marker| contents.contains(marker));
        let frame_selected = match reference.frame_index {
            Some(index) => output_frame == index,
            None => after_armed && marker_match,
        };
        output_frame += 1;
        if after_match {
            after_armed = true;
        }
        if frame_selected {
            selected = Some(contents);
            selected_cells = Some(dump_cells(parser.screen(), cols, rows));
            selected_frame_index = Some(output_frame - 1);
            break;
        }
    }
    let reference_screen = selected.ok_or_else(|| {
        format!(
            "dump {} has no matching frame (index {:?}, markers {:?}, after {:?})",
            path.display(),
            reference.frame_index,
            reference.frame_contains,
            reference.frame_after
        )
    })?;
    if reference.exact_attributes {
        let expected = selected_cells.as_ref().expect("selected frame cells");
        if reference.require_truecolor
            && !expected
                .iter()
                .any(|cell| cell.fg.starts_with("rgb:") || cell.bg.starts_with("rgb:"))
        {
            return Err(
                "reference frame has no RGB cells; capture with COLORTERM=truecolor".to_owned(),
            );
        }
        let expected_width = cols;
        let expected_height = rows;
        if buffer.area.width != expected_width || buffer.area.height != expected_height {
            return Err(format!(
                "full dump dimensions differ: expected {expected_width}x{expected_height}, actual {}x{}",
                buffer.area.width, buffer.area.height
            ));
        }
        let actual = (0..buffer.area.height)
            .flat_map(|row| {
                (0..buffer.area.width).map(move |col| {
                    let cell = buffer.cell((col, row)).expect("Runie cell");
                    DumpCell {
                        symbol: cell_symbol_key(cell.symbol()),
                        width: ratatui_cell_width(buffer, col, row),
                        fg: ratatui_color_key(cell.fg),
                        bg: ratatui_color_key(cell.bg),
                        bold: cell.modifier.contains(ratatui::style::Modifier::BOLD),
                        italic: cell.modifier.contains(ratatui::style::Modifier::ITALIC),
                        underline: cell.modifier.contains(ratatui::style::Modifier::UNDERLINED),
                        inverse: cell.modifier.contains(ratatui::style::Modifier::REVERSED),
                    }
                })
            })
            .collect::<Vec<_>>();
        if expected.as_slice() != actual.as_slice() {
            let width = buffer.area.width as usize;
            let mut details = Vec::new();
            for (index, (left, right)) in expected.iter().zip(&actual).enumerate() {
                if left == right {
                    continue;
                }
                let row = index / width;
                let col = index % width;
                let mut fields = Vec::new();
                if left.symbol != right.symbol {
                    fields.push(format!("symbol {:?} -> {:?}", left.symbol, right.symbol));
                }
                if left.fg != right.fg {
                    fields.push(format!("fg {} -> {}", left.fg, right.fg));
                }
                if left.bg != right.bg {
                    fields.push(format!("bg {} -> {}", left.bg, right.bg));
                }
                if left.bold != right.bold {
                    fields.push(format!("bold {} -> {}", left.bold, right.bold));
                }
                if left.italic != right.italic {
                    fields.push(format!("italic {} -> {}", left.italic, right.italic));
                }
                if left.underline != right.underline {
                    fields.push(format!(
                        "underline {} -> {}",
                        left.underline, right.underline
                    ));
                }
                if left.inverse != right.inverse {
                    fields.push(format!("inverse {} -> {}", left.inverse, right.inverse));
                }
                details.push(format!("({col},{row}): {}", fields.join(", ")));
                if details.len() == 12 {
                    break;
                }
            }
            return Err(format!(
                "full dump cell attribute mismatch ({} differing cells; first differences):\n{}",
                expected
                    .iter()
                    .zip(&actual)
                    .filter(|(left, right)| left != right)
                    .count(),
                details.join("\n")
            ));
        }
    }
    if reference.exact_screen {
        let expected = selected_cells
            .as_ref()
            .expect("selected frame cells for exact symbols");
        let actual = (0..buffer.area.height)
            .flat_map(|row| {
                (0..buffer.area.width).map(move |col| {
                    cell_symbol_key(buffer.cell((col, row)).expect("Runie cell").symbol())
                })
            })
            .collect::<Vec<_>>();
        let first_difference = expected
            .iter()
            .zip(&actual)
            .position(|(left, right)| left.symbol != *right);
        if let Some(index) = first_difference {
            let width = buffer.area.width as usize;
            let row = index / width;
            let col = index % width;
            let expected_row = expected
                .iter()
                .skip(row * width)
                .take(width)
                .map(|cell| cell.symbol.as_str())
                .collect::<String>();
            let actual_row = (0..buffer.area.width)
                .map(|column| {
                    cell_symbol_key(
                        buffer
                            .cell((column, row as u16))
                            .expect("Runie cell")
                            .symbol(),
                    )
                })
                .collect::<String>();
            let expected_context = (row.saturating_sub(2)..=(row + 2).min(rows as usize - 1))
                .map(|context_row| {
                    expected
                        .iter()
                        .skip(context_row * width)
                        .take(width)
                        .map(|cell| cell.symbol.as_str())
                        .collect::<String>()
                })
                .collect::<Vec<_>>();
            let actual_context = (row.saturating_sub(2)..=(row + 2).min(rows as usize - 1))
                .map(|context_row| {
                    (0..buffer.area.width)
                        .map(|column| {
                            cell_symbol_key(
                                buffer
                                    .cell((column, context_row as u16))
                                    .expect("Runie cell")
                                    .symbol(),
                            )
                        })
                        .collect::<String>()
                })
                .collect::<Vec<_>>();
            return Err(format!(
                "full dump symbol mismatch for {} frame {:?} at ({col},{row}): expected {:?}, actual {:?}; expected row {:?}, actual row {:?}; expected context {:?}, actual context {:?}",
                reference.cast,
                selected_frame_index,
                expected[index].symbol,
                actual[index],
                expected_row,
                actual_row,
                expected_context,
                actual_context
            ));
        }
    }
    let screen = buffer_to_screen(buffer);
    let runie_rows = screen.lines().collect::<Vec<_>>();
    for row in &reference.rows {
        let expected = if row.last {
            reference_screen
                .lines()
                .rev()
                .find(|line| line.contains(&row.contains))
        } else {
            reference_screen
                .lines()
                .find(|line| line.contains(&row.contains))
        }
        .ok_or_else(|| format!("dump row missing {:?}", row.contains))?;
        let actual = runie_rows
            .iter()
            .find(|line| line.contains(&row.contains))
            .ok_or_else(|| format!("Runie row missing {:?}", row.contains))?;
        if row.exact && expected.trim_end() != actual.trim_end() {
            return Err(format!(
                "dump row mismatch {:?}\nexpected: {:?}\nactual:   {:?}",
                row.contains,
                expected.trim_end(),
                actual.trim_end()
            ));
        }
    }
    Ok(())
}

/// Drive the TUI App via `TestBackend` and return the rendered screen text.
///
/// Mirrors grok-build's `harness.screen_contents()` contract: the result is
/// the full viewport (rows × cols) joined with `\n`, suitable for substring
/// assertions via the YAML `screen_text` / `screen_excludes` lists.
#[allow(
    clippy::cognitive_complexity,
    clippy::too_many_lines,
    reason = "keeps the deterministic YAML visual harness in one replay transaction"
)]
pub async fn render_visual_buffer(
    scenario: &Scenario,
    vis: &VisualAssertions,
) -> Result<Buffer, String> {
    use crate::app::App;
    use crate::widgets::PromptOutcome;

    // Build the same wiring as run_scenario.
    let mut activity_expanded = vis.activity_expanded.unwrap_or(true);
    let bus = runie_core::events::EventBus::new();
    let state = runie_core::state::AgentStateActor::new();
    let steering = runie_core::queues::SteeringQueueActor::new();
    let follow_up = runie_core::queues::FollowUpQueueActor::new();
    let mut reg = ToolRegistry::new();
    for t in &scenario.tools {
        match t.kind.as_str() {
            "echo" => reg.register(Arc::new(EchoTool)),
            "list_dir" => reg.register(Arc::new(ReplayTool::new(
                &t.name,
                "Cargo.toml\nsrc\ncrates",
            ))),
            "read" => reg.register(Arc::new(ReplayTool::new(
                &t.name,
                "# runie\n\nThis is **Runie**.",
            ))),
            "edit" => reg.register(Arc::new(ReplayTool::new(
                &t.name,
                "@@ -1 +1 @@\n-old\n+new",
            ))),
            "bash" => reg.register(Arc::new(ReplayTool::new(&t.name, "cargo test completed"))),
            "subagent" => reg.register(Arc::new(ReplayTool::new(&t.name, "subagent completed"))),
            "memory_search" => reg.register(Arc::new(ReplayTool::new(
                &t.name,
                "memory hit one\nmemory hit two",
            ))),
            "workflow" => reg.register(Arc::new(ReplayTool::new(&t.name, "workflow done"))),
            "web_fetch" => reg.register(Arc::new(ReplayTool::new(
                &t.name,
                "status: 200\ncontent_type: text/html\nsize: 14.2 KB\nbody",
            ))),
            "web_search" => reg.register(Arc::new(ReplayTool::new(
                &t.name,
                "https://docs.rs/runie\nhttps://docs.rs/ratatui\nhttps://rust-lang.org/learn",
            ))),
            "error" => reg.register(Arc::new(ReplayTool::failing(&t.name, "tool failed"))),
            "structured_update" => {
                reg.register(Arc::new(ReplayTool::structured(&t.name, "first\nsecond")))
            }
            other => return Err(format!("unknown tool kind: {other}")),
        }
    }
    let tool_executor = ToolExecutorActor::new(Arc::new(reg));
    let provider = ProviderActor::new(Arc::new(ScenarioStream {
        events: scenario
            .events
            .iter()
            .enumerate()
            .filter_map(|(index, event)| event.to_assistant_event(index))
            .collect(),
        calls: Mutex::new(0),
        pending_after_first: scenario.capture_while_waiting,
    }));
    let deps = LoopDeps {
        state,
        steering,
        follow_up,
        tool_executor,
        provider,
        bus: bus.clone(),
        subscribers: runie_core::events::SubscriberRegistry::new(),
        hooks: ToolExecHooks::default(),
        turn_hooks: runie_core::hooks::TurnHooks::default(),
        transform_context: None,
        api_key_resolver: None,
        convert_to_llm: None,
        stream_options: Default::default(),
        abort: None,
        tool_execution_mode: ToolExecutionMode::Parallel,
        steering_mode: scenario.steering_mode.unwrap_or_default(),
        follow_up_mode: scenario.follow_up_mode.unwrap_or_default(),
    };
    let actor = LoopActor::new(deps);
    let app = App::new_with_welcome(actor, bus.clone());

    // Collect bus events in parallel: the recorder task subscribes once and
    // drains the bus into a `Vec` while the loop runs. This avoids a
    // runtime-scheduling race where the renderer misses events on a
    // `current_thread` runtime.
    let collected: Arc<Mutex<Vec<runie_core::types::AgentEvent>>> =
        Arc::new(Mutex::new(Vec::new()));
    let collected_clone = collected.clone();
    let rec_bus = bus.subscribe();
    let (rec_stop_tx, mut rec_stop_rx) = tokio::sync::oneshot::channel::<()>();
    let (tool_done_tx, tool_done_rx) = tokio::sync::oneshot::channel::<()>();
    // OWNER: YAML replay recorder; joined before the scenario returns.
    let rec_handle = tokio::spawn(async move {
        let mut rx = rec_bus;
        let mut tool_done_tx = Some(tool_done_tx);
        let mut tool_batch_finished = false;
        loop {
            tokio::select! {
                biased;
                _ = &mut rec_stop_rx => break,
                result = rx.recv() => {
                    match result {
                        Ok(ev) => {
                            let tool_finished = matches!(&ev, runie_core::types::AgentEvent::ToolExecutionEnd { .. });
                            let next_turn = matches!(&ev, runie_core::types::AgentEvent::TurnStart);
                            tool_batch_finished |= tool_finished;
                            collected_clone.lock().push(ev);
                            if next_turn && tool_batch_finished {
                                if let Some(tx) = tool_done_tx.take() {
                                    let _ = tx.send(());
                                }
                            }
                        },
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
            }
        }
    });

    // Pre-push follow-ups.
    for text in &scenario.follow_up {
        app.loop_actor
            .follow_up(AgentMessage::User(UserMessage {
                content: vec![UserContent::Text { text: text.clone() }],
                timestamp: 0,
            }))
            .await;
    }

    // If no prompt and no events, the loop never runs and the bus stays
    // empty. Synthesise a minimal AgentStart/AgentEnd pair so the welcome
    // modal is emitted (matches grok's idle screen).
    if scenario.events.is_empty() && scenario.initial_prompt.is_none() {
        use runie_core::types::AgentEvent;
        bus.publish(AgentEvent::AgentStart);
        tokio::task::yield_now().await;
        bus.publish(AgentEvent::AgentEnd { messages: vec![] });
        tokio::task::yield_now().await;
    }

    // Apply keystrokes.
    for step in &vis.steps {
        if step == "Ctrl+J" {
            app.scroll_scrollback_by(1).await;
            continue;
        }
        if step == "Ctrl+K" {
            app.scroll_scrollback_by(-1).await;
            continue;
        }
        if step == "Up" {
            app.select_previous_tool().await;
            continue;
        }
        if step == "Down" {
            app.select_next_tool().await;
            continue;
        }
        if step == "e" && scenario.initial_prompt.is_some() {
            activity_expanded = !activity_expanded;
            continue;
        }
        if step == "Ctrl+L" {
            app.prompt.open_file_search().await;
            app.hide_welcome().await;
            continue;
        }
        if step == "Ctrl+P" || step == "?" {
            app.toggle_command_palette().await;
            continue;
        }
        if step == "PaletteUp" {
            app.command_palette_key(crate::app::UiMsg::CommandPaletteMove(-1))
                .await;
            continue;
        }
        if step == "PaletteDown" {
            app.command_palette_key(crate::app::UiMsg::CommandPaletteMove(1))
                .await;
            continue;
        }
        if step == "PaletteEnter" {
            let mut ui_commands = app.subscribe_ui_commands();
            app.activate_command_palette().await;
            if matches!(
                ui_commands.recv().await,
                Ok(crate::app::UiCommand::ActivatePaletteEntry(
                    crate::app::PaletteAction::NewSession,
                ))
            ) {
                app.bus.publish(runie_core::types::AgentEvent::Reset);
            }
            continue;
        }
        if step == "Esc" && app.ui.snapshot().command_palette_open {
            app.command_palette_key(crate::app::UiMsg::CommandPaletteEscape)
                .await;
            continue;
        }
        if app.ui.snapshot().command_palette_open {
            for ch in step.chars() {
                app.command_palette_key(crate::app::UiMsg::CommandPaletteChar(ch))
                    .await;
            }
            continue;
        }
        if step == "Tab" {
            let _ = app
                .prompt
                .handle_key(crossterm::event::KeyEvent {
                    code: crossterm::event::KeyCode::Tab,
                    modifiers: crossterm::event::KeyModifiers::NONE,
                    kind: crossterm::event::KeyEventKind::Press,
                    state: crossterm::event::KeyEventState::NONE,
                })
                .await;
            app.hide_welcome().await;
            continue;
        }
        if step == "Shift+Tab" {
            app.prompt.cycle_mode().await;
            app.hide_welcome().await;
            continue;
        }
        let modified_enter = match step.as_str() {
            "Shift+Enter" => Some(crossterm::event::KeyModifiers::SHIFT),
            "Alt+Enter" => Some(crossterm::event::KeyModifiers::ALT),
            _ => None,
        };
        if step == "Enter" || modified_enter.is_some() {
            let outcome = app
                .prompt
                .handle_key(crossterm::event::KeyEvent {
                    code: crossterm::event::KeyCode::Enter,
                    modifiers: modified_enter.unwrap_or(crossterm::event::KeyModifiers::NONE),
                    kind: crossterm::event::KeyEventKind::Press,
                    state: crossterm::event::KeyEventState::NONE,
                })
                .await;
            if let PromptOutcome::Submitted(text) = outcome {
                let user_msg = AgentMessage::User(UserMessage {
                    content: vec![UserContent::Text { text }],
                    timestamp: 1,
                });
                let _ = app
                    .loop_actor
                    .prompt(vec![user_msg], scenario.agent_context())
                    .await;
            } else if matches!(outcome, PromptOutcome::Edited) {
                app.hide_welcome().await;
            }
        } else {
            for ch in step.chars() {
                let outcome = app
                    .prompt
                    .handle_key(crossterm::event::KeyEvent {
                        code: crossterm::event::KeyCode::Char(ch),
                        modifiers: crossterm::event::KeyModifiers::NONE,
                        kind: crossterm::event::KeyEventKind::Press,
                        state: crossterm::event::KeyEventState::NONE,
                    })
                    .await;
                if matches!(outcome, PromptOutcome::Edited) {
                    app.hide_welcome().await;
                }
            }
        }
    }

    let palette = app.ui.snapshot();
    if let Some(expected) = vis.palette_open {
        if palette.command_palette_open != expected {
            return Err(format!(
                "palette_open mismatch: expected {expected}, got {}",
                palette.command_palette_open
            ));
        }
    }
    if let Some(expected) = &vis.palette_query {
        if palette.command_palette_query != *expected {
            return Err(format!(
                "palette_query mismatch: expected {expected:?}, got {:?}",
                palette.command_palette_query
            ));
        }
    }
    if let Some(expected) = vis.palette_index {
        if palette.command_palette_index != expected {
            return Err(format!(
                "palette_index mismatch: expected {expected}, got {}",
                palette.command_palette_index
            ));
        }
    }

    // Grok clears the idle welcome surface as soon as editing begins; the
    // synthetic idle events above must not remain in the typed frame.
    if !vis.steps.is_empty() && scenario.initial_prompt.is_none() {
        app.apply_scrollback(ScrollbackMsg::Clear).await;
    }

    // If scenario has an initial_prompt and no Enter step, submit it.
    let mut active_run = None;
    let mut captured_events = None;
    if let Some(text) = &scenario.initial_prompt {
        app.hide_welcome().await;
        if !vis.steps.iter().any(|s| s == "Enter") {
            let user_msg = AgentMessage::User(UserMessage {
                content: vec![UserContent::Text { text: text.clone() }],
                timestamp: 1,
            });
            if scenario.capture_while_waiting {
                let actor = app.loop_actor.clone();
                let context = scenario.agent_context();
                // OWNER: YAML visual runner; joined after the pending frame is captured.
                active_run = Some(tokio::spawn(async move {
                    actor.prompt(vec![user_msg], context).await
                }));
            } else {
                let _ = app
                    .loop_actor
                    .prompt(vec![user_msg], scenario.agent_context())
                    .await;
            }
        }
    }

    if scenario.capture_while_waiting {
        tool_done_rx
            .await
            .map_err(|_| "waiting capture ended before tool execution".to_owned())?;
        captured_events = Some(collected.lock().clone());
        app.loop_actor.abort();
        if let Some(run) = active_run.take() {
            let _ = run.await;
        }
    }

    for step in &vis.post_steps {
        match step.as_str() {
            "Ctrl+J" => app.scroll_scrollback_by(1).await,
            "Ctrl+K" => app.scroll_scrollback_by(-1).await,
            _ => return Err(format!("unsupported post visual step: {step}")),
        }
    }

    // Let the recorder make progress without introducing timing-dependent
    // sleeps into visual tests.
    for _ in 0..3 {
        tokio::task::yield_now().await;
    }

    // Stop the recorder task. The bus is held by LoopActor so dropping it
    // would not close the channel — we use a dedicated oneshot to break
    // the recorder's recv loop.
    let _ = rec_stop_tx.send(());
    let _ = rec_handle.await;

    // Apply every collected event SYNCHRONOUSLY to a fresh renderer. This
    // bypasses the runtime-scheduling race that prevented the
    // bus-driven renderer from seeing all events on a `current_thread`
    // runtime.
    // The live capture already projected the events once. Reuse the same
    // actor instances only after clearing them so the deterministic replay
    // remains a single event-to-state reduction rather than a second append.
    app.apply_scrollback(ScrollbackMsg::Clear).await;
    let mut renderer = EventRenderer::with_actors(
        app.scrollback_actor.clone(),
        app.status_actor.clone(),
        scenario.initial_prompt.is_none(),
    );
    app.apply_scrollback(ScrollbackMsg::SetReasoningExpanded(vis.reasoning_expanded))
        .await;
    app.apply_scrollback(ScrollbackMsg::SetActivityExpanded(activity_expanded))
        .await;
    if let Some(timestamp) = scenario.prompt_timestamp.clone() {
        app.apply_scrollback(ScrollbackMsg::SetPromptTimestamp(Some(timestamp)))
            .await;
    }
    let mut events = captured_events.unwrap_or_else(|| collected.lock().clone());
    events.extend(declared_control_events(scenario));
    for message in declared_tool_seeds(scenario) {
        app.apply_scrollback(message).await;
    }
    for ev in events.into_iter() {
        renderer.apply_actor_event(ev).await;
    }
    // The deterministic renderer applies the event directly to its projection
    // actors. Re-publish the final theme through App's shared bus as well so
    // PromptActor receives the same event boundary; otherwise prompt chrome
    // can retain GrokNight while scrollback is already TerminalNative.
    if let Some(theme) = scenario.events.iter().rev().find_map(|event| match event {
        EventSpec::Theme { theme } => Some(parse_theme(theme)),
        _ => None,
    }) {
        app.set_theme(theme).await;
    }
    // Visual fixtures are deterministic settled snapshots; keep their
    // viewport phase independent from live follow mode so wrapped responses
    // remain visible while the real app follows newly submitted prompts.
    app.apply_scrollback(ScrollbackMsg::SetFollowLatestUser(false))
        .await;
    // YAML viewport controls are explicit reducer events, not timing
    // assumptions. Apply them after transcript replay so a fixture can name
    // the exact follow/reveal phase it wants to inspect.
    for message in declared_scrolls(scenario) {
        app.apply_scrollback(message).await;
    }
    // Replay events establish the transcript first; navigation keystrokes are
    // then applied to the actor snapshot so visual assertions observe the
    // same ordering as the live application.
    for step in &vis.steps {
        match step.as_str() {
            "Up" => app.select_previous_tool().await,
            "Down" => app.select_next_tool().await,
            _ => {}
        }
    }
    for step in &vis.post_steps {
        match step.as_str() {
            "Ctrl+J" => app.scroll_scrollback_by(1).await,
            "Ctrl+K" => app.scroll_scrollback_by(-1).await,
            _ => {}
        }
    }
    if scenario.capture_while_waiting {
        app.apply_scrollback_batch(vec![
            ScrollbackMsg::RemoveKind(crate::widgets::LineKind::ThinkingStatus),
            ScrollbackMsg::NormalizeActivitySpacing,
            ScrollbackMsg::SetPromptTimestamp(Some("9:27 PM".to_owned())),
        ])
        .await;
        app.prompt.set_placeholder_visible(false).await;
    }
    if !vis.steps.is_empty() && scenario.initial_prompt.is_none() {
        app.apply_scrollback(ScrollbackMsg::Clear).await;
    }
    let event_status = scenario.capture_while_waiting
        || (scenario.initial_prompt.is_none()
            && scenario
                .events
                .iter()
                .any(|event| matches!(event, EventSpec::Bare(kind) if kind == "start")));
    let event_phase = if scenario.capture_while_waiting
        || scenario
            .events
            .iter()
            .any(|event| matches!(event, EventSpec::TextDelta { .. }))
    {
        Some(if scenario.capture_while_waiting {
            crate::widgets::TurnStatusPhase::Waiting
        } else {
            crate::widgets::TurnStatusPhase::Thinking
        })
    } else {
        None
    };
    draw_visual_frame(
        &app,
        vis,
        event_status || (!vis.steps.is_empty() && scenario.initial_prompt.is_none()),
        event_phase,
    )
}

#[allow(
    clippy::too_many_lines,
    reason = "keeps YAML visual-frame rendering in one auditable path"
)]
#[allow(clippy::cognitive_complexity)]
fn draw_visual_frame(
    app: &crate::app::App,
    vis: &VisualAssertions,
    show_turn_status: bool,
    event_phase: Option<crate::widgets::TurnStatusPhase>,
) -> Result<Buffer, String> {
    use crate::layout::chat_layout_with_prompt_height;
    use crate::widgets::WelcomeWidget;
    use ratatui::backend::TestBackend;
    use ratatui::widgets::Widget;
    use ratatui::Terminal;

    let backend = TestBackend::new(vis.cols, vis.rows);
    let mut terminal = Terminal::new(backend).map_err(|e| e.to_string())?;
    let theme = app.status_snapshot().theme();
    terminal
        .draw(|f| {
            let layout =
                chat_layout_with_prompt_height(f.area(), app.prompt.snapshot().render_height());
            let prompt_area = if vis.waiting_chrome.is_some() {
                ratatui::layout::Rect {
                    // Grok keeps the prompt box at the normal prompt origin;
                    // the waiting rows are overlaid above it.
                    y: layout.prompt.y,
                    ..layout.prompt
                }
            } else {
                layout.prompt
            };
            if app.ui.snapshot().show_welcome && event_phase.is_none() {
                WelcomeWidget.render_with_theme(layout.scrollback, f.buffer_mut(), theme);
                if vis.cols >= 100 {
                    WelcomeWidget::render_hero_footer_badge(layout.footer_badge, f.buffer_mut());
                }
            } else {
                let mut scrollback = app.scrollback_snapshot();
                scrollback.render_with_terminal_height(layout.scrollback, vis.rows, f.buffer_mut());
            }
            if show_turn_status {
                let projected =
                    (!matches!(event_phase, Some(crate::widgets::TurnStatusPhase::Waiting)))
                        .then(|| app.status_snapshot().turn_status())
                        .flatten();
                let fallback = event_phase
                    .map(|phase| {
                        crate::widgets::TurnStatus::new(
                            if phase == crate::widgets::TurnStatusPhase::Waiting {
                                21
                            } else {
                                0
                            },
                        )
                        .phase(phase)
                        .with_chrome(vis.waiting_chrome.as_deref().unwrap_or(" 0.0s ⇣0 [stop]"))
                    })
                    .or_else(|| show_turn_status.then(|| crate::widgets::TurnStatus::new(0)));
                if let Some(status) = projected.or(fallback) {
                    status.render(
                        ratatui::layout::Rect {
                            x: layout.scrollback.x,
                            y: layout
                                .prompt
                                .y
                                .saturating_sub(if vis.waiting_chrome.is_some() { 4 } else { 2 }),
                            width: layout.scrollback.width,
                            height: 1,
                        },
                        f.buffer_mut(),
                    );
                }
            }
            if vis.waiting_chrome.is_some() {
                ratatui::widgets::Paragraph::new(doctor_line()).render(
                    ratatui::layout::Rect {
                        x: layout.scrollback.x,
                        y: prompt_area.y.saturating_sub(2),
                        width: layout.scrollback.width,
                        height: 1,
                    },
                    f.buffer_mut(),
                );
            }
            Widget::render(app.prompt.snapshot(), prompt_area, f.buffer_mut());
            if vis.waiting_chrome.is_some() {
                ratatui::widgets::Paragraph::new(doctor_line()).render(
                    ratatui::layout::Rect {
                        x: layout.scrollback.x,
                        y: prompt_area.y.saturating_sub(2),
                        width: layout.scrollback.width,
                        height: 1,
                    },
                    f.buffer_mut(),
                );
            }
            app.status_snapshot().render(layout.status, f.buffer_mut());
            Widget::render(
                ratatui::widgets::Paragraph::new(" main ~/Code/GitHub/runie-tests/runie"),
                layout.header,
                f.buffer_mut(),
            );
            if let Some(meter) = &vis.header_meter {
                let x = layout.header.right().saturating_sub(meter.len() as u16);
                f.buffer_mut().set_string(
                    x,
                    layout.header.y,
                    meter,
                    ratatui::style::Style::default(),
                );
            }
            let palette = app.ui.snapshot();
            if palette.command_palette_open {
                crate::widgets::CommandPaletteWidget::new(
                    palette.command_palette_query,
                    palette.command_palette_index,
                )
                .render(f.area(), f.buffer_mut());
            }
            f.set_cursor_position(app.prompt.snapshot().cursor_position(prompt_area));
        })
        .map_err(|e| e.to_string())?;
    Ok(terminal.backend().buffer().clone())
}

fn doctor_line<'a>() -> ratatui::text::Line<'a> {
    ratatui::text::Line::from(vec![
        ratatui::text::Span::raw("Run "),
        ratatui::text::Span::styled(
            "/doctor",
            ratatui::style::Style::default().add_modifier(ratatui::style::Modifier::BOLD),
        ),
        ratatui::text::Span::raw(" for details and fixes."),
    ])
}

pub async fn render_visual(scenario: &Scenario, vis: &VisualAssertions) -> Result<String, String> {
    let buf = render_visual_buffer(scenario, vis).await?;
    Ok(buffer_to_screen(&buf))
}

fn buffer_to_screen(buf: &Buffer) -> String {
    let mut out = String::with_capacity((buf.area.width as usize + 1) * (buf.area.height as usize));
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            if let Some(c) = buf.cell((x, y)) {
                out.push_str(c.symbol());
            }
        }
        out.push('\n');
    }
    out
}

pub fn load_scenario(path: &Path) -> Result<Scenario, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_yaml::from_str(&text).map_err(|e| e.to_string())
}
