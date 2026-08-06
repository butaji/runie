//! `EventRenderer` — subscribes to `runie-core`'s event bus and mutates widgets.

use std::collections::HashMap;
use std::time::Duration;

#[cfg(test)]
use parking_lot::Mutex;
#[cfg(test)]
use runie_core::types::AssistantContent;
use runie_core::types::{AgentEvent, AssistantMessageEvent};
#[cfg(test)]
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::widgets::{Line, LineKind, Scrollback, ScrollbackMsg, Status, StatusBar, StatusMsg};
use crate::{ScrollbackActor, StatusActor};

const LIVE_TIMESTAMP_SECONDS_MIN: i64 = 1_000_000_000;
const DEFAULT_THINKING_ELAPSED_MS: u64 = 900;

fn default_tool_display_mode(tool_name: &str) -> runie_core::types::ToolDisplayMode {
    if matches!(tool_name, "bash" | "shell" | "exec" | "run") {
        runie_core::types::ToolDisplayMode::Truncated
    } else {
        runie_core::types::ToolDisplayMode::Collapsed
    }
}

/// Pure mapping for status-owned portions of the core event stream.
#[allow(
    clippy::too_many_lines,
    reason = "status event projection keeps terminal usage and error paths declarative"
)]
pub fn status_messages_for_event(event: &AgentEvent) -> Vec<StatusMsg> {
    match event {
        AgentEvent::AgentStart => vec![StatusMsg::Set(Status::Thinking)],
        AgentEvent::Error { message } => vec![StatusMsg::Set(Status::Error(message.clone()))],
        AgentEvent::TurnStart => vec![StatusMsg::BeginTurn, StatusMsg::Set(Status::Thinking)],
        AgentEvent::Waiting { reason } => {
            vec![StatusMsg::Set(Status::Waiting(reason.clone()))]
        }
        AgentEvent::ThemeChanged { theme } => vec![StatusMsg::SetTheme(*theme)],
        AgentEvent::Reset => vec![StatusMsg::Set(Status::Ready)],
        AgentEvent::TurnEnd { .. } | AgentEvent::AgentEnd { .. } => {
            vec![StatusMsg::Set(Status::Ready)]
        }
        AgentEvent::MessageUpdate { event, .. } => match event {
            AssistantMessageEvent::TextDelta { .. } => vec![StatusMsg::Set(Status::Streaming)],
            AssistantMessageEvent::ThinkingDelta { .. } => vec![StatusMsg::Set(Status::Thinking)],
            AssistantMessageEvent::Done {
                stop_reason, usage, ..
            } => vec![
                StatusMsg::FinishTurn(usage.clone(), *stop_reason),
                StatusMsg::Set(Status::Ready),
            ],
            AssistantMessageEvent::Error { error, .. } => {
                vec![StatusMsg::Set(Status::Error(error.error_text()))]
            }
            _ => Vec::new(),
        },
        AgentEvent::MessageEnd {
            message: runie_core::types::AgentMessage::Assistant(assistant),
        } => {
            if let Some(error) = assistant.error_message.as_ref() {
                vec![StatusMsg::Set(Status::Error(error.clone()))]
            } else if let Some(stop_reason) = assistant.stop_reason {
                vec![
                    StatusMsg::FinishTurn(assistant.usage.clone(), stop_reason),
                    StatusMsg::Set(Status::Ready),
                ]
            } else {
                Vec::new()
            }
        }
        _ => Vec::new(),
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "the event projection table keeps actor-owned mappings declarative"
)]
pub fn scrollback_messages_for_event(event: &AgentEvent) -> Vec<ScrollbackMsg> {
    match event {
        AgentEvent::MessageStart {
            message: runie_core::types::AgentMessage::User(user),
        } => {
            let text = user
                .content
                .iter()
                .map(|content| match content {
                    runie_core::types::UserContent::Text { text } => text.as_str(),
                    runie_core::types::UserContent::Image { .. } => "[image]",
                })
                .collect::<Vec<_>>()
                .join("");
            let mut messages = vec![ScrollbackMsg::Append(
                Line::new(LineKind::User, text).with_vpad(true),
            )];
            if user.timestamp >= LIVE_TIMESTAMP_SECONDS_MIN {
                messages.push(ScrollbackMsg::SetPromptTimestamp(Some(
                    format_clock_timestamp(user.timestamp),
                )));
            }
            messages
        }
        AgentEvent::MessageStart {
            message: runie_core::types::AgentMessage::Assistant(_),
        } => vec![
            ScrollbackMsg::Append(Line::new(LineKind::Separator, "")),
            ScrollbackMsg::Append(Line::new(LineKind::ThinkingStatus, "◆ Thinking…")),
            ScrollbackMsg::Append(Line::new(LineKind::Separator, "")),
            ScrollbackMsg::Append(Line::new(LineKind::Assistant, "")),
        ],
        AgentEvent::MessageUpdate {
            event: AssistantMessageEvent::TextDelta { delta, .. },
            ..
        } => vec![ScrollbackMsg::AppendToLastByKind(
            LineKind::Assistant,
            delta.clone(),
        )],
        AgentEvent::MessageUpdate {
            event: AssistantMessageEvent::ThinkingDelta { delta, .. },
            ..
        } => vec![ScrollbackMsg::AppendToLastByKind(
            LineKind::Reasoning,
            delta.clone(),
        )],
        AgentEvent::Reset => vec![ScrollbackMsg::Clear],
        AgentEvent::ThemeChanged { theme } => vec![ScrollbackMsg::SetTheme(*theme)],
        AgentEvent::ToolDisplayModeChanged { tool_call_id, mode } => {
            vec![ScrollbackMsg::SetToolMode(tool_call_id.clone(), *mode)]
        }
        AgentEvent::ToolExecutionStart {
            tool_call_id,
            tool_name,
            ..
        } => vec![
            ScrollbackMsg::SetToolName(tool_call_id.clone(), tool_name.clone()),
            ScrollbackMsg::SetToolMode(tool_call_id.clone(), default_tool_display_mode(tool_name)),
        ],
        AgentEvent::BackgroundWorkStarted {
            work_id,
            description,
            background,
        } => vec![ScrollbackMsg::ToolStart {
            tool_call_id: work_id.clone(),
            header: format!(
                "Subagent {}: {description:?}",
                if *background { "started" } else { "running" }
            ),
            activity: None,
        }],
        AgentEvent::BackgroundWorkProgress {
            work_id,
            description,
            activity,
        } => vec![ScrollbackMsg::ToolUpdate {
            tool_call_id: work_id.clone(),
            header: Some(format!("Subagent running: {description:?} — {activity}")),
            output: Vec::new(),
        }],
        AgentEvent::BackgroundWorkFinished {
            work_id,
            description,
            is_error,
            elapsed_ms,
            error,
        } => {
            let mut messages = vec![ScrollbackMsg::ToolEnd {
                tool_call_id: work_id.clone(),
                header: format!(
                    "Subagent {}{}{}: {description:?}",
                    if *is_error { "failed" } else { "completed" },
                    format_elapsed(*elapsed_ms),
                    format_error(*is_error, error.as_deref())
                ),
                activity: None,
                output: Vec::new(),
            }];
            if *is_error {
                messages.push(ScrollbackMsg::MarkToolError(work_id.clone()));
            }
            messages
        }
        AgentEvent::BackgroundWorkCancelled {
            work_id,
            description,
            elapsed_ms,
        } => vec![
            ScrollbackMsg::ToolEnd {
                tool_call_id: work_id.clone(),
                header: format!(
                    "Subagent cancelled{}: {description:?}",
                    format_elapsed(*elapsed_ms)
                ),
                activity: None,
                output: Vec::new(),
            },
            ScrollbackMsg::MarkToolError(work_id.clone()),
        ],
        AgentEvent::WorkflowStarted {
            run_id,
            name,
            objective,
        } => vec![
            ScrollbackMsg::SetToolName(run_id.clone(), "workflow".into()),
            ScrollbackMsg::WorkflowStart {
                run_id: run_id.clone(),
                name: name.clone(),
                objective: objective.clone(),
            },
        ],
        AgentEvent::WorkflowProgress {
            run_id,
            phase,
            state,
            active_agents,
        } => vec![ScrollbackMsg::WorkflowProgress {
            run_id: run_id.clone(),
            phase: phase.clone(),
            state: state.clone(),
            active_agents: *active_agents,
        }],
        AgentEvent::WorkflowFinished {
            run_id,
            status,
            elapsed_ms,
        } => vec![ScrollbackMsg::WorkflowEnd {
            run_id: run_id.clone(),
            status: status.clone(),
            elapsed_ms: *elapsed_ms,
        }],
        _ => Vec::new(),
    }
}

fn format_elapsed(elapsed_ms: Option<u64>) -> String {
    elapsed_ms
        .map(|millis| format!(" in {:.1}s", millis as f64 / 1_000.0))
        .unwrap_or_default()
}

fn thinking_summary(elapsed_ms: Option<u64>) -> String {
    let elapsed_ms = elapsed_ms.unwrap_or(DEFAULT_THINKING_ELAPSED_MS);
    format!("◆ Thought for {:.1}s", elapsed_ms as f64 / 1_000.0)
}

fn format_error(is_error: bool, error: Option<&str>) -> String {
    if is_error {
        error.map(|value| format!(" ({value})")).unwrap_or_default()
    } else {
        String::new()
    }
}

#[derive(Clone)]
enum Projection<T> {
    #[cfg(test)]
    Legacy(Arc<Mutex<T>>),
    #[cfg(test)]
    Actor,
    #[cfg(not(test))]
    Actor(std::marker::PhantomData<T>),
}

impl<T> Projection<T> {
    fn actor() -> Self {
        #[cfg(test)]
        {
            Self::Actor
        }
        #[cfg(not(test))]
        {
            Self::Actor(std::marker::PhantomData)
        }
    }

    #[cfg(test)]
    fn lock(&self) -> parking_lot::MutexGuard<'_, T> {
        match self {
            Self::Legacy(value) => value.lock(),
            Self::Actor => panic!("legacy projection accessed from actor renderer"),
        }
    }

    #[cfg(not(test))]
    fn lock(&self) -> T {
        // Production projections are actor-backed. A compatibility branch
        // must never manufacture a default snapshot, which would create a
        // silent second source of truth.
        panic!("legacy projection accessed from actor renderer")
    }

    #[cfg(test)]
    fn legacy_arc(&self) -> Arc<Mutex<T>> {
        match self {
            Self::Legacy(value) => value.clone(),
            Self::Actor => panic!("legacy projection accessed from actor renderer"),
        }
    }
}

pub struct EventRenderer {
    scrollback: Projection<Scrollback>,
    scrollback_actor: Option<ScrollbackActor>,
    status: Projection<StatusBar>,
    /// Actor-owned status projection used by the production event loop while
    /// the compatibility widget projection is being retired.
    status_actor: Option<StatusActor>,
    /// Accumulated text while an assistant message is streaming.
    streaming_buffer: String,
    /// Tool rows are keyed by the core tool-call id because parallel tools may
    /// update and finish in a different order than they started.
    tool_rows: HashMap<String, usize>,
    tool_buffers: HashMap<String, String>,
    /// True between MessageStart(assistant) and MessageEnd(assistant).
    in_assistant_stream: bool,
    in_reasoning: bool,
    reasoning_buffer: String,
    thinking_elapsed_ms: Option<u64>,
    /// True between ToolExecutionStart and ToolExecutionEnd.
    in_tool_exec: bool,
    activity_dirs: usize,
    activity_files: usize,
    activity_commands: usize,
    activity_subagents: usize,
    activity_failures: usize,
    active_tool_count: usize,
    activity_group_open: bool,
    turn_started: bool,
    /// If true, the next AgentStart emits the welcome modal lines
    /// (matching grok's minimal-mode chrome) and then clears this flag.
    emit_welcome: bool,
}

impl EventRenderer {
    #[cfg(test)]
    pub fn new(scrollback: Arc<Mutex<Scrollback>>, status: Arc<Mutex<StatusBar>>) -> Self {
        Self::with_welcome(scrollback, status, false)
    }

    #[cfg(test)]
    pub fn with_welcome(
        scrollback: Arc<Mutex<Scrollback>>,
        status: Arc<Mutex<StatusBar>>,
        emit_welcome: bool,
    ) -> Self {
        Self::with_projections(
            Projection::Legacy(scrollback),
            Projection::Legacy(status),
            None,
            None,
            emit_welcome,
        )
    }

    fn with_projections(
        scrollback: Projection<Scrollback>,
        status: Projection<StatusBar>,
        scrollback_actor: Option<ScrollbackActor>,
        status_actor: Option<StatusActor>,
        emit_welcome: bool,
    ) -> Self {
        Self {
            scrollback,
            scrollback_actor,
            status,
            status_actor,
            streaming_buffer: String::new(),
            tool_rows: HashMap::new(),
            tool_buffers: HashMap::new(),
            in_assistant_stream: false,
            in_reasoning: false,
            reasoning_buffer: String::new(),
            thinking_elapsed_ms: None,
            in_tool_exec: false,
            activity_dirs: 0,
            activity_files: 0,
            activity_commands: 0,
            activity_subagents: 0,
            activity_failures: 0,
            active_tool_count: 0,
            activity_group_open: false,
            turn_started: false,
            emit_welcome,
        }
    }

    /// Build the production renderer with its SSOT actors attached at
    /// construction time. The compatibility constructors remain for the
    /// synchronous YAML harness and focused reducer tests.
    pub fn with_actors(
        scrollback_actor: ScrollbackActor,
        status_actor: StatusActor,
        emit_welcome: bool,
    ) -> Self {
        Self::with_projections(
            Projection::actor(),
            Projection::actor(),
            Some(scrollback_actor),
            Some(status_actor),
            emit_welcome,
        )
    }

    /// Drain bus events until the channel closes. Returns when receiver hits
    /// `RecvStreamLagged` or `Closed`.
    #[allow(
        clippy::cognitive_complexity,
        clippy::too_many_lines,
        reason = "event loop coordinates owned status/feed projections and shutdown"
    )]
    pub async fn run(
        mut self,
        mut rx: broadcast::Receiver<AgentEvent>,
        mut shutdown: tokio::sync::watch::Receiver<bool>,
    ) {
        let status_actor = self
            .status_actor
            .clone()
            .expect("production EventRenderer::run requires a StatusActor");
        let scrollback_actor = self
            .scrollback_actor
            .clone()
            .expect("production EventRenderer::run requires a ScrollbackActor");
        const ANIMATION_TICK: Duration = Duration::from_millis(50);
        let mut tick = Box::pin(tokio::time::sleep(ANIMATION_TICK));
        loop {
            let animation_demand = status_actor.snapshot().animation_demand()
                || scrollback_actor.snapshot().animation_demand();
            tokio::select! {
                biased;
                _ = &mut tick, if animation_demand => {
                    self.advance_animation(&status_actor).await;
                    tick.as_mut().reset(tokio::time::Instant::now() + ANIMATION_TICK);
                }
                _ = shutdown.changed() => {
                    if *shutdown.borrow() { break; }
                }
                result = rx.recv() => {
                    match result {
                        Ok(event) => {
                            self.record_thinking_elapsed(&event);
                            let actor_tool_start = match &event {
                                AgentEvent::ToolExecutionStart {
                                    tool_call_id,
                                    tool_name,
                                    args,
                                } => Some(self.handle_tool_start(
                                    tool_call_id.clone(),
                                    tool_name.clone(),
                                    args.clone(),
                                )),
                                _ => None,
                            };
                            let actor_tool_update = match &event {
                                AgentEvent::ToolExecutionUpdate {
                                    tool_call_id,
                                    partial_result,
                                    ..
                                } => self.handle_tool_update(
                                    tool_call_id.clone(),
                                    partial_result.clone(),
                                ),
                                _ => None,
                            };
                            let actor_tool_end = match &event {
                                AgentEvent::ToolExecutionEnd {
                                    tool_call_id,
                                    tool_name,
                                    result,
                                    is_error,
                                } => Some(self.handle_tool_end(
                                    tool_call_id.clone(),
                                    tool_name.clone(),
                                    result.clone(),
                                    *is_error,
                                )),
                                _ => None,
                            };
                            let mut feed_messages = if matches!(
                                event,
                                AgentEvent::BackgroundWorkStarted { .. }
                                    | AgentEvent::BackgroundWorkProgress { .. }
                                    | AgentEvent::BackgroundWorkFinished { .. }
                                    | AgentEvent::BackgroundWorkCancelled { .. }
                            ) {
                                Vec::new()
                            } else {
                                scrollback_messages_for_event(&event)
                            };
                            if matches!(event, AgentEvent::AgentStart) {
                                feed_messages.extend(agent_start_messages(self.emit_welcome));
                            }
                            if let AgentEvent::MessageEnd {
                                message: runie_core::types::AgentMessage::Assistant(_),
                            } = &event
                            {
                                feed_messages.push(ScrollbackMsg::FinalizeAssistant {
                                    has_reasoning: !self.reasoning_buffer.is_empty(),
                                    reasoning_expanded: scrollback_actor.snapshot().reasoning_expanded(),
                                    summary: thinking_summary(self.thinking_elapsed_ms),
                                    settled_no_tool_phase: self.thinking_elapsed_ms.is_some()
                                        && self.tool_rows.is_empty(),
                                });
                            }
                            if matches!(event, AgentEvent::AgentEnd { .. }) && self.turn_started {
                                feed_messages.push(ScrollbackMsg::AppendTurnSummary(
                                    status_actor.snapshot().worked_for_label(),
                                ));
                            }
                            if let AgentEvent::MessageUpdate {
                                event: AssistantMessageEvent::Error { error, .. },
                                ..
                            } = &event
                            {
                                feed_messages.push(ScrollbackMsg::Append(Line::new(
                                    LineKind::System,
                                    format!("error: {}", error.error_text()),
                                )));
                            }
                            if let AgentEvent::MessageEnd {
                                message: runie_core::types::AgentMessage::Assistant(assistant),
                            } = &event
                            {
                                if let Some(error) = &assistant.error_message {
                                    feed_messages.push(ScrollbackMsg::Append(Line::new(
                                        LineKind::System,
                                        format!("error: {error}"),
                                    )));
                                }
                            }
                            if !feed_messages.is_empty() {
                                scrollback_actor.apply_batch(feed_messages).await;
                            }
                            if let Some(tool_start) = actor_tool_start {
                                scrollback_actor.apply(tool_start).await;
                            } else if let Some(tool_update) = actor_tool_update {
                                let _ = tool_update;
                            } else if let Some(tool_end) = actor_tool_end {
                                scrollback_actor.apply(tool_end).await;
                            } else {
                                self.apply_event(event);
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            scrollback_actor
                                .apply(ScrollbackMsg::Append(Line::new(
                                    LineKind::System,
                                    format!("(skipped {n} events)"),
                                )))
                                .await;
                        }
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
            }
        }
    }

    /// Apply one recorded event through the same acknowledged actor
    /// projections used by the live bus loop. This is the YAML replay seam;
    /// it keeps event ordering deterministic without falling back to a
    /// mutex-owned snapshot.
    #[allow(
        clippy::cognitive_complexity,
        clippy::too_many_lines,
        reason = "actor replay keeps one event-to-projection transaction explicit"
    )]
    pub async fn apply_actor_event(&mut self, event: AgentEvent) {
        self.record_thinking_elapsed(&event);
        let status_actor = self
            .status_actor
            .clone()
            .expect("actor replay requires a StatusActor");
        let scrollback_actor = self
            .scrollback_actor
            .clone()
            .expect("actor replay requires a ScrollbackActor");
        status_actor.apply_event(&event).await;
        let tool_message = match &event {
            AgentEvent::ToolExecutionStart {
                tool_call_id,
                tool_name,
                args,
            } => {
                Some(self.handle_tool_start(tool_call_id.clone(), tool_name.clone(), args.clone()))
            }
            AgentEvent::ToolExecutionUpdate {
                tool_call_id,
                partial_result,
                ..
            } => self.handle_tool_update(tool_call_id.clone(), partial_result.clone()),
            AgentEvent::ToolExecutionEnd {
                tool_call_id,
                tool_name,
                result,
                is_error,
            } => Some(self.handle_tool_end(
                tool_call_id.clone(),
                tool_name.clone(),
                result.clone(),
                *is_error,
            )),
            _ => None,
        };
        let mut messages = scrollback_messages_for_event(&event);
        if matches!(event, AgentEvent::AgentStart) {
            messages.extend(agent_start_messages(self.emit_welcome));
        }
        if let AgentEvent::MessageEnd {
            message: runie_core::types::AgentMessage::Assistant(_),
        } = &event
        {
            messages.push(ScrollbackMsg::FinalizeAssistant {
                has_reasoning: !self.reasoning_buffer.is_empty(),
                reasoning_expanded: scrollback_actor.snapshot().reasoning_expanded(),
                summary: thinking_summary(self.thinking_elapsed_ms),
                settled_no_tool_phase: self.thinking_elapsed_ms.is_some()
                    && self.tool_rows.is_empty(),
            });
        }
        if let AgentEvent::MessageUpdate {
            event: AssistantMessageEvent::Error { error, .. },
            ..
        } = &event
        {
            messages.push(ScrollbackMsg::Append(Line::new(
                LineKind::System,
                format!("error: {}", error.error_text()),
            )));
        }
        if let AgentEvent::MessageEnd {
            message: runie_core::types::AgentMessage::Assistant(assistant),
        } = &event
        {
            if let Some(error) = &assistant.error_message {
                messages.push(ScrollbackMsg::Append(Line::new(
                    LineKind::System,
                    format!("error: {error}"),
                )));
            }
        }
        if matches!(event, AgentEvent::AgentEnd { .. }) && self.turn_started {
            messages.push(ScrollbackMsg::Append(Line::new(LineKind::Separator, "")));
            messages.push(ScrollbackMsg::AppendTurnSummary(
                status_actor.snapshot().worked_for_label(),
            ));
        }
        if !messages.is_empty() {
            scrollback_actor.apply_batch(messages).await;
        }
        if let Some(message) = tool_message {
            scrollback_actor.apply(message).await;
        } else {
            self.apply_event(event);
        }
    }

    async fn advance_animation(&self, actor: &StatusActor) {
        actor.apply(StatusMsg::AdvanceAnimation).await;
        if let Some(scrollback_actor) = &self.scrollback_actor {
            scrollback_actor
                .apply(ScrollbackMsg::AdvanceAnimation)
                .await;
        }
    }

    #[allow(
        clippy::cognitive_complexity,
        clippy::too_many_lines,
        reason = "event loop coordinates owned status/feed projections and shutdown"
    )]
    pub fn apply_event(&mut self, event: AgentEvent) {
        self.record_thinking_elapsed(&event);
        if self.status_actor.is_none() {
            for message in status_messages_for_event(&event) {
                self.status.lock().apply(message);
            }
        }
        match event {
            AgentEvent::AgentStart => {
                self.handle_agent_start();
            }
            AgentEvent::AgentEnd { .. } => self.handle_agent_end(),
            AgentEvent::Error { .. } => {}
            AgentEvent::ThinkingLevelChanged { .. } => {}
            AgentEvent::Reset => self.handle_reset(),
            AgentEvent::TurnStart => {
                self.turn_started = true;
            }
            AgentEvent::Waiting { reason } => {
                let _ = reason;
            }
            AgentEvent::ThemeChanged { theme } => {
                if self.scrollback_actor.is_none() {
                    self.scrollback.lock().set_theme(theme);
                }
            }
            AgentEvent::ToolDisplayModeChanged { tool_call_id, mode } => {
                if self.scrollback_actor.is_none() {
                    self.scrollback.lock().set_tool_mode(tool_call_id, mode);
                }
            }
            AgentEvent::TurnEnd { .. } => {}
            AgentEvent::MessageStart { message } => {
                self.handle_message_start(message);
            }
            AgentEvent::MessageUpdate { event, .. } => self.handle_message_update(event),
            AgentEvent::MessageEnd { message } => self.handle_message_end(message),
            AgentEvent::ToolExecutionStart {
                tool_call_id,
                tool_name,
                args,
            } => {
                let _ = self.handle_tool_start(tool_call_id.clone(), tool_name.clone(), args);
                if self.scrollback_actor.is_none() {
                    self.scrollback
                        .lock()
                        .set_tool_mode(tool_call_id, default_tool_display_mode(&tool_name));
                }
            }
            AgentEvent::ToolExecutionUpdate {
                tool_call_id,
                partial_result,
                ..
            } => {
                let _ = self.handle_tool_update(tool_call_id, partial_result);
            }
            AgentEvent::ToolExecutionEnd {
                tool_call_id,
                tool_name,
                result,
                is_error,
            } => {
                let _ = self.handle_tool_end(tool_call_id, tool_name, result, is_error);
            }
            AgentEvent::BackgroundWorkStarted { .. }
            | AgentEvent::BackgroundWorkProgress { .. }
            | AgentEvent::BackgroundWorkFinished { .. }
            | AgentEvent::BackgroundWorkCancelled { .. }
            | AgentEvent::WorkflowStarted { .. }
            | AgentEvent::WorkflowProgress { .. }
            | AgentEvent::WorkflowFinished { .. } => {}
        }
    }

    fn record_thinking_elapsed(&mut self, event: &AgentEvent) {
        if let AgentEvent::MessageUpdate {
            event: AssistantMessageEvent::ThinkingEnd { elapsed_ms, .. },
            ..
        } = event
        {
            self.thinking_elapsed_ms = *elapsed_ms;
        }
        if let AgentEvent::MessageEnd {
            message: runie_core::types::AgentMessage::Assistant(assistant),
        } = event
        {
            self.thinking_elapsed_ms = assistant.thinking_elapsed_ms;
        }
        if matches!(event, AgentEvent::AgentStart | AgentEvent::Reset) {
            self.thinking_elapsed_ms = None;
        }
    }

    fn handle_reset(&mut self) {
        if self.scrollback_actor.is_none() {
            self.scrollback.lock().clear();
        }
        if self.status_actor.is_none() {
            self.status.lock().set(Status::Ready);
        }
    }

    fn handle_agent_end(&mut self) {
        if self.turn_started && self.scrollback_actor.is_none() {
            let worked_for = self
                .status_actor
                .as_ref()
                .map(|actor| actor.snapshot().worked_for_label())
                .unwrap_or_else(|| self.status.lock().worked_for_label());
            let mut scrollback = self.scrollback.lock();
            scrollback.append(Line::new(LineKind::Separator, ""));
            scrollback.append(Line::new(LineKind::TurnSummary, worked_for));
        }
        self.turn_started = false;
        if self.status_actor.is_none() {
            self.status.lock().set(Status::Ready);
        }
    }

    #[allow(
        clippy::cognitive_complexity,
        clippy::too_many_lines,
        reason = "tool-start reduction keeps activity grouping and tool-row ownership together"
    )]
    fn handle_tool_start(
        &mut self,
        tool_call_id: String,
        tool_name: String,
        args: serde_json::Value,
    ) -> ScrollbackMsg {
        let starts_new_activity_group = self.active_tool_count == 0 && !self.activity_group_open;
        if starts_new_activity_group {
            self.activity_dirs = 0;
            self.activity_files = 0;
            self.activity_commands = 0;
            self.activity_subagents = 0;
            self.activity_failures = 0;
            self.activity_group_open = true;
        }
        self.in_tool_exec = true;
        if matches!(tool_name.as_str(), "list_dir" | "list_files") {
            self.activity_dirs += 1;
        } else if matches!(tool_name.as_str(), "read" | "read_file") {
            self.activity_files += 1;
        } else if matches!(tool_name.as_str(), "bash" | "shell" | "exec" | "run") {
            self.activity_commands += 1;
        } else if matches!(tool_name.as_str(), "subagent" | "agent" | "task") {
            self.activity_subagents += 1;
        }
        self.active_tool_count += 1;
        let tool_buffer = tool_header(&tool_name, &args);
        let activity = if self.activity_dirs
            + self.activity_files
            + self.activity_commands
            + self.activity_subagents
            > 0
        {
            Some(activity_text(
                self.activity_dirs,
                self.activity_files,
                self.activity_commands,
                self.activity_subagents,
                self.activity_failures,
                true,
            ))
        } else {
            None
        };
        if self.scrollback_actor.is_none() {
            if let Some(activity) = &activity {
                let mut scrollback = self.scrollback.lock();
                if !starts_new_activity_group {
                    if let Some(line) = scrollback.last_mut_by_kind(LineKind::Activity) {
                        line.text = activity.clone();
                    } else {
                        scrollback.append(Line::new(LineKind::Activity, activity.clone()));
                    }
                } else {
                    scrollback.append(Line::new(LineKind::Activity, activity.clone()));
                }
            }
        }
        if self.scrollback_actor.is_none() {
            let row = self.scrollback.lock().append(
                Line::new(LineKind::ToolRunning, tool_buffer.clone()).for_tool(&tool_call_id),
            );
            self.tool_rows.insert(tool_call_id.clone(), row);
        }
        self.tool_buffers
            .insert(tool_call_id.clone(), tool_buffer.clone());
        ScrollbackMsg::ToolStartRunning {
            tool_call_id,
            header: tool_buffer,
            activity,
        }
    }

    #[allow(clippy::too_many_lines, clippy::question_mark)]
    fn handle_tool_update(
        &mut self,
        tool_call_id: String,
        partial_result: serde_json::Value,
    ) -> Option<ScrollbackMsg> {
        // Grok treats transport-only lifecycle updates (for example
        // `{status: "running"}`) as block state, not transcript text. Do not
        // leak those envelopes into a specialized card header.
        if partial_result
            .get("status")
            .and_then(serde_json::Value::as_str)
            .is_some()
            && structured_update_text(&partial_result).is_none()
        {
            return None;
        }
        if self.in_tool_exec {
            if let Some(output) = structured_update_text(&partial_result) {
                let output_lines = output
                    .lines()
                    .filter(|line| !line.is_empty())
                    .map(str::to_owned)
                    .collect::<Vec<_>>();
                if self.scrollback_actor.is_none() {
                    for line in output.lines().filter(|line| !line.is_empty()) {
                        self.scrollback
                            .lock()
                            .append(Line::new(LineKind::ToolOutput, line).for_tool(&tool_call_id));
                    }
                }
                return Some(ScrollbackMsg::ToolUpdate {
                    tool_call_id,
                    header: None,
                    output: output_lines,
                });
            }
            let Some(buffer) = self.tool_buffers.get_mut(&tool_call_id) else {
                return None;
            };
            buffer.push_str(&format!(
                " | update: {}",
                serde_json::to_string(&partial_result).unwrap_or_default()
            ));
            let updated = buffer.clone();
            if self.scrollback_actor.is_none() {
                if let Some(row) = self.tool_rows.get(&tool_call_id).copied() {
                    self.replace_tool_line(row, &updated);
                }
            }
            if self.scrollback_actor.is_some() || self.tool_rows.contains_key(&tool_call_id) {
                return Some(ScrollbackMsg::ToolUpdate {
                    tool_call_id,
                    header: Some(updated),
                    output: Vec::new(),
                });
            }
        }
        None
    }

    #[allow(
        clippy::too_many_lines,
        reason = "tool completion reduction keeps card and activity ownership together"
    )]
    #[allow(clippy::cognitive_complexity)]
    fn handle_tool_end(
        &mut self,
        tool_call_id: String,
        tool_name: String,
        result: serde_json::Value,
        is_error: bool,
    ) -> ScrollbackMsg {
        self.in_tool_exec = false;
        self.active_tool_count = self.active_tool_count.saturating_sub(1);
        if is_error {
            self.activity_failures += 1;
        }
        let tool_buffer = self.tool_buffers.remove(&tool_call_id).unwrap_or_default();
        let tool_buffer = if is_error {
            format!("{tool_buffer} ✗")
        } else {
            completed_tool_header(&tool_buffer, &tool_name, &result)
        };
        if self.scrollback_actor.is_none() {
            if let Some(row) = self.tool_rows.remove(&tool_call_id) {
                self.settle_tool_line(row);
                self.replace_tool_line(row, &tool_buffer);
            }
        }
        let activity = if self.active_tool_count == 0
            && self.activity_dirs
                + self.activity_files
                + self.activity_commands
                + self.activity_subagents
                > 0
        {
            Some(activity_text(
                self.activity_dirs,
                self.activity_files,
                self.activity_commands,
                self.activity_subagents,
                self.activity_failures,
                false,
            ))
        } else {
            None
        };
        if self.scrollback_actor.is_none() {
            if let Some(activity) = &activity {
                if let Some(line) = self.scrollback.lock().last_mut_by_kind(LineKind::Activity) {
                    line.text = activity.clone();
                }
            }
        }
        let mut output = Vec::new();
        if !is_error {
            let kind = if matches!(
                tool_name.as_str(),
                "list_dir"
                    | "list_files"
                    | "read"
                    | "read_file"
                    | "web_fetch"
                    | "web-fetch"
                    | "fetch"
            ) {
                LineKind::ToolOutput
            } else {
                LineKind::ToolResult
            };
            for line in tool_result_text(&result)
                .lines()
                .filter(|line| !line.is_empty())
            {
                if self.scrollback_actor.is_none() {
                    self.scrollback
                        .lock()
                        .append(Line::new(kind, line).for_tool(&tool_call_id));
                }
                output.push((kind, line.to_owned()));
            }
            if matches!(tool_name.as_str(), "web_search" | "web-search") {
                if let Some(sources) = web_search_sources_line(&tool_result_text(&result)) {
                    if self.scrollback_actor.is_none() {
                        self.scrollback.lock().append(
                            Line::new(LineKind::ToolResult, &sources).for_tool(&tool_call_id),
                        );
                    }
                    output.push((LineKind::ToolResult, sources));
                }
            }
        }
        ScrollbackMsg::ToolEnd {
            tool_call_id,
            header: tool_buffer,
            activity,
            output,
        }
    }

    fn handle_agent_start(&mut self) {
        if self.scrollback_actor.is_none() {
            if self.emit_welcome {
                self.emit_welcome_modal();
                self.emit_welcome = false;
            } else {
                let mut scrollback = self.scrollback.lock();
                scrollback.append(Line::new(LineKind::Separator, ""));
                scrollback.append(Line::new(
                    LineKind::SessionStart,
                    "◆ session_start  [hooks: 1]",
                ));
                scrollback.append(Line::new(LineKind::Separator, ""));
            }
        } else {
            self.emit_welcome = false;
        }
        if self.status_actor.is_none() {
            self.status.lock().set(Status::Thinking);
        }
        self.streaming_buffer.clear();
        self.tool_rows.clear();
        self.tool_buffers.clear();
        self.in_assistant_stream = false;
        self.in_reasoning = false;
        self.reasoning_buffer.clear();
        self.in_tool_exec = false;
        self.activity_dirs = 0;
        self.activity_files = 0;
        self.activity_commands = 0;
        self.activity_subagents = 0;
        self.activity_failures = 0;
        self.active_tool_count = 0;
        self.activity_group_open = false;
        self.turn_started = false;
    }

    fn handle_message_start(&mut self, message: runie_core::types::AgentMessage) {
        use runie_core::types::AgentMessage;
        match message {
            AgentMessage::User(user) => {
                self.activity_group_open = false;
                if self.scrollback_actor.is_none() && user.timestamp >= LIVE_TIMESTAMP_SECONDS_MIN {
                    self.scrollback
                        .lock()
                        .set_prompt_timestamp(Some(format_clock_timestamp(user.timestamp)));
                }
                let text = user
                    .content
                    .iter()
                    .map(|content| match content {
                        runie_core::types::UserContent::Text { text } => text.as_str(),
                        runie_core::types::UserContent::Image { .. } => "[image]",
                    })
                    .collect::<Vec<_>>()
                    .join("");
                if self.scrollback_actor.is_none() {
                    self.scrollback
                        .lock()
                        .append(Line::new(LineKind::User, text).with_vpad(true));
                }
            }
            AgentMessage::Assistant(_) => {
                self.activity_group_open = false;
                self.in_assistant_stream = true;
                self.streaming_buffer.clear();
                self.reasoning_buffer.clear();
                if self.scrollback_actor.is_none() {
                    let mut scrollback = self.scrollback.lock();
                    scrollback.append(Line::new(LineKind::Separator, ""));
                    scrollback.append(Line::new(LineKind::ThinkingStatus, "◆ Thinking…"));
                    scrollback.append(Line::new(LineKind::Separator, ""));
                    scrollback.append(Line::new(LineKind::Assistant, String::new()));
                }
            }
            AgentMessage::ToolResult(_) | AgentMessage::Custom(_) => {}
        }
    }

    #[allow(clippy::cognitive_complexity)]
    fn handle_message_end(&mut self, message: runie_core::types::AgentMessage) {
        if let runie_core::types::AgentMessage::Assistant(assistant) = message {
            self.in_assistant_stream = false;
            self.in_reasoning = false;
            if self.scrollback_actor.is_none() {
                let mut scrollback = self.scrollback.lock();
                if !scrollback.reasoning_expanded() {
                    if let Some(reasoning) = scrollback.last_mut_by_kind(LineKind::Reasoning) {
                        reasoning.text = "Thought".into();
                    }
                }
                // Grok commits the provisional thinking indicator as a compact
                // session event in collapsed mode. Expanded mode keeps the
                // reasoning event as the authoritative body projection.
                if !self.reasoning_buffer.is_empty() && scrollback.reasoning_expanded() {
                    scrollback.remove_kind(LineKind::ThinkingStatus);
                } else if !self.reasoning_buffer.is_empty() {
                    if let Some(thinking) = scrollback.last_mut_by_kind(LineKind::ThinkingStatus) {
                        thinking.kind = LineKind::TurnSummary;
                        thinking.text = thinking_summary(self.thinking_elapsed_ms);
                    }
                    scrollback.remove_kind(LineKind::Reasoning);
                } else {
                    // Plain responses do not create a Grok Thinking block; the
                    // provisional working indicator is transient only.
                    scrollback.remove_kind(LineKind::ThinkingStatus);
                }
                drop(scrollback);
                self.replace_last_assistant_line(&self.streaming_buffer.clone());
            }
            if let Some(error) = assistant.error_message {
                if self.status_actor.is_none() {
                    self.status.lock().set(Status::Error(error.clone()));
                }
                if self.scrollback_actor.is_none() {
                    self.scrollback
                        .lock()
                        .append(Line::new(LineKind::System, format!("error: {error}")));
                }
            }
        }
    }

    #[allow(
        clippy::too_many_lines,
        reason = "message lifecycle keeps compatibility feed and actor status transition aligned"
    )]
    #[allow(clippy::cognitive_complexity)]
    fn handle_message_update(&mut self, event: AssistantMessageEvent) {
        match event {
            AssistantMessageEvent::TextDelta { delta, .. } if self.in_assistant_stream => {
                if self.status_actor.is_none() {
                    self.status.lock().set(Status::Streaming);
                }
                self.in_reasoning = false;
                self.streaming_buffer.push_str(&delta);
                if self.scrollback_actor.is_none() {
                    self.replace_last_assistant_line(&self.streaming_buffer.clone());
                }
            }
            AssistantMessageEvent::ThinkingDelta { delta, .. } if self.in_assistant_stream => {
                if self.status_actor.is_none() {
                    self.status.lock().set(Status::Thinking);
                }
                self.in_reasoning = true;
                self.reasoning_buffer.push_str(&delta);
                if self.scrollback_actor.is_none() {
                    self.replace_last_reasoning_line(&self.reasoning_buffer.clone());
                }
            }
            AssistantMessageEvent::Done {
                stop_reason, usage, ..
            } => {
                if self.status_actor.is_none() {
                    let mut status = self.status.lock();
                    status.finish_turn(usage, stop_reason);
                    status.set(Status::Ready);
                }
            }
            AssistantMessageEvent::Error { error, .. } => {
                if self.status_actor.is_none() {
                    self.status.lock().set(Status::Error(error.error_text()));
                }
                if self.scrollback_actor.is_none() {
                    self.scrollback.lock().append(Line::new(
                        LineKind::System,
                        format!("error: {}", error.error_text()),
                    ));
                }
            }
            AssistantMessageEvent::ToolCallDelta { .. }
            | AssistantMessageEvent::ToolCallStart { .. }
            | AssistantMessageEvent::ToolCallEnd { .. }
            | AssistantMessageEvent::Start { .. }
            | AssistantMessageEvent::TextStart { .. }
            | AssistantMessageEvent::TextEnd { .. }
            | AssistantMessageEvent::ThinkingStart { .. }
            | AssistantMessageEvent::ThinkingEnd { .. }
            | AssistantMessageEvent::TextDelta { .. }
            | AssistantMessageEvent::ThinkingDelta { .. } => {}
        }
    }

    fn replace_last_assistant_line(&self, text: &str) {
        let mut sb = self.scrollback.lock();
        if let Some(last) = sb.lines_mut_last_assistant() {
            last.text = text.to_string();
        } else {
            sb.append(Line::new(LineKind::Assistant, text.to_string()));
        }
    }

    fn replace_tool_line(&self, row: usize, text: &str) {
        let mut sb = self.scrollback.lock();
        if let Some(line) = sb.line_mut(row) {
            line.text = text.to_string();
        } else {
            sb.append(Line::new(LineKind::Tool, text.to_string()));
        }
    }

    fn settle_tool_line(&self, row: usize) {
        let mut sb = self.scrollback.lock();
        if let Some(line) = sb.line_mut(row) {
            line.kind = LineKind::Tool;
            line.settle_tool_row();
        }
    }

    fn replace_last_reasoning_line(&self, text: &str) {
        let mut sb = self.scrollback.lock();
        if let Some(last) = sb.last_mut_by_kind(LineKind::Reasoning) {
            last.text = text.to_string();
        } else {
            sb.append(Line::new(LineKind::Reasoning, text.to_string()));
        }
    }

    /// Emit the welcome-modal lines (matches grok-build's minimal-mode chrome).
    /// Called once on the first `AgentStart` to seed the transcript with the
    /// version/cwd/model block, the event-log entry, and the hint line.
    fn emit_welcome_modal(&mut self) {
        for line in welcome_modal_lines() {
            self.scrollback.lock().append(line);
        }
    }
}

fn format_clock_timestamp(timestamp: i64) -> String {
    let (hour24, minute) = local_clock_parts(timestamp).unwrap_or_else(|| {
        const SECONDS_PER_DAY: i64 = 86_400;
        const SECONDS_PER_HOUR: i64 = 3_600;
        const SECONDS_PER_MINUTE: i64 = 60;
        let seconds = timestamp.rem_euclid(SECONDS_PER_DAY);
        (
            seconds / SECONDS_PER_HOUR,
            (seconds % SECONDS_PER_HOUR) / SECONDS_PER_MINUTE,
        )
    });
    let hour12 = match hour24 % 12 {
        0 => 12,
        hour => hour,
    };
    let meridiem = if hour24 < 12 { "AM" } else { "PM" };
    format!("{hour12}:{minute:02} {meridiem}")
}

fn local_clock_parts(timestamp: i64) -> Option<(i64, i64)> {
    let raw = timestamp as libc::time_t;
    let mut local = std::mem::MaybeUninit::<libc::tm>::uninit();
    // SAFETY: `localtime_r` writes a complete `tm` into the valid pointer or
    // returns null. No global libc timezone state is exposed to the caller.
    let result = unsafe { libc::localtime_r(&raw, local.as_mut_ptr()) };
    if result.is_null() {
        return None;
    }
    // SAFETY: a non-null result means libc initialized the structure.
    let local = unsafe { local.assume_init() };
    Some((i64::from(local.tm_hour), i64::from(local.tm_min)))
}

#[allow(
    clippy::too_many_lines,
    reason = "the pure tool-header DSL keeps Grok's specialized card vocabulary together"
)]
pub(crate) fn tool_header(tool_name: &str, args: &serde_json::Value) -> String {
    match tool_name {
        "list_dir" | "list_files" => {
            let path = args
                .get("path")
                .and_then(serde_json::Value::as_str)
                .unwrap_or(".");
            format!("List {}", make_relative_path(path))
        }
        "read" | "read_file" => {
            let path = args
                .get("path")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            format!("Read {}", make_relative_path(path))
        }
        "edit" | "write" | "write_file" | "search_replace" => {
            let path = args
                .get("path")
                .or_else(|| args.get("file_path"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            format!("Edit {}", make_relative_path(path))
        }
        "search" | "grep" | "find" => {
            let pattern = args
                .get("pattern")
                .or_else(|| args.get("query"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            let path = args
                .get("path")
                .or_else(|| args.get("cwd"))
                .and_then(serde_json::Value::as_str);
            match path {
                Some(path) if !path.is_empty() => {
                    format!("Search {pattern:?} in {}", make_relative_path(path))
                }
                _ => format!("Search {pattern:?}"),
            }
        }
        "web_search" | "web-search" => {
            let query = args
                .get("query")
                .or_else(|| args.get("q"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            format!("Web Search {query}")
        }
        "web_fetch" | "web-fetch" | "fetch" => {
            let url = args
                .get("url")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            format!("Fetch {url}")
        }
        "search_tools" | "search-tools" => {
            let query = args
                .get("query")
                .or_else(|| args.get("pattern"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            format!("Search Tools {query}")
        }
        "memory_search" | "memory-search" => {
            let query = args
                .get("query")
                .or_else(|| args.get("q"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            format!("Memory Search {query}")
        }
        "todo" | "todo_write" | "todo-write" => {
            let title = args
                .get("title")
                .or_else(|| args.get("task"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("Update todos");
            format!("Todo {title}")
        }
        "workflow" | "run_workflow" | "run-workflow" => {
            let name = args
                .get("name")
                .or_else(|| args.get("workflow"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            format!("Workflow {name}")
        }
        "use" | "use_tool" | "use-tool" => {
            let name = args
                .get("tool")
                .or_else(|| args.get("name"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            format!("Use {name}")
        }
        "subagent" | "agent" | "task" => {
            let description = args
                .get("description")
                .or_else(|| args.get("task"))
                .or_else(|| args.get("prompt"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            format!("Subagent started: {description:?}")
        }
        "bash" | "shell" | "exec" | "run" => {
            let command = args
                .get("command")
                .or_else(|| args.get("cmd"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            format!("Run {command}")
        }
        _ => format!(
            "{tool_name} {}",
            serde_json::to_string(args).unwrap_or_default()
        ),
    }
}

/// Grok displays tool paths relative to the active workspace whenever the
/// provider sends an absolute path. Keep this pure at the renderer boundary
/// so replay fixtures remain independent of the host's username.
fn make_relative_path(path: &str) -> String {
    let Ok(cwd) = std::env::current_dir() else {
        return path.to_owned();
    };
    let cwd = cwd.to_string_lossy();
    let Some(relative) = path.strip_prefix(cwd.as_ref()) else {
        return path.to_owned();
    };
    let relative = relative.strip_prefix('/').unwrap_or(relative);
    if relative.is_empty() {
        ".".to_owned()
    } else {
        relative.to_owned()
    }
}

/// Grok keeps the tool card header semantic after completion: it adds the
/// result cardinality instead of an arrow/status suffix. This is also the
/// stable text used by collapsed and expanded block modes.
#[allow(
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    reason = "the pure completion-header DSL keeps Grok's cardinality variants together"
)]
pub(crate) fn completed_tool_header(
    pending_header: &str,
    tool_name: &str,
    result: &serde_json::Value,
) -> String {
    let output = tool_result_text(result);
    match tool_name {
        "list_dir" | "list_files" => {
            let entries = output
                .lines()
                .filter(|line| !line.trim().is_empty())
                .count();
            format!(
                "{pending_header} ({entries} entr{})",
                if entries == 1 { "y" } else { "ies" }
            )
        }
        "read" | "read_file" => {
            let lines = output.lines().count();
            format!("{pending_header} ({lines} lines)")
        }
        "search" | "grep" | "find" => {
            let matches = output
                .lines()
                .filter(|line| !line.trim().is_empty())
                .count();
            format!(
                "{pending_header} ({matches} match{})",
                if matches == 1 { "" } else { "es" }
            )
        }
        "web_search" | "web-search" => {
            let sites = web_search_site_count(&output);
            format!(
                "{pending_header} ({sites} site{})",
                if sites == 1 { "" } else { "s" }
            )
        }
        "search_tools" | "search-tools" => {
            let results = output
                .lines()
                .filter(|line| !line.trim().is_empty())
                .count();
            format!(
                "{pending_header} ({results} result{})",
                if results == 1 { "" } else { "s" }
            )
        }
        "memory_search" | "memory-search" => {
            let matches = output
                .lines()
                .filter(|line| !line.trim().is_empty())
                .count();
            format!(
                "{pending_header} ({matches} result{})",
                if matches == 1 { "" } else { "s" }
            )
        }
        "todo" | "todo_write" | "todo-write" => {
            let items = output
                .lines()
                .filter(|line| !line.trim().is_empty())
                .count();
            if items == 0 {
                pending_header.to_owned()
            } else {
                format!(
                    "{pending_header} ({items} item{})",
                    if items == 1 { "" } else { "s" }
                )
            }
        }
        "workflow" | "run_workflow" | "run-workflow" => pending_header
            .strip_prefix("Workflow ")
            .map(|name| format!("Workflow completed: {name}"))
            .unwrap_or_else(|| pending_header.to_owned()),
        "use" | "use_tool" | "use-tool" => pending_header
            .strip_prefix("Use ")
            .map(|name| format!("Used {name}"))
            .unwrap_or_else(|| pending_header.to_owned()),
        "subagent" | "agent" | "task" => pending_header
            .strip_prefix("Subagent started: ")
            .map(|description| format!("Subagent completed: {description}"))
            .unwrap_or_else(|| pending_header.to_owned()),
        "edit" | "write" | "write_file" | "search_replace" => {
            let edits = output
                .lines()
                .filter(|line| !line.trim().is_empty())
                .count();
            if edits > 0 {
                format!(
                    "{pending_header} ({edits} edit{})",
                    if edits == 1 { "" } else { "s" }
                )
            } else {
                pending_header.to_owned()
            }
        }
        _ => format!("{pending_header} → ✓"),
    }
}

pub(crate) fn tool_result_text(result: &serde_json::Value) -> String {
    result
        .as_str()
        .map(str::to_owned)
        .or_else(|| {
            result
                .get("content")
                .and_then(serde_json::Value::as_array)
                .and_then(|content| content.iter().find_map(|item| item.get("text")))
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
        .or_else(|| {
            result
                .get("output")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_else(|| serde_json::to_string(result).unwrap_or_default())
}

fn web_search_site_count(output: &str) -> usize {
    let mut domains = std::collections::HashSet::new();
    for token in output.split_whitespace() {
        let Some(url) = token
            .strip_prefix("https://")
            .or_else(|| token.strip_prefix("http://"))
        else {
            continue;
        };
        if let Some(domain) = url.split(['/', '?', '#', ')', ']', ',']).next() {
            if !domain.is_empty() {
                domains.insert(domain.to_ascii_lowercase());
            }
        }
    }
    if domains.is_empty() {
        output
            .lines()
            .filter(|line| !line.trim().is_empty())
            .count()
    } else {
        domains.len()
    }
}

fn web_search_sources_line(output: &str) -> Option<String> {
    let mut domains = Vec::new();
    for token in output.split_whitespace() {
        let Some(url) = token
            .strip_prefix("https://")
            .or_else(|| token.strip_prefix("http://"))
        else {
            continue;
        };
        let Some(domain) = url
            .split(['/', '?', '#', ')', ']', ','])
            .next()
            .filter(|domain| !domain.is_empty())
        else {
            continue;
        };
        if !domains.iter().any(|seen| seen == domain) {
            domains.push(domain.to_owned());
        }
    }
    if domains.is_empty() {
        return None;
    }
    let shown = domains
        .iter()
        .take(3)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    let remaining = domains.len().saturating_sub(3);
    Some(if remaining == 0 {
        format!("  Sources: {shown}")
    } else {
        format!("  Sources: {shown} (+{remaining} more)")
    })
}

fn structured_update_text(result: &serde_json::Value) -> Option<String> {
    result
        .get("output")
        .and_then(serde_json::Value::as_str)
        .or_else(|| result.get("content").and_then(serde_json::Value::as_str))
        .map(str::to_owned)
}

#[allow(
    clippy::cognitive_complexity,
    reason = "activity label projection keeps Grok's ordered vocabulary together"
)]
fn agent_start_messages(emit_welcome: bool) -> Vec<ScrollbackMsg> {
    if emit_welcome {
        return welcome_modal_lines()
            .into_iter()
            .map(ScrollbackMsg::Append)
            .collect();
    }
    vec![
        ScrollbackMsg::Append(Line::new(LineKind::Separator, "")),
        ScrollbackMsg::Append(Line::new(
            LineKind::SessionStart,
            "◆ session_start  [hooks: 1]",
        )),
        ScrollbackMsg::Append(Line::new(LineKind::Separator, "")),
    ]
}

#[allow(
    clippy::cognitive_complexity,
    reason = "activity vocabulary remains one pure Grok label projection"
)]
pub(crate) fn activity_text(
    dirs: usize,
    files: usize,
    commands: usize,
    subagents: usize,
    failures: usize,
    running: bool,
) -> String {
    let dir_verb = if running { "Listing" } else { "Listed" };
    let file_verb = if running { "Reading" } else { "Read" };
    let command_verb = if running { "Running" } else { "Ran" };
    let subagent_verb = if running { "Running" } else { "Ran" };
    let mut parts = Vec::new();
    if dirs > 0 {
        parts.push(format!(
            "{dir_verb} {dirs} dir{}",
            if dirs == 1 { "" } else { "s" }
        ));
    }
    if files > 0 {
        parts.push(format!(
            "{file_verb} {files} file{}",
            if files == 1 { "" } else { "s" }
        ));
    }
    if commands > 0 {
        parts.push(format!(
            "{command_verb} {commands} command{}",
            if commands == 1 { "" } else { "s" }
        ));
    }
    if subagents > 0 {
        parts.push(format!(
            "{subagent_verb} {subagents} subagent{}",
            if subagents == 1 { "" } else { "s" }
        ));
    }
    append_failure_suffix(format!("◈ {}", parts.join(", ")), failures, running)
}

fn append_failure_suffix(mut text: String, failures: usize, running: bool) -> String {
    if failures > 0 && !running {
        text.push_str(&format!(" · {failures} failed"));
    }
    text
}

/// Pure function: returns the welcome-modal lines (matches grok-build's
/// minimal-mode chrome). Adopts grok's `insta::assert_snapshot!` pattern:
/// the function is a pure formatter, the test pins its output to a snapshot.
pub fn welcome_modal_lines() -> Vec<Line> {
    let cwd = std::env::current_dir()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "runie".into());
    let branch = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout).ok()
            } else {
                None
            }
        })
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "main".into());
    let version = env!("CARGO_PKG_VERSION");
    vec![
        Line::new(LineKind::System, format!("╭─ Runie  v{version} ─")),
        Line::new(LineKind::System, format!("│ {branch} {cwd}")),
        Line::new(LineKind::System, String::from("│ Model · runie-core")),
        Line::new(LineKind::System, String::from("│ /help for commands")),
        Line::new(LineKind::System, String::from("╰─")),
        Line::new(LineKind::System, String::from("◆ session_start")),
    ]
}

// Extension methods on Scrollback for last-line replacement. Kept here to
// avoid touching the widget from the renderer module.
trait ScrollbackExt {
    fn lines_mut_last_assistant(&mut self) -> Option<&mut Line>;
}
impl ScrollbackExt for Scrollback {
    fn lines_mut_last_assistant(&mut self) -> Option<&mut Line> {
        self.last_mut_by_kind(LineKind::Assistant)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web_search_sources_projection_matches_grok_summary() {
        assert_eq!(
            web_search_sources_line(
                "https://docs.rs/runie https://docs.rs/ratatui https://rust-lang.org/learn https://github.com/runie https://docs.rs/extra"
            ),
            Some("  Sources: docs.rs, rust-lang.org, github.com".to_owned())
        );
        assert_eq!(web_search_sources_line("no citations"), None);
    }
    use runie_core::types::{AgentMessage, StopReason, ThemeKind, Usage, UserContent, UserMessage};

    fn new_renderer() -> (EventRenderer, Arc<Mutex<Scrollback>>, Arc<Mutex<StatusBar>>) {
        let sb = Arc::new(Mutex::new(Scrollback::new()));
        let st = Arc::new(Mutex::new(StatusBar::new()));
        (EventRenderer::new(sb.clone(), st.clone()), sb, st)
    }

    #[test]
    fn status_event_mapping_is_pure_and_ordered() {
        assert_eq!(
            status_messages_for_event(&AgentEvent::AgentStart),
            vec![StatusMsg::Set(Status::Thinking)]
        );
        let messages = status_messages_for_event(&AgentEvent::TurnStart);
        assert_eq!(
            messages,
            vec![StatusMsg::BeginTurn, StatusMsg::Set(Status::Thinking)]
        );
        assert_eq!(
            status_messages_for_event(&AgentEvent::Reset),
            vec![StatusMsg::Set(Status::Ready)]
        );
    }

    #[allow(
        clippy::too_many_lines,
        reason = "keeps the pure feed mapping table together"
    )]
    #[test]
    fn feed_event_mapping_is_pure_and_explicit() {
        let reset = scrollback_messages_for_event(&AgentEvent::Reset);
        assert_eq!(reset, vec![ScrollbackMsg::Clear]);
        let theme = scrollback_messages_for_event(&AgentEvent::ThemeChanged {
            theme: ThemeKind::GrokDay,
        });
        assert_eq!(theme, vec![ScrollbackMsg::SetTheme(ThemeKind::GrokDay)]);
        assert!(scrollback_messages_for_event(&AgentEvent::TurnStart).is_empty());
        assert_eq!(
            scrollback_messages_for_event(&AgentEvent::ToolExecutionStart {
                tool_call_id: "bash-1".into(),
                tool_name: "bash".into(),
                args: serde_json::json!({"command": "pwd"}),
            }),
            vec![
                ScrollbackMsg::SetToolName("bash-1".into(), "bash".into()),
                ScrollbackMsg::SetToolMode(
                    "bash-1".into(),
                    runie_core::types::ToolDisplayMode::Truncated,
                ),
            ]
        );
        let user = scrollback_messages_for_event(&AgentEvent::MessageStart {
            message: AgentMessage::User(UserMessage {
                content: vec![UserContent::Text {
                    text: "hello".into(),
                }],
                timestamp: 0,
            }),
        });
        assert!(matches!(
            user.as_slice(),
            [ScrollbackMsg::Append(line)]
                if line.kind == LineKind::User && line.text == "hello"
        ));
        assert_eq!(
            scrollback_messages_for_event(&AgentEvent::MessageStart {
                message: AgentMessage::Assistant(Default::default()),
            })
            .len(),
            4
        );
        let delta = scrollback_messages_for_event(&AgentEvent::MessageUpdate {
            event: AssistantMessageEvent::TextDelta {
                index: 0,
                delta: "world".into(),
                partial: runie_core::types::AssistantMessage::default(),
            },
            message: AgentMessage::Assistant(Default::default()),
        });
        assert_eq!(
            delta,
            vec![ScrollbackMsg::AppendToLastByKind(
                LineKind::Assistant,
                "world".into()
            )]
        );
    }

    #[tokio::test]
    async fn actor_renderer_reduces_events_without_legacy_projections() {
        let scrollback = ScrollbackActor::new();
        let status = StatusActor::new();
        let mut renderer = EventRenderer::with_actors(scrollback.clone(), status.clone(), false);

        renderer.apply_actor_event(AgentEvent::AgentStart).await;
        assert_eq!(status.snapshot().current(), &Status::Thinking);
        assert!(scrollback
            .snapshot()
            .find_first_containing("session_start")
            .is_some());

        renderer
            .apply_actor_event(AgentEvent::MessageStart {
                message: AgentMessage::User(UserMessage {
                    content: vec![UserContent::Text {
                        text: "hello".into(),
                    }],
                    timestamp: 0,
                }),
            })
            .await;
        assert!(scrollback
            .snapshot()
            .find_first_containing("hello")
            .is_some());
    }

    #[test]
    fn agent_start_emits_welcome_and_sets_thinking() {
        let (mut r, sb, st) = new_renderer();
        // Pre-seed a stale line. AgentStart now emits the welcome modal
        // instead of clearing (matches grok's minimal-mode chrome where
        // the welcome block persists across runs).
        r = EventRenderer::with_welcome(r.scrollback.legacy_arc(), r.status.legacy_arc(), true);
        r.apply_event(AgentEvent::AgentStart);
        assert!(
            !sb.lock().is_empty(),
            "welcome modal should populate scrollback"
        );
        assert!(sb.lock().find_first_containing("Runie").is_some());
        assert_eq!(st.lock().current(), &Status::Thinking);
    }

    #[test]
    fn message_update_appends_text_to_assistant_line() {
        let (mut r, sb, _) = new_renderer();
        r.apply_event(AgentEvent::AgentStart);
        r.apply_event(AgentEvent::MessageStart {
            message: AgentMessage::Assistant(runie_core::types::AssistantMessage {
                content: vec![],
                stop_reason: None,
                model: "t".into(),
                timestamp: 0,
                ..runie_core::types::AssistantMessage::default()
            }),
        });
        r.apply_event(AgentEvent::MessageUpdate {
            message: AgentMessage::Assistant(runie_core::types::AssistantMessage {
                content: vec![AssistantContent::Text { text: "hi".into() }],
                stop_reason: None,
                model: "t".into(),
                timestamp: 0,
                ..runie_core::types::AssistantMessage::default()
            }),
            event: AssistantMessageEvent::TextDelta {
                index: 0,
                delta: "Hello".into(),
                partial: runie_core::types::AssistantMessage::default(),
            },
        });
        let snap = sb.lock().find_first_containing("Hello").is_some();
        assert!(snap);
    }

    #[test]
    fn text_delta_enters_streaming_status() {
        let (mut r, _, st) = new_renderer();
        r.apply_event(AgentEvent::AgentStart);
        r.apply_event(AgentEvent::MessageStart {
            message: AgentMessage::Assistant(runie_core::types::AssistantMessage {
                content: vec![],
                stop_reason: None,
                model: "t".into(),
                timestamp: 0,
                ..runie_core::types::AssistantMessage::default()
            }),
        });
        r.apply_event(AgentEvent::MessageUpdate {
            message: AgentMessage::Assistant(runie_core::types::AssistantMessage {
                content: vec![],
                stop_reason: None,
                model: "t".into(),
                timestamp: 0,
                ..runie_core::types::AssistantMessage::default()
            }),
            event: AssistantMessageEvent::TextDelta {
                index: 0,
                delta: "partial".into(),
                partial: runie_core::types::AssistantMessage::default(),
            },
        });
        assert_eq!(st.lock().current(), &Status::Streaming);
    }

    #[test]
    fn agent_end_sets_ready() {
        let (mut r, sb, st) = new_renderer();
        r.apply_event(AgentEvent::AgentStart);
        r.apply_event(AgentEvent::AgentEnd { messages: vec![] });
        assert_eq!(st.lock().current(), &Status::Ready);
        assert!(sb.lock().find_first_containing("Worked for").is_none());
    }

    #[test]
    fn agent_end_emits_worked_for_only_after_turn_start() {
        let (mut r, sb, _) = new_renderer();
        r.apply_event(AgentEvent::AgentStart);
        r.apply_event(AgentEvent::TurnStart);
        r.apply_event(AgentEvent::AgentEnd { messages: vec![] });
        let scrollback = sb.lock();
        let summary = scrollback
            .lines()
            .iter()
            .find(|line| line.text == "Worked for 0.0s")
            .expect("completion summary");
        assert_eq!(summary.kind, LineKind::TurnSummary);
    }

    #[test]
    fn terminal_assistant_error_message_sets_error_status() {
        let (mut r, sb, st) = new_renderer();
        r.apply_event(AgentEvent::AgentStart);
        r.apply_event(AgentEvent::MessageStart {
            message: AgentMessage::Assistant(runie_core::types::AssistantMessage::default()),
        });
        r.apply_event(AgentEvent::MessageEnd {
            message: AgentMessage::Assistant(runie_core::types::AssistantMessage {
                stop_reason: Some(StopReason::Error),
                error_message: Some("api: unavailable".into()),
                ..Default::default()
            }),
        });
        assert_eq!(
            st.lock().current(),
            &Status::Error("api: unavailable".into())
        );
        assert!(sb
            .lock()
            .find_first_containing("error: api: unavailable")
            .is_some());
    }

    #[test]
    fn tool_execution_lifecycle() {
        let (mut r, sb, _) = new_renderer();
        r.apply_event(AgentEvent::AgentStart);
        r.apply_event(AgentEvent::ToolExecutionStart {
            tool_call_id: "1".into(),
            tool_name: "bash".into(),
            args: serde_json::json!({"cmd": "ls"}),
        });
        assert_eq!(
            sb.lock().tool_blocks()[0].mode,
            runie_core::types::ToolDisplayMode::Truncated
        );
        assert!(sb.lock().tool_blocks()[0].is_running);
        r.apply_event(AgentEvent::ToolExecutionEnd {
            tool_call_id: "1".into(),
            tool_name: "bash".into(),
            result: serde_json::json!({"ok": true}),
            is_error: false,
        });
        let lines = sb.lock();
        assert!(lines.find_first_containing("Run ls").is_some());
        assert!(lines.find_first_containing("✓").is_some());
        assert!(!lines.tool_blocks()[0].is_running);
        let _ = (
            StopReason::Stop,
            Usage::default(),
            UserContent::Text { text: "x".into() },
            UserMessage {
                content: vec![],
                timestamp: 0,
            },
        );
    }

    #[test]
    fn parallel_tool_updates_stay_on_their_own_rows() {
        let (mut renderer, scrollback, _) = new_renderer();
        renderer.apply_event(AgentEvent::ToolExecutionStart {
            tool_call_id: "a".into(),
            tool_name: "alpha".into(),
            args: serde_json::json!({}),
        });
        renderer.apply_event(AgentEvent::ToolExecutionStart {
            tool_call_id: "b".into(),
            tool_name: "beta".into(),
            args: serde_json::json!({}),
        });
        renderer.apply_event(AgentEvent::ToolExecutionUpdate {
            tool_call_id: "a".into(),
            tool_name: "alpha".into(),
            args: serde_json::json!({}),
            partial_result: serde_json::json!("a-update"),
        });
        renderer.apply_event(AgentEvent::ToolExecutionEnd {
            tool_call_id: "b".into(),
            tool_name: "beta".into(),
            result: serde_json::json!({}),
            is_error: false,
        });
        let lines = scrollback.lock();
        let alpha = lines.find_first_containing("alpha").expect("alpha row");
        let beta = lines.find_first_containing("beta").expect("beta row");
        assert!(lines.lines()[alpha].text.contains("a-update"));
        assert!(lines.lines()[beta].text.contains("✓"));
        assert!(!lines.lines()[beta].text.contains("a-update"));
    }

    #[test]
    #[allow(clippy::cognitive_complexity, clippy::too_many_lines)]
    fn structured_tools_use_grok_headers_and_preserve_output_rows() {
        assert_eq!(
            tool_header("list_dir", &serde_json::json!({"path":"."})),
            "List ."
        );
        assert_eq!(
            tool_header("read", &serde_json::json!({"path":"README.md"})),
            "Read README.md"
        );
        assert_eq!(
            tool_header("edit", &serde_json::json!({"path":"src/main.rs"})),
            "Edit src/main.rs"
        );
        assert_eq!(
            tool_header(
                "search",
                &serde_json::json!({"pattern":"TODO","path":"src"})
            ),
            "Search \"TODO\" in src"
        );
        assert_eq!(
            tool_header("web_search", &serde_json::json!({"query":"rust tui"})),
            "Web Search rust tui"
        );
        assert_eq!(
            tool_header(
                "web_fetch",
                &serde_json::json!({"url":"https://example.com"})
            ),
            "Fetch https://example.com"
        );
        assert_eq!(
            tool_header("memory_search", &serde_json::json!({"query":"actors"})),
            "Memory Search actors"
        );
        assert_eq!(
            tool_header("workflow", &serde_json::json!({"name":"release"})),
            "Workflow release"
        );
        assert_eq!(
            tool_header("use", &serde_json::json!({"tool":"browser"})),
            "Use browser"
        );
        assert_eq!(tool_result_text(&serde_json::json!("one\ntwo")), "one\ntwo");
        assert_eq!(
            tool_result_text(&serde_json::json!({"output":"one\ntwo"})),
            "one\ntwo"
        );
        assert_eq!(
            web_search_site_count(
                "https://docs.rs/a\nhttps://docs.rs/b\nhttps://rust-lang.org/learn"
            ),
            2
        );
        let (mut renderer, _, _) = new_renderer();
        let end = renderer.handle_tool_end(
            "fetch-1".into(),
            "web_fetch".into(),
            serde_json::json!("status: 200\nbody"),
            false,
        );
        let ScrollbackMsg::ToolEnd { output, .. } = end else {
            panic!("expected tool end projection");
        };
        assert!(output.iter().all(|(kind, _)| *kind == LineKind::ToolOutput));
    }

    #[test]
    fn absolute_tool_paths_are_workspace_relative() {
        let cwd = std::env::current_dir().expect("workspace cwd");
        let absolute = cwd.join("src/main.rs").to_string_lossy().into_owned();
        assert_eq!(
            tool_header("read", &serde_json::json!({"path": absolute})),
            "Read src/main.rs"
        );
        assert_eq!(make_relative_path(cwd.to_string_lossy().as_ref()), ".");
        assert_eq!(make_relative_path("/tmp/other/file"), "/tmp/other/file");
    }

    #[test]
    fn live_prompt_timestamp_uses_grok_short_clock_format() {
        for timestamp in [13 * 3_600 + 7 * 60, 0] {
            let formatted = format_clock_timestamp(timestamp);
            assert!(formatted.contains(':'), "{formatted}");
            assert!(formatted.ends_with(" AM") || formatted.ends_with(" PM"));
        }
    }

    #[test]
    fn completed_file_tools_use_grok_card_cardinality() {
        assert_eq!(
            completed_tool_header(
                "List .",
                "list_dir",
                &serde_json::json!("Cargo.toml\nsrc\ncrates")
            ),
            "List . (3 entries)"
        );
        assert_eq!(
            completed_tool_header("Read README.md", "read", &serde_json::json!("a\nb")),
            "Read README.md (2 lines)"
        );
        assert_eq!(
            completed_tool_header("Search \"TODO\"", "search", &serde_json::json!("a\nb")),
            "Search \"TODO\" (2 matches)"
        );
        assert_eq!(
            completed_tool_header("Edit src/main.rs", "edit", &serde_json::json!("hunk")),
            "Edit src/main.rs (1 edit)"
        );
        assert_eq!(
            completed_tool_header(
                "Memory Search actors",
                "memory_search",
                &serde_json::json!("one\ntwo"),
            ),
            "Memory Search actors (2 results)"
        );
        assert_eq!(
            completed_tool_header("Workflow release", "workflow", &serde_json::json!("done")),
            "Workflow completed: release"
        );
    }

    #[test]
    fn structured_tool_updates_append_indented_output_rows() {
        let (mut renderer, scrollback, _) = new_renderer();
        renderer.apply_event(AgentEvent::ToolExecutionStart {
            tool_call_id: "structured".into(),
            tool_name: "read".into(),
            args: serde_json::json!({"path":"README.md"}),
        });
        renderer.apply_event(AgentEvent::ToolExecutionUpdate {
            tool_call_id: "structured".into(),
            tool_name: "read".into(),
            args: serde_json::json!({}),
            partial_result: serde_json::json!({"output":"first\nsecond"}),
        });
        let lines = scrollback.lock();
        assert!(lines
            .lines()
            .iter()
            .any(|line| { line.kind == LineKind::ToolOutput && line.text == "first" }));
        assert!(lines
            .lines()
            .iter()
            .any(|line| { line.kind == LineKind::ToolOutput && line.text == "second" }));
    }

    #[test]
    fn activity_group_labels_match_grok_rich_recording() {
        assert_eq!(
            activity_text(1, 1, 0, 0, 0, true),
            "◈ Listing 1 dir, Reading 1 file"
        );
        assert_eq!(
            activity_text(1, 1, 0, 0, 0, false),
            "◈ Listed 1 dir, Read 1 file"
        );
        assert_eq!(activity_text(2, 0, 0, 0, 0, false), "◈ Listed 2 dirs");
        assert_eq!(
            activity_text(1, 0, 1, 0, 0, false),
            "◈ Listed 1 dir, Ran 1 command"
        );
        assert_eq!(
            activity_text(0, 0, 2, 0, 1, false),
            "◈ Ran 2 commands · 1 failed"
        );
        assert_eq!(
            activity_text(0, 1, 0, 1, 0, false),
            "◈ Read 1 file, Ran 1 subagent"
        );
    }

    #[test]
    fn activity_groups_do_not_merge_non_consecutive_tool_batches() {
        let (mut renderer, scrollback, _) = new_renderer();
        for (id, name) in [("first", "list_dir"), ("second", "read")] {
            if id == "second" {
                renderer.apply_event(AgentEvent::MessageStart {
                    message: AgentMessage::User(UserMessage {
                        content: vec![UserContent::Text {
                            text: "next".into(),
                        }],
                        timestamp: 0,
                    }),
                });
            }
            renderer.apply_event(AgentEvent::ToolExecutionStart {
                tool_call_id: id.into(),
                tool_name: name.into(),
                args: serde_json::json!({}),
            });
            renderer.apply_event(AgentEvent::ToolExecutionEnd {
                tool_call_id: id.into(),
                tool_name: name.into(),
                result: serde_json::json!({}),
                is_error: false,
            });
        }

        let rendered = scrollback.lock();
        let activities = rendered
            .lines()
            .iter()
            .filter(|line| line.kind == LineKind::Activity)
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>();
        assert_eq!(activities, ["◈ Listed 1 dir", "◈ Read 1 file"]);
    }

    /// Pure-function snapshot (adopted from grok-build's `insta` pattern).
    /// The welcome modal is a deterministic formatter; the test pins its
    /// text to a saved snapshot so accidental layout drift gets caught.
    #[test]
    fn welcome_modal_snapshot() {
        let text: String = super::welcome_modal_lines()
            .iter()
            .map(|l| l.text.clone())
            .collect::<Vec<_>>()
            .join("\n");
        insta::assert_snapshot!("welcome_modal", text);
    }
}
