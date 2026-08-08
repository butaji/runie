//! `EventRenderer` — subscribes to `runie-core`'s event bus and mutates widgets.

use std::time::Duration;

#[cfg(test)]
use runie_core::types::AssistantContent;
use runie_core::types::{AgentEvent, AssistantMessageEvent};
use tokio::sync::broadcast;

use crate::widgets::{Line, LineKind, ScrollbackMsg, StatusMsg};
use crate::{ScrollbackActor, StatusActor};

#[cfg(test)]
use crate::widgets::Status;

pub use runie_tui_model::activity_text;
pub use runie_tui_model::status_messages_for_event;
pub use runie_tui_model::thinking_summary;
use runie_tui_model::FeedSnapshot;

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
            let mut messages = vec![
                ScrollbackMsg::ActivityReset,
                ScrollbackMsg::Append(Line::new(LineKind::User, text).with_vpad(true)),
            ];
            if user.timestamp >= runie_tui_model::PROMPT_TIMESTAMP_LIVE_THRESHOLD {
                messages.push(ScrollbackMsg::SetPromptTimestamp(Some(
                    runie_tui_model::format_clock_timestamp(user.timestamp),
                )));
            }
            messages
        }
        AgentEvent::MessageStart {
            message: runie_core::types::AgentMessage::Assistant(_),
        } => vec![
            ScrollbackMsg::AssistantStreamStart,
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
        AgentEvent::MessageEnd {
            message: runie_core::types::AgentMessage::Assistant(_),
        } => vec![ScrollbackMsg::AssistantStreamEnd],
        AgentEvent::ThemeChanged { theme } => vec![ScrollbackMsg::SetTheme(*theme)],
        AgentEvent::ModelChanged { .. } => Vec::new(),
        AgentEvent::ToolDisplayModeChanged { tool_call_id, mode } => {
            vec![ScrollbackMsg::SetToolMode(tool_call_id.clone(), *mode)]
        }
        AgentEvent::ToolExecutionStart {
            tool_call_id,
            tool_name,
            args,
            ..
        } => vec![
            ScrollbackMsg::SetToolName(tool_call_id.clone(), tool_name.clone()),
            ScrollbackMsg::SetToolArgs(tool_call_id.clone(), args.clone()),
            ScrollbackMsg::ActivityToolStart(tool_name.clone()),
            ScrollbackMsg::SetToolMode(
                tool_call_id.clone(),
                runie_tui_model::default_tool_display_mode(tool_name),
            ),
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
                    runie_tui_model::format_elapsed(*elapsed_ms),
                    runie_tui_model::format_error(*is_error, error.as_deref())
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
                    runie_tui_model::format_elapsed(*elapsed_ms)
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
        AgentEvent::ToolExecutionEnd {
            tool_call_id,
            is_error,
            ..
        } => {
            vec![
                ScrollbackMsg::ActivityToolEnd {
                    is_error: *is_error,
                },
                ScrollbackMsg::RemoveToolArgs(tool_call_id.clone()),
            ]
        }
        AgentEvent::AgentStart
        | AgentEvent::AgentEnd { .. }
        | AgentEvent::Error { .. }
        | AgentEvent::ThinkingLevelChanged { .. }
        | AgentEvent::ActiveToolsChanged { .. }
        | AgentEvent::SessionLabelChanged { .. }
        | AgentEvent::SessionNameChanged { .. }
        | AgentEvent::SessionLaneChanged { .. }
        | AgentEvent::SessionEntryAppended { .. }
        | AgentEvent::BranchSummaryCreated { .. }
        | AgentEvent::CustomSessionEntryCreated { .. }
        | AgentEvent::CompactionCreated { .. }
        | AgentEvent::OperationRecordCreated { .. }
        | AgentEvent::TurnStart
        | AgentEvent::Waiting { .. }
        | AgentEvent::TurnEnd { .. }
        | AgentEvent::MessageStart { .. }
        | AgentEvent::MessageUpdate { .. }
        | AgentEvent::MessageEnd { .. }
        | AgentEvent::ToolExecutionUpdate { .. } => Vec::new(),
    }
}

pub struct EventRenderer {
    scrollback_actor: ScrollbackActor,
    status_actor: StatusActor,
    /// The live Grok surface places the thinking row directly after the user
    /// entry; deterministic replay keeps the historical four-row contract.
    live_grok_layout: bool,
}

impl EventRenderer {
    fn with_actors_inner(scrollback_actor: ScrollbackActor, status_actor: StatusActor) -> Self {
        Self {
            scrollback_actor,
            status_actor,
            live_grok_layout: false,
        }
    }

    /// Build the production renderer with its SSOT actors attached at
    /// construction time. The compatibility constructors remain for the
    /// synchronous YAML harness and focused reducer tests.
    pub fn with_actors(scrollback_actor: ScrollbackActor, status_actor: StatusActor) -> Self {
        Self::with_actors_inner(scrollback_actor, status_actor)
    }

    /// Production interactive projection with Grok's live assistant spacing.
    pub fn with_live_actors(scrollback_actor: ScrollbackActor, status_actor: StatusActor) -> Self {
        let mut renderer = Self::with_actors(scrollback_actor, status_actor);
        renderer.live_grok_layout = true;
        renderer
    }

    /// Drain bus events until the channel closes. Returns when receiver hits
    /// `RecvStreamLagged` or `Closed`.
    #[allow(
        clippy::cognitive_complexity,
        clippy::too_many_lines,
        reason = "event loop coordinates owned status/feed projections and shutdown from one atomic FeedSnapshot read per delivery"
    )]
    pub async fn run(
        mut self,
        mut rx: broadcast::Receiver<AgentEvent>,
        mut shutdown: tokio::sync::watch::Receiver<bool>,
    ) {
        let status_actor = self.status_actor.clone();
        let scrollback_actor = self.scrollback_actor.clone();
        const ANIMATION_TICK: Duration = Duration::from_millis(50);
        let mut tick = Box::pin(tokio::time::sleep(ANIMATION_TICK));
        loop {
            let animation_demand = status_actor.model_snapshot().animation_demand()
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
                            // EventRenderer is the single production bus
                            // delivery boundary. Actors are mailbox-only in
                            // App, so one core event produces one acknowledged
                            // status transition rather than racing a second
                            // bus-owned projection.
                            status_actor.apply_event(&event).await;
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
                            // One atomic `FeedSnapshot` read per bus delivery.
                            // Everything below this point is synchronous, so
                            // the turn flag and the assistant-finalize inputs
                            // observe the same scrollback generation instead
                            // of two independently torn reads.
                            let scrollback_snapshot = scrollback_actor.model_snapshot();
                            let turn_was_started = scrollback_snapshot.turn_started;
                            let mut feed_messages = scrollback_messages_for_event(&event);
                            if matches!(event, AgentEvent::TurnStart) {
                                feed_messages.push(ScrollbackMsg::TurnStart);
                            }
                            if self.live_grok_layout
                                && matches!(
                                    event,
                                    AgentEvent::MessageStart {
                                        message: runie_core::types::AgentMessage::Assistant(_),
                                    }
                                )
                                && matches!(feed_messages.get(1), Some(ScrollbackMsg::Append(line)) if line.kind == LineKind::Separator)
                            {
                                feed_messages.remove(1);
                            }
                            if matches!(event, AgentEvent::AgentStart) {
                                feed_messages.extend(session_start_messages());
                            }
                            if let AgentEvent::MessageEnd {
                                message: runie_core::types::AgentMessage::Assistant(_),
                            } = &event
                            {
                                let feed_snapshot = &scrollback_snapshot;
                                let has_reasoning = feed_snapshot.lines.iter().any(|line| {
                                    line.kind == LineKind::Reasoning && !line.text.is_empty()
                                });
                                let thinking_elapsed_ms = self.thinking_elapsed_ms();
                                feed_messages.push(ScrollbackMsg::FinalizeAssistant {
                                    has_reasoning,
                                    reasoning_expanded: feed_snapshot.reasoning_expanded,
                                    summary: thinking_summary(thinking_elapsed_ms),
                                    settled_no_tool_phase: thinking_elapsed_ms.is_some()
                                        && feed_snapshot.tool_blocks.is_empty(),
                                });
                            }
                            if matches!(event, AgentEvent::AgentEnd { .. }) && turn_was_started {
                                feed_messages.push(ScrollbackMsg::AppendTurnSummary(
                                    status_actor.model_snapshot().worked_for_label(),
                                ));
                                feed_messages.push(ScrollbackMsg::TurnEnd);
                            } else if matches!(event, AgentEvent::AgentEnd { .. }) {
                                feed_messages.push(ScrollbackMsg::TurnEnd);
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
                            // Coalesce the specialized tool message into the
                            // single actor mailbox hop for this event so the
                            // scrollback actor publishes one snapshot per bus
                            // delivery rather than racing two reductions.
                            // Message order is preserved by `scrollback_messages_for_event`:
                            // tool start emits `Set*` rows before
                            // `ToolStartRunning`, and tool end emits
                            // `ActivityToolEnd` / `RemoveToolArgs` before
                            // `ToolEnd`.
                            if let Some(tool_start) = actor_tool_start {
                                feed_messages.push(tool_start);
                            } else if let Some(tool_update) = actor_tool_update {
                                feed_messages.push(tool_update);
                            } else if let Some(tool_end) = actor_tool_end {
                                feed_messages.push(tool_end);
                            }
                            if !feed_messages.is_empty() {
                                scrollback_actor.apply_batch(feed_messages).await;
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
        reason = "actor replay keeps one event-to-projection transaction explicit on a single atomic FeedSnapshot read"
    )]
    pub async fn apply_actor_event(&mut self, event: AgentEvent) {
        let status_actor = self.status_actor.clone();
        let scrollback_actor = self.scrollback_actor.clone();
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
        // One atomic `FeedSnapshot` read per replayed event. The remainder of
        // this body is synchronous, so the turn flag and the
        // assistant-finalize inputs observe the same scrollback generation
        // instead of two independently torn reads.
        let scrollback_snapshot = scrollback_actor.model_snapshot();
        let turn_was_started = scrollback_snapshot.turn_started;
        let mut messages = scrollback_messages_for_event(&event);
        if matches!(event, AgentEvent::TurnStart) {
            messages.push(ScrollbackMsg::TurnStart);
        }
        if matches!(event, AgentEvent::AgentStart) {
            messages.extend(session_start_messages());
        }
        if let AgentEvent::MessageEnd {
            message: runie_core::types::AgentMessage::Assistant(_),
        } = &event
        {
            let feed_snapshot = &scrollback_snapshot;
            let has_reasoning = feed_snapshot
                .lines
                .iter()
                .any(|line| line.kind == LineKind::Reasoning && !line.text.is_empty());
            let thinking_elapsed_ms = self.thinking_elapsed_ms();
            messages.push(ScrollbackMsg::FinalizeAssistant {
                has_reasoning,
                reasoning_expanded: feed_snapshot.reasoning_expanded,
                summary: thinking_summary(thinking_elapsed_ms),
                settled_no_tool_phase: thinking_elapsed_ms.is_some()
                    && feed_snapshot.tool_blocks.is_empty(),
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
        if matches!(event, AgentEvent::AgentEnd { .. }) && turn_was_started {
            messages.push(ScrollbackMsg::AppendTurnSummary(
                status_actor.model_snapshot().worked_for_label(),
            ));
            messages.push(ScrollbackMsg::TurnEnd);
        } else if matches!(event, AgentEvent::AgentEnd { .. }) {
            messages.push(ScrollbackMsg::TurnEnd);
        }
        // Coalesce the specialized tool message into the single actor
        // mailbox hop for this event so the replay path mirrors the live
        // `run` path and the scrollback actor publishes one snapshot per
        // event. Message order matches the live path: tool start appends
        // `ToolStartRunning` after the `Set*` rows, tool end appends
        // `ToolEnd` after `ActivityToolEnd` / `RemoveToolArgs`.
        if let Some(message) = tool_message {
            messages.push(message);
        }
        if !messages.is_empty() {
            scrollback_actor.apply_batch(messages).await;
        }
    }

    async fn advance_animation(&self, actor: &StatusActor) {
        actor.apply(StatusMsg::AdvanceAnimation).await;
        self.scrollback_actor
            .apply(ScrollbackMsg::AdvanceAnimation)
            .await;
    }

    fn thinking_elapsed_ms(&self) -> Option<u64> {
        self.status_actor.model_snapshot().thinking_elapsed_ms
    }

    #[allow(
        clippy::cognitive_complexity,
        clippy::too_many_lines,
        reason = "single atomic FeedSnapshot read keeps activity-grouping and tool-row ownership together"
    )]
    fn handle_tool_start(
        &mut self,
        tool_call_id: String,
        tool_name: String,
        args: serde_json::Value,
    ) -> ScrollbackMsg {
        let snapshot = self.scrollback_actor.model_snapshot();
        let starts_new_activity_group = active_tool_count(&snapshot) == 0
            && !activity_group_exists_since_latest_user(&snapshot);
        let counts = activity_counts_with_start(&snapshot, &tool_name, starts_new_activity_group);
        let (
            activity_dirs,
            activity_files,
            activity_commands,
            activity_subagents,
            activity_failures,
        ) = counts;
        let tool_buffer = tool_header(&tool_name, &args);
        let activity =
            if activity_dirs + activity_files + activity_commands + activity_subagents > 0 {
                Some(activity_text(
                    activity_dirs,
                    activity_files,
                    activity_commands,
                    activity_subagents,
                    activity_failures,
                    true,
                ))
            } else {
                None
            };
        ScrollbackMsg::ToolStartRunning {
            tool_call_id,
            header: tool_buffer,
            activity,
        }
    }

    #[allow(
        clippy::too_many_lines,
        clippy::question_mark,
        reason = "single atomic FeedSnapshot read keeps the running-block check \
                  and the current-tool header consistent"
    )]
    fn handle_tool_update(
        &mut self,
        tool_call_id: String,
        partial_result: serde_json::Value,
    ) -> Option<ScrollbackMsg> {
        // Grok treats transport-only lifecycle updates (for example
        // `{status: "running"}`) as block state, not transcript text. Do not
        // leak those envelopes into a specialized card header.
        if runie_tui_model::is_transport_only_update(&partial_result) {
            return None;
        }
        let snapshot = self.scrollback_actor.model_snapshot();
        if snapshot
            .tool_blocks
            .iter()
            .any(|block| block.tool_call_id == tool_call_id && block.is_running)
        {
            if let Some(output) = runie_tui_model::structured_update_text(&partial_result) {
                let output_lines = structured_memory_lines(&output);
                return Some(ScrollbackMsg::ToolUpdate {
                    tool_call_id,
                    header: None,
                    output: output_lines,
                });
            }
            let Some(current_header) = current_tool_header(&snapshot, &tool_call_id) else {
                return None;
            };
            let updated =
                runie_tui_model::tool_update_header_text(&current_header, &partial_result);
            return Some(ScrollbackMsg::ToolUpdate {
                tool_call_id,
                header: Some(updated),
                output: Vec::new(),
            });
        }
        None
    }

    #[allow(
        clippy::too_many_lines,
        reason = "single atomic FeedSnapshot read keeps tool-end card, \
                  activity, and args projections consistent"
    )]
    #[allow(clippy::cognitive_complexity)]
    fn handle_tool_end(
        &mut self,
        tool_call_id: String,
        tool_name: String,
        result: serde_json::Value,
        is_error: bool,
    ) -> ScrollbackMsg {
        let snapshot = self.scrollback_actor.model_snapshot();
        let (
            activity_dirs,
            activity_files,
            activity_commands,
            activity_subagents,
            mut activity_failures,
        ) = activity_counts(&snapshot);
        if is_error {
            activity_failures += 1;
        }
        let tool_buffer = current_tool_header(&snapshot, &tool_call_id).unwrap_or_default();
        let tool_args = current_tool_args(&snapshot, &tool_call_id);
        let tool_buffer = if is_error {
            tool_buffer
        } else {
            completed_tool_header_with_args(&tool_buffer, &tool_name, &tool_args, &result)
        };
        let activity = if active_tool_count(&snapshot) <= 1
            && activity_dirs + activity_files + activity_commands + activity_subagents > 0
        {
            Some(activity_text(
                activity_dirs,
                activity_files,
                activity_commands,
                activity_subagents,
                activity_failures,
                false,
            ))
        } else {
            None
        };
        let mut output = Vec::new();
        {
            let raw_output = runie_tui_model::tool_result_text(&result);
            let kind = if is_error {
                LineKind::ToolError
            } else if runie_tui_model::is_output_tool(&tool_name) {
                LineKind::ToolOutput
            } else {
                LineKind::ToolResult
            };
            let rendered_lines: Vec<String> =
                if matches!(tool_name.as_str(), "memory_search" | "memory-search") {
                    runie_tui_model::memory_display_lines(&raw_output)
                } else {
                    raw_output.lines().map(str::to_owned).collect()
                };
            for line in rendered_lines.iter().filter(|line| !line.is_empty()) {
                output.push((kind, line.to_owned()));
            }
            if !is_error && matches!(tool_name.as_str(), "web_search" | "web-search") {
                if let Some(sources) =
                    web_search_sources_line(&runie_tui_model::tool_result_text(&result))
                {
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
}

fn current_tool_header(snapshot: &FeedSnapshot, tool_call_id: &str) -> Option<String> {
    snapshot
        .tool_blocks
        .iter()
        .rev()
        .find(|block| block.tool_call_id == tool_call_id && block.is_running)
        .map(|block| block.header.clone())
}

fn current_tool_args(snapshot: &FeedSnapshot, tool_call_id: &str) -> serde_json::Value {
    snapshot
        .tool_args
        .get(tool_call_id)
        .cloned()
        .unwrap_or(serde_json::Value::Null)
}

fn active_tool_count(snapshot: &FeedSnapshot) -> usize {
    snapshot
        .tool_blocks
        .iter()
        .filter(|block| block.is_running)
        .count()
}

fn activity_counts(snapshot: &FeedSnapshot) -> (usize, usize, usize, usize, usize) {
    (
        snapshot.activity_dirs,
        snapshot.activity_files,
        snapshot.activity_commands,
        snapshot.activity_subagents,
        snapshot.activity_failures,
    )
}

fn activity_group_exists_since_latest_user(snapshot: &FeedSnapshot) -> bool {
    let lines = &snapshot.lines;
    let latest_user = lines
        .iter()
        .rposition(|line| line.kind == LineKind::User)
        .unwrap_or(0);
    lines[latest_user..]
        .iter()
        .any(|line| line.kind == LineKind::Activity)
}

fn activity_counts_with_start(
    snapshot: &FeedSnapshot,
    tool_name: &str,
    reset: bool,
) -> (usize, usize, usize, usize, usize) {
    let (mut dirs, mut files, mut commands, mut subagents, failures) = if reset {
        (0, 0, 0, 0, 0)
    } else {
        activity_counts(snapshot)
    };
    match runie_tui_model::classify_activity_tool(tool_name) {
        Some(runie_tui_model::ActivityKind::Dir) => dirs += 1,
        Some(runie_tui_model::ActivityKind::File) => files += 1,
        Some(runie_tui_model::ActivityKind::Command) => commands += 1,
        Some(runie_tui_model::ActivityKind::Subagent) => subagents += 1,
        None => {}
    }
    (dirs, files, commands, subagents, failures)
}

#[allow(
    clippy::too_many_lines,
    reason = "the pure tool-header DSL keeps Grok's specialized card vocabulary together"
)]
pub(crate) fn tool_header(tool_name: &str, args: &serde_json::Value) -> String {
    match tool_name {
        "list_dir" | "list_files" | "ls" => {
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
        "edit" | "write" | "write_file" | "search_replace" | "apply_patch" | "strreplace" => {
            let path = args
                .get("path")
                .or_else(|| args.get("file_path"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            format!("Edit {}", make_relative_path(path))
        }
        "search" | "grep" | "find" | "glob" => {
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
        "search_tools" | "search-tools" | "search_tool" => {
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
        "bash"
        | "shell"
        | "exec"
        | "run"
        | "execute"
        | "run_terminal_command"
        | "run_terminal_cmd" => {
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
    let output = runie_tui_model::tool_result_text(result);
    match tool_name {
        "list_dir" | "list_files" | "ls" => {
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
        "search" | "grep" | "find" | "glob" => {
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
        "search_tools" | "search-tools" | "search_tool" => {
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
            let matches = runie_tui_model::parse_memory_results(&output).len();
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

/// Add Grok's source-backed Read range suffix while retaining the generic
/// completion-header API for callers that do not have tool arguments.
pub(crate) fn completed_tool_header_with_args(
    pending_header: &str,
    tool_name: &str,
    args: &serde_json::Value,
    result: &serde_json::Value,
) -> String {
    if matches!(tool_name, "read" | "read_file") {
        if result
            .get("content")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|content| {
                content.iter().any(|item| {
                    item.get("type") == Some(&serde_json::Value::String("image".into()))
                })
            })
        {
            return format!("{pending_header} (image)");
        }
        let Some(offset) = args.get("offset").and_then(serde_json::Value::as_u64) else {
            return completed_tool_header(pending_header, tool_name, result);
        };
        let output = runie_tui_model::tool_result_text(result);
        let content_lines = output
            .lines()
            .take_while(|line| !line.starts_with('['))
            .count() as u64;
        let end = offset.saturating_add(content_lines.max(1));
        let total = result
            .get("details")
            .and_then(|details| details.get("truncation"))
            .and_then(|truncation| truncation.get("totalLines"))
            .and_then(serde_json::Value::as_u64)
            .or_else(|| {
                output.lines().find_map(|line| {
                    line.split(" of ")
                        .nth(1)
                        .and_then(|part| part.split(|c: char| !c.is_ascii_digit()).next())
                        .and_then(|value| value.parse().ok())
                })
            });
        return match total {
            Some(total) => format!("{pending_header} ({}-{} of {total})", offset + 1, end),
            None => format!("{pending_header} ({}-{end})", offset + 1),
        };
    }
    completed_tool_header(pending_header, tool_name, result)
}

fn structured_memory_lines(output: &str) -> Vec<String> {
    runie_tui_model::memory_display_lines(output)
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

#[allow(
    clippy::cognitive_complexity,
    reason = "activity label projection keeps Grok's ordered vocabulary together"
)]
fn session_start_messages() -> Vec<ScrollbackMsg> {
    vec![
        ScrollbackMsg::Append(Line::new(LineKind::Separator, "")),
        ScrollbackMsg::Append(Line::new(
            LineKind::SessionStart,
            "◆ session_start  [hooks: 1]",
        )),
        ScrollbackMsg::Append(Line::new(LineKind::Separator, "")),
    ]
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
            vec![StatusMsg::Reset]
        );
    }

    #[allow(
        clippy::too_many_lines,
        reason = "keeps the pure feed mapping table together"
    )]
    #[test]
    #[allow(clippy::cognitive_complexity)]
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
                ScrollbackMsg::SetToolArgs("bash-1".into(), serde_json::json!({"command": "pwd"}),),
                ScrollbackMsg::ActivityToolStart("bash".into()),
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
            [ScrollbackMsg::ActivityReset, ScrollbackMsg::Append(line)]
                if line.kind == LineKind::User && line.text == "hello"
        ));
        assert_eq!(
            scrollback_messages_for_event(&AgentEvent::MessageStart {
                message: AgentMessage::Assistant(Default::default()),
            })
            .len(),
            5
        );
        assert_eq!(
            scrollback_messages_for_event(&AgentEvent::MessageEnd {
                message: AgentMessage::Assistant(Default::default()),
            }),
            vec![ScrollbackMsg::AssistantStreamEnd]
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
    async fn live_and_replay_assistant_start_preserve_layout_parity_contract() {
        let assistant_start = || AgentEvent::MessageStart {
            message: AgentMessage::Assistant(Default::default()),
        };

        let live_messages = {
            let mut messages = scrollback_messages_for_event(&assistant_start());
            messages.remove(1);
            messages
        };
        let replay_messages = scrollback_messages_for_event(&assistant_start());
        assert_eq!(live_messages.len(), 4);
        assert_eq!(replay_messages.len(), 5);

        let live_bus = runie_core::events::EventBus::new();
        let live_scrollback = ScrollbackActor::new();
        let live_status = StatusActor::new();
        let mut live_feed = live_scrollback.subscribe();
        let live_renderer = EventRenderer::with_live_actors(live_scrollback.clone(), live_status);
        let (live_shutdown_tx, live_shutdown_rx) = tokio::sync::watch::channel(false);
        // OWNER: test — joins the renderer after the shutdown event.
        let live_task = tokio::spawn(live_renderer.run(live_bus.subscribe(), live_shutdown_rx));

        live_bus.publish(assistant_start());
        live_feed
            .changed()
            .await
            .expect("live assistant start delivery");
        assert_eq!(live_scrollback.model_snapshot().lines.len(), 3);

        live_shutdown_tx.send(true).expect("live renderer shutdown");
        live_task.await.expect("live renderer task");

        let replay_scrollback = ScrollbackActor::new();
        let replay_status = StatusActor::new();
        let mut replay_renderer =
            EventRenderer::with_actors(replay_scrollback.clone(), replay_status);
        replay_renderer.apply_actor_event(assistant_start()).await;
        assert_eq!(replay_scrollback.model_snapshot().lines.len(), 4);
    }
    #[tokio::test]
    async fn actor_renderer_reduces_events_without_legacy_projections() {
        let scrollback = ScrollbackActor::new();
        let status = StatusActor::new();
        let mut renderer = EventRenderer::with_actors(scrollback.clone(), status.clone());

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

    #[tokio::test]
    async fn live_renderer_is_the_single_status_event_delivery_boundary() {
        let bus = runie_core::events::EventBus::new();
        let status = StatusActor::new();
        let scrollback = ScrollbackActor::new();
        let renderer = EventRenderer::with_actors(scrollback, status.clone());
        let mut status_updates = status.subscribe();
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        // OWNER: test — joins the renderer after the shutdown event.
        let task = tokio::spawn(renderer.run(bus.subscribe(), shutdown_rx));

        bus.publish(AgentEvent::AgentStart);
        status_updates
            .changed()
            .await
            .expect("renderer status delivery");
        assert_eq!(status.snapshot().current(), &Status::Thinking);

        shutdown_tx.send(true).expect("renderer shutdown");
        task.await.expect("renderer task");
    }

    #[tokio::test]
    #[allow(
        clippy::too_many_lines,
        reason = "end-to-end regression keeps tool start/update/end delivery in one assertion block"
    )]
    async fn live_renderer_delivers_tool_updates_to_the_feed_actor() {
        let bus = runie_core::events::EventBus::new();
        let status = StatusActor::new();
        let scrollback = ScrollbackActor::new();
        let mut feed = scrollback.subscribe();
        let renderer = EventRenderer::with_live_actors(scrollback.clone(), status);
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        // OWNER: test — joins the renderer after the shutdown event.
        let task = tokio::spawn(renderer.run(bus.subscribe(), shutdown_rx));

        bus.publish(AgentEvent::ToolExecutionStart {
            tool_call_id: "tool-1".into(),
            tool_name: "read".into(),
            args: serde_json::json!({"path":"src/lib.rs"}),
        });
        feed.changed().await.expect("tool start delivery");
        assert!(scrollback
            .model_snapshot()
            .tool_blocks
            .iter()
            .any(|block| block.tool_call_id == "tool-1" && block.is_running));

        bus.publish(AgentEvent::ToolExecutionUpdate {
            tool_call_id: "tool-1".into(),
            tool_name: "read".into(),
            args: serde_json::json!({"path":"src/lib.rs"}),
            partial_result: serde_json::json!({"output":"hello"}),
        });
        feed.changed().await.expect("tool update delivery");
        assert!(scrollback
            .model_snapshot()
            .tool_blocks
            .iter()
            .any(|block| block.tool_call_id == "tool-1"
                && block.output.iter().any(|row| row == "hello")));

        bus.publish(AgentEvent::ToolExecutionEnd {
            tool_call_id: "tool-1".into(),
            tool_name: "read".into(),
            result: serde_json::json!({"output":"hello"}),
            is_error: false,
        });
        feed.changed().await.expect("tool end delivery");
        assert!(
            scrollback
                .model_snapshot()
                .tool_blocks
                .iter()
                .any(|block| block.tool_call_id == "tool-1" && !block.is_running),
            "ToolExecutionEnd must mark the block as completed in a single actor hop"
        );

        shutdown_tx.send(true).expect("renderer shutdown");
        task.await.expect("renderer task");
    }

    #[allow(
        clippy::too_many_lines,
        reason = "live BackgroundWork* regression keeps all four lifecycle variants explicit"
    )]
    #[tokio::test]
    async fn live_renderer_delivers_background_work_lifecycle_to_the_feed_actor() {
        // Regression: the live `run` path used to drop every BackgroundWork*
        // event so the scrollback actor never produced the `Subagent`
        // started/running/completed/failed/cancelled rows. The replay path
        // (`apply_actor_event`) drove `scrollback_messages_for_event` directly,
        // so the two paths drifted apart. After dropping the filter, the live
        // bus loop must produce exactly the same rows the replay path emits.
        let snapshot_lines = |scrollback: &ScrollbackActor, work_id: &str| {
            let snapshot = scrollback.snapshot();
            snapshot
                .lines()
                .iter()
                .filter(|line| line.tool_call_id.as_deref() == Some(work_id))
                .map(|line| (line.kind, line.text.clone()))
                .collect::<Vec<_>>()
        };

        // 1. BackgroundWorkStarted — must seed the `Subagent started: …`
        //    row that the replay path emits for `background: true`.
        let started_bus = runie_core::events::EventBus::new();
        let started_scrollback = ScrollbackActor::new();
        let mut started_feed = started_scrollback.subscribe();
        let started_renderer =
            EventRenderer::with_live_actors(started_scrollback.clone(), StatusActor::new());
        let (started_shutdown, started_shutdown_rx) = tokio::sync::watch::channel(false);
        // OWNER: test — joins the renderer after the shutdown event.
        let started_task =
            tokio::spawn(started_renderer.run(started_bus.subscribe(), started_shutdown_rx));
        started_bus.publish(AgentEvent::BackgroundWorkStarted {
            work_id: "worker-started".into(),
            description: "inspect files".into(),
            background: true,
        });
        started_feed
            .changed()
            .await
            .expect("background start delivery");
        let started_lines = snapshot_lines(&started_scrollback, "worker-started");
        assert!(
            started_lines
                .iter()
                .any(|(kind, text)| matches!(kind, LineKind::Tool)
                    && text.contains("Subagent started")
                    && text.contains("inspect files")),
            "live `run` path must produce the `Subagent started: …` row: {started_lines:?}"
        );
        // And the row kind/kind must mirror what the replay path emits:
        // `scrollback_messages_for_event(BackgroundWorkStarted { background: true })`
        // is a single `ToolStart` whose header starts with `Subagent started`.
        assert_eq!(
            started_lines.len(),
            1,
            "started branch should emit exactly one transcript row, got: {started_lines:?}"
        );
        started_shutdown.send(true).expect("renderer shutdown");
        started_task.await.expect("renderer task");

        // 2. BackgroundWorkProgress — must append the `Subagent running: …`
        //    header update that the replay path emits.
        let progress_bus = runie_core::events::EventBus::new();
        let progress_scrollback = ScrollbackActor::new();
        let mut progress_feed = progress_scrollback.subscribe();
        let progress_renderer =
            EventRenderer::with_live_actors(progress_scrollback.clone(), StatusActor::new());
        let (progress_shutdown, progress_shutdown_rx) = tokio::sync::watch::channel(false);
        // OWNER: test — joins the renderer after the shutdown event.
        let progress_task =
            tokio::spawn(progress_renderer.run(progress_bus.subscribe(), progress_shutdown_rx));
        progress_bus.publish(AgentEvent::BackgroundWorkStarted {
            work_id: "worker-progress".into(),
            description: "list docs".into(),
            background: false,
        });
        progress_feed
            .changed()
            .await
            .expect("background start delivery");
        progress_bus.publish(AgentEvent::BackgroundWorkProgress {
            work_id: "worker-progress".into(),
            description: "list docs".into(),
            activity: "scanning".into(),
        });
        progress_feed
            .changed()
            .await
            .expect("background progress delivery");
        let progress_lines = snapshot_lines(&progress_scrollback, "worker-progress");
        assert!(
            progress_lines
                .iter()
                .any(|(_, text)| text.contains("Subagent running")
                    && text.contains("scanning")),
            "live `run` path must produce the `Subagent running: … — scanning` row: {progress_lines:?}"
        );
        // The replay path replaces the existing header row in place, so the
        // transcript still contains a single line tied to `worker-progress`.
        assert_eq!(
            progress_lines.len(),
            1,
            "progress branch should keep exactly one transcript row: {progress_lines:?}"
        );
        progress_shutdown.send(true).expect("renderer shutdown");
        progress_task.await.expect("renderer task");

        // 3. BackgroundWorkFinished (success) — must emit the
        //    `Subagent completed …` closure header.
        let finished_bus = runie_core::events::EventBus::new();
        let finished_scrollback = ScrollbackActor::new();
        let mut finished_feed = finished_scrollback.subscribe();
        let finished_renderer =
            EventRenderer::with_live_actors(finished_scrollback.clone(), StatusActor::new());
        let (finished_shutdown, finished_shutdown_rx) = tokio::sync::watch::channel(false);
        // OWNER: test — joins the renderer after the shutdown event.
        let finished_task =
            tokio::spawn(finished_renderer.run(finished_bus.subscribe(), finished_shutdown_rx));
        finished_bus.publish(AgentEvent::BackgroundWorkStarted {
            work_id: "worker-finished".into(),
            description: "compile".into(),
            background: true,
        });
        finished_feed
            .changed()
            .await
            .expect("background start delivery");
        finished_bus.publish(AgentEvent::BackgroundWorkFinished {
            work_id: "worker-finished".into(),
            description: "compile".into(),
            is_error: false,
            elapsed_ms: Some(1_250),
            error: None,
        });
        finished_feed
            .changed()
            .await
            .expect("background finished delivery");
        let finished_lines = snapshot_lines(&finished_scrollback, "worker-finished");
        assert!(
            finished_lines
                .iter()
                .any(|(_, text)| text.contains("Subagent completed") && text.contains("compile")),
            "live `run` path must produce the `Subagent completed …` row: {finished_lines:?}"
        );
        assert!(
            finished_lines
                .iter()
                .all(|(_, text)| !text.contains("Subagent failed")),
            "successful BackgroundWorkFinished must not produce a `Subagent failed` row: {finished_lines:?}"
        );
        assert_eq!(
            finished_lines.len(),
            1,
            "finished success branch should keep exactly one transcript row: {finished_lines:?}"
        );
        finished_shutdown.send(true).expect("renderer shutdown");
        finished_task.await.expect("renderer task");

        // 4. BackgroundWorkFinished (failure) — must emit the
        //    `Subagent failed …` closure header.
        let failed_bus = runie_core::events::EventBus::new();
        let failed_scrollback = ScrollbackActor::new();
        let mut failed_feed = failed_scrollback.subscribe();
        let failed_renderer =
            EventRenderer::with_live_actors(failed_scrollback.clone(), StatusActor::new());
        let (failed_shutdown, failed_shutdown_rx) = tokio::sync::watch::channel(false);
        // OWNER: test — joins the renderer after the shutdown event.
        let failed_task =
            tokio::spawn(failed_renderer.run(failed_bus.subscribe(), failed_shutdown_rx));
        failed_bus.publish(AgentEvent::BackgroundWorkStarted {
            work_id: "worker-failed".into(),
            description: "compile".into(),
            background: true,
        });
        failed_feed
            .changed()
            .await
            .expect("background start delivery");
        failed_bus.publish(AgentEvent::BackgroundWorkFinished {
            work_id: "worker-failed".into(),
            description: "compile".into(),
            is_error: true,
            elapsed_ms: Some(900),
            error: Some("boom".to_string()),
        });
        failed_feed
            .changed()
            .await
            .expect("background finished delivery");
        let failed_lines = snapshot_lines(&failed_scrollback, "worker-failed");
        assert!(
            failed_lines
                .iter()
                .any(|(_, text)| text.contains("Subagent failed") && text.contains("compile")),
            "live `run` path must produce the `Subagent failed …` row: {failed_lines:?}"
        );
        assert!(
            failed_lines
                .iter()
                .any(|(kind, _)| matches!(kind, LineKind::ToolError)),
            "failed BackgroundWorkFinished must mark the tool row as ToolError: {failed_lines:?}"
        );
        failed_shutdown.send(true).expect("renderer shutdown");
        failed_task.await.expect("renderer task");

        // 5. BackgroundWorkCancelled — must emit the
        //    `Subagent cancelled …` closure header and mark the row as
        //    `ToolError`, matching the replay path.
        let cancelled_bus = runie_core::events::EventBus::new();
        let cancelled_scrollback = ScrollbackActor::new();
        let mut cancelled_feed = cancelled_scrollback.subscribe();
        let cancelled_renderer =
            EventRenderer::with_live_actors(cancelled_scrollback.clone(), StatusActor::new());
        let (cancelled_shutdown, cancelled_shutdown_rx) = tokio::sync::watch::channel(false);
        // OWNER: test — joins the renderer after the shutdown event.
        let cancelled_task =
            tokio::spawn(cancelled_renderer.run(cancelled_bus.subscribe(), cancelled_shutdown_rx));
        cancelled_bus.publish(AgentEvent::BackgroundWorkStarted {
            work_id: "worker-cancelled".into(),
            description: "compile".into(),
            background: false,
        });
        cancelled_feed
            .changed()
            .await
            .expect("background start delivery");
        cancelled_bus.publish(AgentEvent::BackgroundWorkCancelled {
            work_id: "worker-cancelled".into(),
            description: "compile".into(),
            elapsed_ms: Some(250),
        });
        cancelled_feed
            .changed()
            .await
            .expect("background cancelled delivery");
        let cancelled_lines = snapshot_lines(&cancelled_scrollback, "worker-cancelled");
        assert!(
            cancelled_lines
                .iter()
                .any(|(_, text)| text.contains("Subagent cancelled") && text.contains("compile")),
            "live `run` path must produce the `Subagent cancelled …` row: {cancelled_lines:?}"
        );
        assert!(
            cancelled_lines
                .iter()
                .any(|(kind, _)| matches!(kind, LineKind::ToolError)),
            "BackgroundWorkCancelled must mark the tool row as ToolError: {cancelled_lines:?}"
        );
        cancelled_shutdown.send(true).expect("renderer shutdown");
        cancelled_task.await.expect("renderer task");
    }

    /// The replay path (`apply_actor_event`) and the live `run` path must
    /// project the same `Subagent …` rows for BackgroundWork* events. This
    /// pins the parity by driving one of each event through both paths and
    /// asserting the per-`work_id` transcript lines match.
    #[allow(
        clippy::too_many_lines,
        reason = "paired live/replay BackgroundWork parity keeps every variant explicit"
    )]
    #[tokio::test]
    async fn live_and_replay_background_work_paths_produce_identical_rows() {
        let events = vec![
            AgentEvent::BackgroundWorkStarted {
                work_id: "parity-started".into(),
                description: "started task".into(),
                background: true,
            },
            AgentEvent::BackgroundWorkStarted {
                work_id: "parity-foreground".into(),
                description: "running task".into(),
                background: false,
            },
            AgentEvent::BackgroundWorkProgress {
                work_id: "parity-foreground".into(),
                description: "running task".into(),
                activity: "scanning".into(),
            },
            AgentEvent::BackgroundWorkFinished {
                work_id: "parity-started".into(),
                description: "started task".into(),
                is_error: false,
                elapsed_ms: Some(500),
                error: None,
            },
            AgentEvent::BackgroundWorkFinished {
                work_id: "parity-failed".into(),
                description: "failed task".into(),
                is_error: true,
                elapsed_ms: Some(700),
                error: Some("oops".to_string()),
            },
            AgentEvent::BackgroundWorkCancelled {
                work_id: "parity-cancelled".into(),
                description: "cancelled task".into(),
                elapsed_ms: Some(100),
            },
        ];

        // Live path — drive every event through a `run` loop and snapshot
        // the per-`work_id` transcript rows.
        let live_bus = runie_core::events::EventBus::new();
        let live_scrollback = ScrollbackActor::new();
        let live_renderer =
            EventRenderer::with_live_actors(live_scrollback.clone(), StatusActor::new());
        let (live_shutdown, live_shutdown_rx) = tokio::sync::watch::channel(false);
        // OWNER: test — joins the renderer after the shutdown event.
        let live_task = tokio::spawn(live_renderer.run(live_bus.subscribe(), live_shutdown_rx));
        for event in &events {
            live_bus.publish(event.clone());
        }
        // Wait until the scrollback has reduced every event. The actor
        // publishes a snapshot after each batch, so reading the snapshot
        // once the bus has been drained is sufficient — no sleeps.
        for _ in 0..events.len() {
            live_scrollback
                .subscribe()
                .changed()
                .await
                .expect("scrollback actor alive");
        }
        live_shutdown.send(true).expect("renderer shutdown");
        live_task.await.expect("renderer task");

        // Replay path — apply the same events through `apply_actor_event`.
        let replay_scrollback = ScrollbackActor::new();
        let mut replay_renderer =
            EventRenderer::with_actors(replay_scrollback.clone(), StatusActor::new());
        for event in events {
            replay_renderer.apply_actor_event(event).await;
        }

        let live_snapshot = live_scrollback.snapshot();
        let replay_snapshot = replay_scrollback.snapshot();
        for work_id in [
            "parity-started",
            "parity-foreground",
            "parity-failed",
            "parity-cancelled",
        ] {
            let live_rows = live_snapshot
                .lines()
                .iter()
                .filter(|line| line.tool_call_id.as_deref() == Some(work_id))
                .map(|line| (line.kind, line.text.clone()))
                .collect::<Vec<_>>();
            let replay_rows = replay_snapshot
                .lines()
                .iter()
                .filter(|line| line.tool_call_id.as_deref() == Some(work_id))
                .map(|line| (line.kind, line.text.clone()))
                .collect::<Vec<_>>();
            assert_eq!(
                live_rows, replay_rows,
                "live and replay paths must emit the same BackgroundWork rows for {work_id}",
            );
        }
    }

    #[tokio::test]
    async fn renderer_has_no_actor_metadata_state_to_retain() {
        // The compatibility `apply_actor_metadata` hook has been retired; the
        // renderer no longer owns any mutable metadata for actor-owned events.
        // This test pins the pure projection by verifying that a renderer
        // built without a welcome flag still drives an `AgentStart` through
        // the actor-backed session-start path.
        let scrollback = ScrollbackActor::new();
        let status = StatusActor::new();
        let mut renderer = EventRenderer::with_actors(scrollback.clone(), status.clone());
        renderer.apply_actor_event(AgentEvent::AgentStart).await;
        assert!(scrollback
            .snapshot()
            .find_first_containing("session_start")
            .is_some());
        assert_eq!(status.snapshot().current(), &Status::Thinking);
    }

    #[tokio::test]
    async fn actor_preinjected_welcome_modal_populates_scrollback() {
        // The welcome modal is now actor-driven: callers pre-inject the
        // `welcome_modal_lines()` projection through the scrollback actor
        // before driving the renderer. The renderer neither owns nor
        // re-emits the welcome lines.
        let scrollback = ScrollbackActor::new();
        let status = StatusActor::new();
        let mut renderer = EventRenderer::with_actors(scrollback.clone(), status.clone());
        for line in crate::widgets::welcome_modal_lines() {
            scrollback.apply(ScrollbackMsg::Append(line)).await;
        }
        renderer.apply_actor_event(AgentEvent::AgentStart).await;
        assert!(
            !scrollback.snapshot().is_empty(),
            "welcome modal should populate scrollback"
        );
        assert!(scrollback
            .snapshot()
            .find_first_containing("Runie")
            .is_some());
        assert_eq!(status.snapshot().current(), &Status::Thinking);
    }

    #[tokio::test]
    async fn actor_message_update_appends_text_to_assistant_line() {
        let scrollback = ScrollbackActor::new();
        let status = StatusActor::new();
        let mut renderer = EventRenderer::with_actors(scrollback.clone(), status);
        renderer.apply_actor_event(AgentEvent::AgentStart).await;
        renderer
            .apply_actor_event(AgentEvent::MessageStart {
                message: AgentMessage::Assistant(runie_core::types::AssistantMessage {
                    content: vec![],
                    stop_reason: None,
                    model: "t".into(),
                    timestamp: 0,
                    ..runie_core::types::AssistantMessage::default()
                }),
            })
            .await;
        renderer
            .apply_actor_event(AgentEvent::MessageUpdate {
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
            })
            .await;
        assert!(scrollback
            .snapshot()
            .find_first_containing("Hello")
            .is_some());
    }

    #[tokio::test]
    async fn actor_text_delta_enters_streaming_status() {
        let scrollback = ScrollbackActor::new();
        let status = StatusActor::new();
        let mut renderer = EventRenderer::with_actors(scrollback, status.clone());
        renderer.apply_actor_event(AgentEvent::AgentStart).await;
        renderer
            .apply_actor_event(AgentEvent::MessageStart {
                message: AgentMessage::Assistant(runie_core::types::AssistantMessage {
                    content: vec![],
                    stop_reason: None,
                    model: "t".into(),
                    timestamp: 0,
                    ..runie_core::types::AssistantMessage::default()
                }),
            })
            .await;
        renderer
            .apply_actor_event(AgentEvent::MessageUpdate {
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
            })
            .await;
        assert_eq!(status.snapshot().current(), &Status::Streaming);
    }

    #[tokio::test]
    async fn actor_agent_end_sets_ready() {
        let scrollback = ScrollbackActor::new();
        let status = StatusActor::new();
        let mut renderer = EventRenderer::with_actors(scrollback.clone(), status.clone());
        renderer.apply_actor_event(AgentEvent::AgentStart).await;
        renderer
            .apply_actor_event(AgentEvent::AgentEnd { messages: vec![] })
            .await;
        assert_eq!(status.snapshot().current(), &Status::Ready);
        assert!(scrollback
            .snapshot()
            .find_first_containing("Worked for")
            .is_none());
    }

    #[tokio::test]
    #[allow(
        clippy::too_many_lines,
        reason = "with/without-TurnStart branches keep the AgentEnd closure contract together"
    )]
    async fn actor_agent_end_emits_worked_for_only_after_turn_start() {
        // With-TurnStart: AgentStart seeds the session-start rows, TurnStart
        // flips the state, and AgentEnd must emit the TurnSummary without
        // appending a phantom Separator between AgentStart and AgentEnd.
        let with_turn = ScrollbackActor::new();
        let status = StatusActor::new();
        let mut renderer = EventRenderer::with_actors(with_turn.clone(), status);
        renderer.apply_actor_event(AgentEvent::AgentStart).await;
        renderer.apply_actor_event(AgentEvent::TurnStart).await;
        assert!(with_turn.model_snapshot().turn_started);
        renderer
            .apply_actor_event(AgentEvent::AgentEnd { messages: vec![] })
            .await;
        let lines = with_turn.snapshot().lines().to_vec();
        let summary = lines
            .iter()
            .find(|line| line.text == "Worked for 0.0s")
            .expect("completion summary");
        assert_eq!(summary.kind, LineKind::TurnSummary);
        // The session-start block is the legitimate source of Separator rows;
        // AgentEnd must not insert a phantom blank row between the session
        // start and the TurnSummary. The session-start block owns exactly two
        // Separator rows (one above and one below the session_start label),
        // so the total Separator count for the with-TurnStart path is
        // exactly 2 — never 3, which would indicate the AgentEnd closure
        // re-inserted a phantom blank row.
        let separator_count = lines
            .iter()
            .filter(|line| line.kind == LineKind::Separator)
            .count();
        assert_eq!(
            separator_count, 2,
            "AgentEnd must not re-insert a phantom Separator between AgentStart and AgentEnd: {lines:?}"
        );
        // session_start emits 3 rows; AgentEnd contributes the TurnSummary
        // only (no Separator, no TurnEnd marker row). The TurnEnd reducer
        // message is a navigation-only transition, not a transcript line.
        assert_eq!(
            lines.len(),
            4,
            "with-TurnStart branch should emit exactly 4 transcript lines: {lines:?}"
        );
        assert_eq!(
            lines
                .iter()
                .filter(|line| line.kind == LineKind::TurnSummary)
                .count(),
            1,
            "with-TurnStart branch should emit exactly one TurnSummary row"
        );
        drop(renderer);

        // No-TurnStart: AgentStart seeds the session-start rows and AgentEnd
        // must not emit a TurnSummary. The session-start rows are the only
        // transcript output (3 rows, no extra Separator).
        let no_turn = ScrollbackActor::new();
        let mut renderer = EventRenderer::with_actors(no_turn.clone(), StatusActor::new());
        renderer.apply_actor_event(AgentEvent::AgentStart).await;
        renderer
            .apply_actor_event(AgentEvent::AgentEnd { messages: vec![] })
            .await;
        let no_turn_lines = no_turn.snapshot().lines().to_vec();
        assert!(
            no_turn_lines
                .iter()
                .all(|line| line.kind != LineKind::TurnSummary),
            "no-TurnStart branch must not emit a TurnSummary row"
        );
        let no_turn_separator_count = no_turn_lines
            .iter()
            .filter(|line| line.kind == LineKind::Separator)
            .count();
        assert_eq!(
            no_turn_separator_count, 2,
            "no-TurnStart branch must not gain a phantom Separator: {no_turn_lines:?}"
        );
        assert_eq!(
            no_turn_lines.len(),
            3,
            "no-TurnStart branch should emit exactly 3 transcript lines: {no_turn_lines:?}"
        );
    }

    #[tokio::test]
    async fn actor_terminal_assistant_error_message_sets_error_status() {
        let scrollback = ScrollbackActor::new();
        let status = StatusActor::new();
        let mut renderer = EventRenderer::with_actors(scrollback.clone(), status.clone());
        renderer.apply_actor_event(AgentEvent::AgentStart).await;
        renderer
            .apply_actor_event(AgentEvent::MessageStart {
                message: AgentMessage::Assistant(runie_core::types::AssistantMessage::default()),
            })
            .await;
        renderer
            .apply_actor_event(AgentEvent::MessageEnd {
                message: AgentMessage::Assistant(runie_core::types::AssistantMessage {
                    stop_reason: Some(StopReason::Error),
                    error_message: Some("api: unavailable".into()),
                    ..Default::default()
                }),
            })
            .await;
        assert_eq!(
            status.snapshot().current(),
            &Status::Error("api: unavailable".into())
        );
        assert!(scrollback
            .snapshot()
            .find_first_containing("error: api: unavailable")
            .is_some());
    }

    #[tokio::test]
    async fn actor_tool_execution_lifecycle() {
        let scrollback = ScrollbackActor::new();
        let status = StatusActor::new();
        let mut renderer = EventRenderer::with_actors(scrollback.clone(), status);
        renderer.apply_actor_event(AgentEvent::AgentStart).await;
        renderer
            .apply_actor_event(AgentEvent::ToolExecutionStart {
                tool_call_id: "1".into(),
                tool_name: "bash".into(),
                args: serde_json::json!({"cmd": "ls"}),
            })
            .await;
        assert_eq!(
            scrollback.snapshot().tool_blocks()[0].mode,
            runie_core::types::ToolDisplayMode::Truncated
        );
        assert!(scrollback.snapshot().tool_blocks()[0].is_running);
        renderer
            .apply_actor_event(AgentEvent::ToolExecutionEnd {
                tool_call_id: "1".into(),
                tool_name: "bash".into(),
                result: serde_json::json!({"ok": true}),
                is_error: false,
            })
            .await;
        let snapshot = scrollback.snapshot();
        assert!(snapshot.find_first_containing("Run ls").is_some());
        assert!(snapshot.find_first_containing("✓").is_some());
        assert!(!snapshot.tool_blocks()[0].is_running);
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

    #[tokio::test]
    async fn actor_parallel_tool_updates_stay_on_their_own_rows() {
        let scrollback = ScrollbackActor::new();
        let status = StatusActor::new();
        let mut renderer = EventRenderer::with_actors(scrollback.clone(), status);
        renderer
            .apply_actor_event(AgentEvent::ToolExecutionStart {
                tool_call_id: "a".into(),
                tool_name: "alpha".into(),
                args: serde_json::json!({}),
            })
            .await;
        renderer
            .apply_actor_event(AgentEvent::ToolExecutionStart {
                tool_call_id: "b".into(),
                tool_name: "beta".into(),
                args: serde_json::json!({}),
            })
            .await;
        renderer
            .apply_actor_event(AgentEvent::ToolExecutionUpdate {
                tool_call_id: "a".into(),
                tool_name: "alpha".into(),
                args: serde_json::json!({}),
                partial_result: serde_json::json!("a-update"),
            })
            .await;
        renderer
            .apply_actor_event(AgentEvent::ToolExecutionEnd {
                tool_call_id: "b".into(),
                tool_name: "beta".into(),
                result: serde_json::json!({}),
                is_error: false,
            })
            .await;
        let snapshot = scrollback.snapshot();
        let alpha = snapshot.find_first_containing("alpha").expect("alpha row");
        let beta = snapshot.find_first_containing("beta").expect("beta row");
        assert!(snapshot.lines()[alpha].text.contains("a-update"));
        assert!(snapshot.lines()[beta].text.contains("✓"));
        assert!(!snapshot.lines()[beta].text.contains("a-update"));
    }

    #[tokio::test]
    #[allow(clippy::cognitive_complexity, clippy::too_many_lines)]
    async fn structured_tools_use_grok_headers_and_preserve_output_rows() {
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
        assert_eq!(
            runie_tui_model::tool_result_text(&serde_json::json!("one\ntwo")),
            "one\ntwo"
        );
        assert_eq!(
            runie_tui_model::tool_result_text(&serde_json::json!({"output":"one\ntwo"})),
            "one\ntwo"
        );
        assert_eq!(
            runie_tui_model::tool_result_text(
                &serde_json::json!({"content": [], "isError": false})
            ),
            ""
        );
        assert_eq!(
            runie_tui_model::tool_result_text(
                &serde_json::json!({"content": [], "error": "denied"})
            ),
            "denied"
        );
        assert_eq!(
            web_search_site_count(
                "https://docs.rs/a\nhttps://docs.rs/b\nhttps://rust-lang.org/learn"
            ),
            2
        );
        let mut renderer = EventRenderer::with_actors(ScrollbackActor::new(), StatusActor::new());
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
    #[allow(clippy::too_many_lines)]
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
                &serde_json::json!("### Result 1 (score: 0.72, source: global)\n**File:** /memory/MEMORY.md (lines 0-1)\n```\none\n```\n### Result 2 (score: 0.42, source: session)\n**File:** /memory/session.md (lines 2-3)\n```\ntwo\n```") ,
            ),
            "Memory Search actors (2 results)"
        );
        assert_eq!(
            completed_tool_header("Workflow release", "workflow", &serde_json::json!("done")),
            "Workflow completed: release"
        );
        assert_eq!(
            completed_tool_header_with_args(
                "Read src/lib.rs",
                "read_file",
                &serde_json::json!({"offset": 40, "limit": 20}),
                &serde_json::json!({
                    "content": [{"text": "line 41\nline 42\n[18 more lines in file. Use offset=61 to continue.]"}],
                    "details": {"truncation": {"totalLines": 100}}
                })
            ),
            "Read src/lib.rs (41-42 of 100)"
        );
    }

    #[tokio::test]
    async fn actor_structured_tool_updates_append_indented_output_rows() {
        let scrollback = ScrollbackActor::new();
        let status = StatusActor::new();
        let mut renderer = EventRenderer::with_actors(scrollback.clone(), status);
        renderer
            .apply_actor_event(AgentEvent::ToolExecutionStart {
                tool_call_id: "structured".into(),
                tool_name: "read".into(),
                args: serde_json::json!({"path":"README.md"}),
            })
            .await;
        renderer
            .apply_actor_event(AgentEvent::ToolExecutionUpdate {
                tool_call_id: "structured".into(),
                tool_name: "read".into(),
                args: serde_json::json!({}),
                partial_result: serde_json::json!({"output":"first\nsecond"}),
            })
            .await;
        let snapshot = scrollback.snapshot();
        assert!(snapshot
            .lines()
            .iter()
            .any(|line| { line.kind == LineKind::ToolOutput && line.text == "first" }));
        assert!(snapshot
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

    #[tokio::test]
    async fn actor_activity_groups_do_not_merge_non_consecutive_tool_batches() {
        let scrollback = ScrollbackActor::new();
        let status = StatusActor::new();
        let mut renderer = EventRenderer::with_actors(scrollback.clone(), status);
        for (id, name) in [("first", "list_dir"), ("second", "read")] {
            if id == "second" {
                renderer
                    .apply_actor_event(AgentEvent::MessageStart {
                        message: AgentMessage::User(UserMessage {
                            content: vec![UserContent::Text {
                                text: "next".into(),
                            }],
                            timestamp: 0,
                        }),
                    })
                    .await;
            }
            renderer
                .apply_actor_event(AgentEvent::ToolExecutionStart {
                    tool_call_id: id.into(),
                    tool_name: name.into(),
                    args: serde_json::json!({}),
                })
                .await;
            renderer
                .apply_actor_event(AgentEvent::ToolExecutionEnd {
                    tool_call_id: id.into(),
                    tool_name: name.into(),
                    result: serde_json::json!({}),
                    is_error: false,
                })
                .await;
        }

        let snapshot = scrollback.snapshot();
        let activities = snapshot
            .lines()
            .iter()
            .filter(|line| line.kind == LineKind::Activity)
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>();
        assert_eq!(activities, ["◈ Listed 1 dir", "◈ Read 1 file"]);
    }
}
