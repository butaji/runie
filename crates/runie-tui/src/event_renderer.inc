// `EventRenderer` — subscribes to `runie-core`'s event bus and mutates widgets.

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
    reason = "the compatibility export preserves existing replay call sites"
)]
pub fn scrollback_messages_for_event(event: &AgentEvent) -> Vec<ScrollbackMsg> {
    runie_tui_model::scrollback_messages_for_event(event)
}
pub struct EventRenderer {
    scrollback_actor: ScrollbackActor,
    status_actor: StatusActor,
    /// Absolute workspace path used to strip provider-supplied absolute paths
    /// down to workspace-relative form during tool-header projection. Owned
    /// by the renderer so replays and live buses agree on the same anchor
    /// regardless of `std::env::current_dir` drift between hosts.
    workspace: String,
    /// The live Grok surface places the thinking row directly after the user
    /// entry; deterministic replay keeps the historical four-row contract.
    live_grok_layout: bool,
}

impl EventRenderer {
    fn with_actors_inner(
        scrollback_actor: ScrollbackActor,
        status_actor: StatusActor,
        workspace: String,
    ) -> Self {
        Self {
            scrollback_actor,
            status_actor,
            workspace,
            live_grok_layout: false,
        }
    }

    /// Build the production renderer with its SSOT actors attached at
    /// construction time. The compatibility constructors remain for the
    /// synchronous YAML harness and focused reducer tests.
    pub fn with_actors(
        scrollback_actor: ScrollbackActor,
        status_actor: StatusActor,
        workspace: impl Into<String>,
    ) -> Self {
        Self::with_actors_inner(scrollback_actor, status_actor, workspace.into())
    }

    /// Production interactive projection with Grok's live assistant spacing.
    pub fn with_live_actors(
        scrollback_actor: ScrollbackActor,
        status_actor: StatusActor,
        workspace: impl Into<String>,
    ) -> Self {
        let mut renderer = Self::with_actors(scrollback_actor, status_actor, workspace);
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
            let animation_demand = status_actor.shared_model_snapshot().animation_demand()
                || scrollback_actor.shared_snapshot().animation_demand();
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
                            let turn_was_started = scrollback_snapshot.facts.turn_started;
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
        let tool_message = self.tool_message_for_event(&event);
        // One atomic `FeedSnapshot` read per replayed event. The remainder of
        // this body is synchronous, so the turn flag and the
        // assistant-finalize inputs observe the same scrollback generation
        // instead of two independently torn reads.
        let scrollback_snapshot = scrollback_actor.model_snapshot();
        let turn_was_started = scrollback_snapshot.facts.turn_started;
        let mut messages = scrollback_messages_for_event(&event);
        append_lifecycle_messages(&event, &mut messages, turn_was_started, &status_actor);
        self.append_assistant_messages(&event, &scrollback_snapshot, &mut messages);
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

    fn tool_message_for_event(&mut self, event: &AgentEvent) -> Option<ScrollbackMsg> {
        match event {
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
        }
    }

    fn append_assistant_messages(
        &self,
        event: &AgentEvent,
        snapshot: &runie_tui_model::FeedSnapshot,
        messages: &mut Vec<ScrollbackMsg>,
    ) {
        if let AgentEvent::MessageEnd {
            message: runie_core::types::AgentMessage::Assistant(assistant),
        } = event
        {
            let has_reasoning = snapshot
                .lines
                .iter()
                .any(|line| line.kind == LineKind::Reasoning && !line.text.is_empty());
            let elapsed = self.thinking_elapsed_ms();
            messages.push(ScrollbackMsg::FinalizeAssistant {
                has_reasoning,
                reasoning_expanded: snapshot.reasoning_expanded,
                summary: thinking_summary(elapsed),
                settled_no_tool_phase: elapsed.is_some() && snapshot.tool_blocks.is_empty(),
            });
            if let Some(error) = &assistant.error_message {
                messages.push(ScrollbackMsg::Append(Line::new(
                    LineKind::System,
                    format!("error: {error}"),
                )));
            }
        }
        if let AgentEvent::MessageUpdate {
            event: AssistantMessageEvent::Error { error, .. },
            ..
        } = event
        {
            messages.push(ScrollbackMsg::Append(Line::new(
                LineKind::System,
                format!("error: {}", error.error_text()),
            )));
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
        let tool_buffer = runie_tui_model::tool_header(&tool_name, &args, &self.workspace);
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
        let header = completed_tool_header(&snapshot, &tool_call_id, &tool_name, &result, is_error);
        let activity = completed_tool_activity(&snapshot, is_error);
        let output = completed_tool_output(&tool_name, &result, is_error);
        ScrollbackMsg::ToolEnd {
            tool_call_id,
            header,
            activity,
            output,
        }
    }
}

fn completed_tool_header(
    snapshot: &FeedSnapshot,
    tool_call_id: &str,
    tool_name: &str,
    result: &serde_json::Value,
    is_error: bool,
) -> String {
    let header = current_tool_header(snapshot, tool_call_id).unwrap_or_default();
    if is_error {
        header
    } else {
        runie_tui_model::completed_tool_header_with_args(
            &header,
            tool_name,
            &current_tool_args(snapshot, tool_call_id),
            result,
        )
    }
}

fn completed_tool_activity(snapshot: &FeedSnapshot, is_error: bool) -> Option<String> {
    let (dirs, files, commands, subagents, failures) = activity_counts(snapshot);
    let failures = failures + usize::from(is_error);
    if active_tool_count(snapshot) <= 1 && dirs + files + commands + subagents > 0 {
        Some(activity_text(
            dirs, files, commands, subagents, failures, false,
        ))
    } else {
        None
    }
}

fn completed_tool_output(
    tool_name: &str,
    result: &serde_json::Value,
    is_error: bool,
) -> Vec<(LineKind, String)> {
    let raw = runie_tui_model::tool_result_text(result);
    let kind = if is_error {
        LineKind::ToolError
    } else if runie_tui_model::is_output_tool(tool_name) {
        LineKind::ToolOutput
    } else {
        LineKind::ToolResult
    };
    let lines = if matches!(tool_name, "memory_search" | "memory-search") {
        runie_tui_model::memory_display_lines(&raw)
    } else {
        raw.lines().map(str::to_owned).collect()
    };
    let mut output = lines
        .into_iter()
        .filter(|line| !line.is_empty())
        .map(|line| (kind, line))
        .collect::<Vec<_>>();
    if !is_error && matches!(tool_name, "web_search" | "web-search") {
        if let Some(sources) = runie_tui_model::web_search_sources_line(&raw) {
            output.push((LineKind::ToolResult, sources));
        }
    }
    output
}

fn append_lifecycle_messages(
    event: &AgentEvent,
    messages: &mut Vec<ScrollbackMsg>,
    turn_was_started: bool,
    status_actor: &StatusActor,
) {
    if matches!(event, AgentEvent::TurnStart) {
        messages.push(ScrollbackMsg::TurnStart);
    }
    if matches!(event, AgentEvent::AgentStart) {
        messages.extend(session_start_messages());
    }
    if matches!(event, AgentEvent::AgentEnd { .. }) {
        if turn_was_started {
            messages.push(ScrollbackMsg::AppendTurnSummary(
                status_actor.model_snapshot().worked_for_label(),
            ));
        }
        messages.push(ScrollbackMsg::TurnEnd);
    }
}

fn current_tool_header(snapshot: &FeedSnapshot, tool_call_id: &str) -> Option<String> {
    runie_tui_model::current_tool_header(snapshot, tool_call_id)
}

fn current_tool_args(snapshot: &FeedSnapshot, tool_call_id: &str) -> serde_json::Value {
    runie_tui_model::current_tool_args(snapshot, tool_call_id)
}

fn active_tool_count(snapshot: &FeedSnapshot) -> usize {
    runie_tui_model::active_tool_count(snapshot)
}

fn activity_counts(snapshot: &FeedSnapshot) -> (usize, usize, usize, usize, usize) {
    runie_tui_model::activity_counts(snapshot)
}

fn activity_group_exists_since_latest_user(snapshot: &FeedSnapshot) -> bool {
    runie_tui_model::activity_group_exists_since_latest_user(snapshot)
}

fn activity_counts_with_start(
    snapshot: &FeedSnapshot,
    tool_name: &str,
    reset: bool,
) -> (usize, usize, usize, usize, usize) {
    runie_tui_model::activity_counts_with_start(snapshot, tool_name, reset)
}

#[allow(
    clippy::too_many_lines,
    reason = "the pure tool-header DSL keeps Grok's specialized card vocabulary together"
)]
#[cfg(test)]
pub(crate) fn tool_header(workspace: &str, tool_name: &str, args: &serde_json::Value) -> String {
    runie_tui_model::tool_header(tool_name, args, workspace)
}

/// Grok displays tool paths relative to the active workspace whenever the
/// provider sends an absolute path. The renderer defers to
/// `runie_tui_model::tool_header` so replay fixtures remain independent of
/// the host's username; the workspace anchor is owned by the renderer.
#[cfg(test)]
fn make_relative_path(workspace: &str, path: &str) -> String {
    runie_tui_model::make_relative_path(workspace, path)
}

fn structured_memory_lines(output: &str) -> Vec<String> {
    runie_tui_model::memory_display_lines(output)
}

#[allow(
    clippy::cognitive_complexity,
    reason = "activity label projection keeps Grok's ordered vocabulary together"
)]
fn session_start_messages() -> Vec<ScrollbackMsg> {
    runie_tui_model::session_start_messages()
}

#[cfg(test)]
mod tests {
    use super::*;

    use runie_core::types::{AgentMessage, StopReason, ThemeKind, Usage, UserContent, UserMessage};

    /// Anchor workspace used by reducer/projection tests that do not exercise
    /// absolute-path stripping. The renderer now defers to
    /// `runie_tui_model::tool_header`, which strips this prefix from
    /// tool-supplied absolute paths.
    const TEST_WORKSPACE: &str = "/work";

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
    fn feed_event_mapping_is_pure_and_explicit() {
        assert_feed_control_mapping();
        assert_feed_tool_mapping();
        assert_feed_message_mapping();
    }

    fn assert_feed_control_mapping() {
        let reset = scrollback_messages_for_event(&AgentEvent::Reset);
        assert_eq!(reset, vec![ScrollbackMsg::Clear]);
        let theme = scrollback_messages_for_event(&AgentEvent::ThemeChanged {
            theme: ThemeKind::GrokDay,
        });
        assert_eq!(theme, vec![ScrollbackMsg::SetTheme(ThemeKind::GrokDay)]);
        assert!(scrollback_messages_for_event(&AgentEvent::TurnStart).is_empty());
    }

    fn assert_feed_tool_mapping() {
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
    }

    fn assert_feed_message_mapping() {
        assert_feed_user_mapping();
        assert_feed_assistant_mapping();
    }

    fn assert_feed_user_mapping() {
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
    }

    fn assert_feed_assistant_mapping() {
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
        assert_layout_message_shapes(&assistant_start());
        assert_live_assistant_start(&assistant_start()).await;
        assert_replay_assistant_start(&assistant_start()).await;
    }

    fn assert_layout_message_shapes(event: &AgentEvent) {
        let mut live_messages = scrollback_messages_for_event(event);
        live_messages.remove(1);
        let replay_messages = scrollback_messages_for_event(event);
        assert_eq!(live_messages.len(), 4);
        assert_eq!(replay_messages.len(), 5);
    }

    async fn assert_live_assistant_start(event: &AgentEvent) {
        let live_bus = runie_core::events::EventBus::new();
        let live_scrollback = ScrollbackActor::new();
        let live_status = StatusActor::new();
        let mut live_feed = live_scrollback.subscribe();
        let live_renderer =
            EventRenderer::with_live_actors(live_scrollback.clone(), live_status, TEST_WORKSPACE);
        let (live_shutdown_tx, live_shutdown_rx) = tokio::sync::watch::channel(false);
        // OWNER: test — joins the renderer after the shutdown event.
        let live_task = tokio::spawn(live_renderer.run(live_bus.subscribe(), live_shutdown_rx));

        live_bus.publish(event.clone());
        live_feed
            .changed()
            .await
            .expect("live assistant start delivery");
        assert_eq!(live_scrollback.model_snapshot().lines.len(), 3);

        live_shutdown_tx.send(true).expect("live renderer shutdown");
        live_task.await.expect("live renderer task");
    }

    async fn assert_replay_assistant_start(event: &AgentEvent) {
        let replay_scrollback = ScrollbackActor::new();
        let replay_status = StatusActor::new();
        let mut replay_renderer =
            EventRenderer::with_actors(replay_scrollback.clone(), replay_status, TEST_WORKSPACE);
        replay_renderer.apply_actor_event(event.clone()).await;
        assert_eq!(replay_scrollback.model_snapshot().lines.len(), 4);
    }
    #[tokio::test]
    async fn actor_renderer_reduces_events_without_legacy_projections() {
        let scrollback = ScrollbackActor::new();
        let status = StatusActor::new();
        let mut renderer =
            EventRenderer::with_actors(scrollback.clone(), status.clone(), TEST_WORKSPACE);

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
        let renderer = EventRenderer::with_actors(scrollback, status.clone(), TEST_WORKSPACE);
        let mut status_updates = status.shared_subscribe();
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
        let renderer = EventRenderer::with_live_actors(scrollback.clone(), status, TEST_WORKSPACE);
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        // OWNER: test — joins the renderer after the shutdown event.
        let task = tokio::spawn(renderer.run(bus.subscribe(), shutdown_rx));

        publish_tool_start(&bus);
        assert_tool_started(&mut feed, &scrollback).await;

        publish_tool_update(&bus);
        assert_tool_updated(&mut feed, &scrollback).await;

        publish_tool_end(&bus);
        assert_tool_ended(&mut feed, &scrollback).await;

        shutdown_tx.send(true).expect("renderer shutdown");
        task.await.expect("renderer task");
    }

    fn publish_tool_start(bus: &runie_core::events::EventBus) {
        bus.publish(AgentEvent::ToolExecutionStart {
            tool_call_id: "tool-1".into(),
            tool_name: "read".into(),
            args: serde_json::json!({"path":"src/lib.rs"}),
        });
    }

    async fn assert_tool_started(
        feed: &mut tokio::sync::watch::Receiver<FeedSnapshot>,
        scrollback: &ScrollbackActor,
    ) {
        feed.changed().await.expect("tool start delivery");
        assert!(scrollback
            .model_snapshot()
            .tool_blocks
            .iter()
            .any(|block| block.tool_call_id == "tool-1" && block.is_running));
    }

    fn publish_tool_update(bus: &runie_core::events::EventBus) {
        bus.publish(AgentEvent::ToolExecutionUpdate {
            tool_call_id: "tool-1".into(),
            tool_name: "read".into(),
            args: serde_json::json!({"path":"src/lib.rs"}),
            partial_result: serde_json::json!({"output":"hello"}),
        });
    }

    async fn assert_tool_updated(
        feed: &mut tokio::sync::watch::Receiver<FeedSnapshot>,
        scrollback: &ScrollbackActor,
    ) {
        feed.changed().await.expect("tool update delivery");
        assert!(scrollback
            .model_snapshot()
            .tool_blocks
            .iter()
            .any(|block| block.tool_call_id == "tool-1"
                && block.output.iter().any(|row| row == "hello")));
    }

    fn publish_tool_end(bus: &runie_core::events::EventBus) {
        bus.publish(AgentEvent::ToolExecutionEnd {
            tool_call_id: "tool-1".into(),
            tool_name: "read".into(),
            result: serde_json::json!({"output":"hello"}),
            is_error: false,
        });
    }

    async fn assert_tool_ended(
        feed: &mut tokio::sync::watch::Receiver<FeedSnapshot>,
        scrollback: &ScrollbackActor,
    ) {
        feed.changed().await.expect("tool end delivery");
        assert!(
            scrollback
                .model_snapshot()
                .tool_blocks
                .iter()
                .any(|block| block.tool_call_id == "tool-1" && !block.is_running),
            "ToolExecutionEnd must mark the block as completed in a single actor hop"
        );
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
        let started_renderer = EventRenderer::with_live_actors(
            started_scrollback.clone(),
            StatusActor::new(),
            TEST_WORKSPACE,
        );
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
        let progress_renderer = EventRenderer::with_live_actors(
            progress_scrollback.clone(),
            StatusActor::new(),
            TEST_WORKSPACE,
        );
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
        let finished_renderer = EventRenderer::with_live_actors(
            finished_scrollback.clone(),
            StatusActor::new(),
            TEST_WORKSPACE,
        );
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
        let failed_renderer = EventRenderer::with_live_actors(
            failed_scrollback.clone(),
            StatusActor::new(),
            TEST_WORKSPACE,
        );
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
        let cancelled_renderer = EventRenderer::with_live_actors(
            cancelled_scrollback.clone(),
            StatusActor::new(),
            TEST_WORKSPACE,
        );
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
        let events = background_parity_events();

        // Live path — drive every event through a `run` loop and snapshot
        // the per-`work_id` transcript rows.
        let live_bus = runie_core::events::EventBus::new();
        let live_scrollback = ScrollbackActor::new();
        let live_renderer = EventRenderer::with_live_actors(
            live_scrollback.clone(),
            StatusActor::new(),
            TEST_WORKSPACE,
        );
        let (live_shutdown, live_shutdown_rx) = tokio::sync::watch::channel(false);
        // OWNER: test — joins the renderer after the shutdown event.
        let live_task = tokio::spawn(live_renderer.run(live_bus.subscribe(), live_shutdown_rx));
        for event in &events {
            live_bus.publish(event.clone());
        }
        for _ in 0..events.len() {
            live_scrollback
                .subscribe()
                .changed()
                .await
                .expect("scrollback actor alive");
        }
        live_shutdown.send(true).expect("renderer shutdown");
        live_task.await.expect("renderer task");

        let replay_scrollback = ScrollbackActor::new();
        let mut replay_renderer = EventRenderer::with_actors(
            replay_scrollback.clone(),
            StatusActor::new(),
            TEST_WORKSPACE,
        );
        for event in events {
            replay_renderer.apply_actor_event(event).await;
        }

        assert_background_parity(&live_scrollback.snapshot(), &replay_scrollback.snapshot());
    }

    fn background_parity_events() -> Vec<AgentEvent> {
        vec![
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
                error: Some("oops".into()),
            },
            AgentEvent::BackgroundWorkCancelled {
                work_id: "parity-cancelled".into(),
                description: "cancelled task".into(),
                elapsed_ms: Some(100),
            },
        ]
    }

    fn assert_background_parity(
        live_snapshot: &crate::widgets::scrollback::Scrollback,
        replay_snapshot: &crate::widgets::scrollback::Scrollback,
    ) {
        for work_id in [
            "parity-started",
            "parity-foreground",
            "parity-failed",
            "parity-cancelled",
        ] {
            let rows = |snapshot: &crate::widgets::scrollback::Scrollback| {
                snapshot
                    .lines()
                    .iter()
                    .filter(|line| line.tool_call_id.as_deref() == Some(work_id))
                    .map(|line| (line.kind, line.text.clone()))
                    .collect::<Vec<_>>()
            };
            assert_eq!(
                rows(live_snapshot),
                rows(replay_snapshot),
                "live and replay paths must emit the same BackgroundWork rows for {work_id}"
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
        let mut renderer =
            EventRenderer::with_actors(scrollback.clone(), status.clone(), TEST_WORKSPACE);
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
        let mut renderer =
            EventRenderer::with_actors(scrollback.clone(), status.clone(), TEST_WORKSPACE);
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
        let mut renderer = EventRenderer::with_actors(scrollback.clone(), status, TEST_WORKSPACE);
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
        let mut renderer = EventRenderer::with_actors(scrollback, status.clone(), TEST_WORKSPACE);
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
        let mut renderer =
            EventRenderer::with_actors(scrollback.clone(), status.clone(), TEST_WORKSPACE);
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
        assert_agent_end_with_turn().await;

        assert_agent_end_without_turn().await;
    }

    async fn assert_agent_end_with_turn() {
        let scrollback = ScrollbackActor::new();
        let mut renderer =
            EventRenderer::with_actors(scrollback.clone(), StatusActor::new(), TEST_WORKSPACE);
        renderer.apply_actor_event(AgentEvent::AgentStart).await;
        renderer.apply_actor_event(AgentEvent::TurnStart).await;
        assert!(scrollback.model_snapshot().facts.turn_started);
        renderer
            .apply_actor_event(AgentEvent::AgentEnd { messages: vec![] })
            .await;
        let lines = scrollback.snapshot().lines().to_vec();
        assert!(lines.iter().any(|line| line.kind == LineKind::TurnSummary));
        assert_eq!(
            lines
                .iter()
                .filter(|line| line.kind == LineKind::Separator)
                .count(),
            2
        );
        assert_eq!(lines.len(), 4);
        assert_eq!(
            lines
                .iter()
                .filter(|line| line.kind == LineKind::TurnSummary)
                .count(),
            1
        );
    }

    async fn assert_agent_end_without_turn() {
        let scrollback = ScrollbackActor::new();
        let mut renderer =
            EventRenderer::with_actors(scrollback.clone(), StatusActor::new(), TEST_WORKSPACE);
        renderer.apply_actor_event(AgentEvent::AgentStart).await;
        renderer
            .apply_actor_event(AgentEvent::AgentEnd { messages: vec![] })
            .await;
        let lines = scrollback.snapshot().lines().to_vec();
        assert!(lines.iter().all(|line| line.kind != LineKind::TurnSummary));
        assert_eq!(
            lines
                .iter()
                .filter(|line| line.kind == LineKind::Separator)
                .count(),
            2
        );
        assert_eq!(lines.len(), 3);
    }

    #[tokio::test]
    async fn actor_terminal_assistant_error_message_sets_error_status() {
        let scrollback = ScrollbackActor::new();
        let status = StatusActor::new();
        let mut renderer =
            EventRenderer::with_actors(scrollback.clone(), status.clone(), TEST_WORKSPACE);
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
        let mut renderer = EventRenderer::with_actors(scrollback.clone(), status, TEST_WORKSPACE);
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
        let mut renderer = EventRenderer::with_actors(scrollback.clone(), status, TEST_WORKSPACE);
        apply_parallel_tool_events(&mut renderer).await;
        let snapshot = scrollback.snapshot();
        let alpha = snapshot.find_first_containing("alpha").expect("alpha row");
        let beta = snapshot.find_first_containing("beta").expect("beta row");
        assert!(snapshot.lines()[alpha].text.contains("a-update"));
        assert!(snapshot.lines()[beta].text.contains("✓"));
        assert!(!snapshot.lines()[beta].text.contains("a-update"));
    }

    async fn apply_tool_event(renderer: &mut EventRenderer, event: AgentEvent) {
        renderer.apply_actor_event(event).await;
    }

    async fn apply_parallel_tool_events(renderer: &mut EventRenderer) {
        apply_tool_event(
            renderer,
            AgentEvent::ToolExecutionStart {
                tool_call_id: "a".into(),
                tool_name: "alpha".into(),
                args: serde_json::json!({}),
            },
        )
        .await;
        apply_tool_event(
            renderer,
            AgentEvent::ToolExecutionStart {
                tool_call_id: "b".into(),
                tool_name: "beta".into(),
                args: serde_json::json!({}),
            },
        )
        .await;
        apply_tool_event(
            renderer,
            AgentEvent::ToolExecutionUpdate {
                tool_call_id: "a".into(),
                tool_name: "alpha".into(),
                args: serde_json::json!({}),
                partial_result: serde_json::json!("a-update"),
            },
        )
        .await;
        apply_tool_event(
            renderer,
            AgentEvent::ToolExecutionEnd {
                tool_call_id: "b".into(),
                tool_name: "beta".into(),
                result: serde_json::json!({}),
                is_error: false,
            },
        )
        .await;
    }

    #[tokio::test]
    #[allow(clippy::cognitive_complexity, clippy::too_many_lines)]
    async fn structured_tools_use_grok_headers_and_preserve_output_rows() {
        let workspace = "/work";
        assert_structured_tool_headers(workspace);
        assert_structured_tool_outputs();
        let mut renderer =
            EventRenderer::with_actors(ScrollbackActor::new(), StatusActor::new(), workspace);
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

    fn assert_structured_tool_outputs() {
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
            runie_tui_model::web_search_site_count(
                "https://docs.rs/a\nhttps://docs.rs/b\nhttps://rust-lang.org/learn"
            ),
            2
        );
    }

    fn assert_structured_tool_headers(workspace: &str) {
        assert_structured_tool_headers_primary(workspace);
        let cases = [
            (
                "memory_search",
                serde_json::json!({"query":"actors"}),
                "Memory Search actors",
            ),
            (
                "search_tools",
                serde_json::json!({"query":"bash"}),
                "Search Tools bash",
            ),
            (
                "search-tools",
                serde_json::json!({"pattern":"bash"}),
                "Search Tools bash",
            ),
            (
                "search_tool",
                serde_json::json!({"query":""}),
                "Search Tools ",
            ),
            (
                "workflow",
                serde_json::json!({"name":"release"}),
                "Workflow release",
            ),
            ("use", serde_json::json!({"tool":"browser"}), "Use browser"),
        ];
        for (name, args, expected) in cases {
            assert_eq!(tool_header(workspace, name, &args), expected);
        }
    }

    fn assert_structured_tool_headers_primary(workspace: &str) {
        let cases = [
            ("list_dir", serde_json::json!({"path":"."}), "List ."),
            (
                "read",
                serde_json::json!({"path":"README.md"}),
                "Read README.md",
            ),
            (
                "edit",
                serde_json::json!({"path":"src/main.rs"}),
                "Edit src/main.rs",
            ),
            (
                "search",
                serde_json::json!({"pattern":"TODO","path":"src"}),
                "Search \"TODO\" in src",
            ),
            (
                "web_search",
                serde_json::json!({"query":"rust tui"}),
                "Web Search rust tui",
            ),
            (
                "web_fetch",
                serde_json::json!({"url":"https://example.com"}),
                "Fetch https://example.com",
            ),
        ];
        for (name, args, expected) in cases {
            assert_eq!(tool_header(workspace, name, &args), expected);
        }
    }

    #[test]
    fn absolute_tool_paths_are_workspace_relative() {
        let workspace = "/work";
        let absolute = format!("{workspace}/src/main.rs");
        assert_eq!(
            tool_header(workspace, "read", &serde_json::json!({"path": absolute})),
            "Read src/main.rs"
        );
        assert_eq!(
            tool_header(
                workspace,
                "list_dir",
                &serde_json::json!({"path": absolute})
            ),
            "List src/main.rs"
        );
        assert_eq!(
            tool_header(
                workspace,
                "edit",
                &serde_json::json!({"path": format!("{workspace}/Cargo.toml")})
            ),
            "Edit Cargo.toml"
        );
        assert_eq!(make_relative_path(workspace, workspace), ".");
        assert_eq!(
            make_relative_path(workspace, "/tmp/other/file"),
            "/tmp/other/file"
        );
    }

    #[test]
    fn renderer_tool_header_projects_absolute_paths_through_workspace_anchor() {
        // The renderer no longer calls `std::env::current_dir` directly; it
        // threads the workspace through the model projection so replays and
        // hosts agree on the same anchor regardless of where the binary was
        // invoked from.
        let workspace = "/Users/admin/Code/GitHub/runie-tests/runie";
        let absolute = format!("{workspace}/crates/runie-tui/src/event_renderer.rs");
        assert_eq!(
            runie_tui_model::tool_header("read", &serde_json::json!({"path": absolute}), workspace,),
            "Read crates/runie-tui/src/event_renderer.rs"
        );
        // A path that already lives outside the workspace is rendered as the
        // original absolute path so cross-host fixtures stay identifiable.
        assert_eq!(
            runie_tui_model::tool_header(
                "list_dir",
                &serde_json::json!({"path": "/tmp/other"}),
                workspace,
            ),
            "List /tmp/other"
        );
    }

    #[test]
    fn renderer_tool_header_strips_leading_separator_after_workspace() {
        // Paths supplied as `<workspace>/relative` (with the canonical
        // separator) collapse to the relative form. The renderer's
        // workspace-relative projection is centralized in
        // `runie_tui_model::tool_header` so this assertion pins the model
        // contract rather than a renderer-local helper.
        let workspace = "/work";
        assert_eq!(
            runie_tui_model::tool_header(
                "read",
                &serde_json::json!({"path": format!("{workspace}/README.md")}),
                workspace,
            ),
            "Read README.md"
        );
    }

    #[tokio::test]
    async fn actor_structured_tool_updates_append_indented_output_rows() {
        let scrollback = ScrollbackActor::new();
        let status = StatusActor::new();
        let mut renderer = EventRenderer::with_actors(scrollback.clone(), status, TEST_WORKSPACE);
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
        let mut renderer = EventRenderer::with_actors(scrollback.clone(), status, TEST_WORKSPACE);
        for (id, name) in [("first", "list_dir"), ("second", "read")] {
            if id == "second" {
                apply_user_message(&mut renderer).await;
            }
            apply_tool_lifecycle(&mut renderer, id, name).await;
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

    async fn apply_user_message(renderer: &mut EventRenderer) {
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

    async fn apply_tool_lifecycle(renderer: &mut EventRenderer, id: &str, name: &str) {
        apply_tool_event(
            renderer,
            AgentEvent::ToolExecutionStart {
                tool_call_id: id.into(),
                tool_name: name.into(),
                args: serde_json::json!({}),
            },
        )
        .await;
        apply_tool_event(
            renderer,
            AgentEvent::ToolExecutionEnd {
                tool_call_id: id.into(),
                tool_name: name.into(),
                result: serde_json::json!({}),
                is_error: false,
            },
        )
        .await;
    }
}
