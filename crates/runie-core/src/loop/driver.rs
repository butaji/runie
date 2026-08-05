//! Agent loop driver. Pure `async fn` — no actor state. The owning
//! `LoopActor` spawns this and joins it via `JoinHandle`.
//!
//! Event sequence matches the TS README exactly.

use std::sync::Arc;

use crate::convert::default_convert_to_llm;
use crate::events::EventBus;
use crate::hooks::{ShouldStopAfterTurnContext, TurnHooks};
use crate::provider::ProviderActor;
use crate::queues::{FollowUpQueueActor, SteeringQueueActor};
use crate::r#loop::turn::{decide_next_turn, TurnPlan};
use crate::state::AgentStateActor;
use crate::tools::executor::ToolExecHooks;
use crate::tools::ToolExecutorActor;
use crate::types::{
    AgentContext, AgentEvent, AgentMessage, AgentTool, AgentToolResult, AssistantContent,
    AssistantMessage, AssistantMessageEvent, Model, QueueMode, StopReason, ToolCall,
    ToolExecutionMode, ToolResultContent, ToolResultMessage, WireMessage,
};

/// Bag of dependencies the driver needs.
#[derive(Clone)]
pub struct RunLoopDeps {
    pub state: AgentStateActor,
    pub steering: SteeringQueueActor,
    pub follow_up: FollowUpQueueActor,
    pub tool_executor: ToolExecutorActor,
    pub provider: ProviderActor,
    pub bus: EventBus,
    pub hooks: ToolExecHooks,
    pub turn_hooks: TurnHooks,
    /// pi `transformContext` (agent-loop.ts:289): pre-processes the agent
    /// messages before `convert_to_llm` each turn.
    pub transform_context: Option<
        Arc<
            dyn Fn(Vec<AgentMessage>) -> futures::future::BoxFuture<'static, Vec<AgentMessage>>
                + Send
                + Sync,
        >,
    >,
    /// Abort signal: when it flips true the loop stops before the next turn
    /// (pi `Agent.abort()`).
    pub abort: Option<tokio::sync::watch::Receiver<bool>>,
    pub tool_execution_mode: ToolExecutionMode,
    pub steering_mode: QueueMode,
    pub follow_up_mode: QueueMode,
}

#[derive(Debug, Default)]
pub struct RunLoopOutcome {
    pub new_messages: Vec<AgentMessage>,
}

/// Run a full agent loop for the supplied prompts. Mirrors
/// `pi-agent-core`'s `prompt("X")` event sequence.
pub async fn run_loop(
    prompts: Vec<AgentMessage>,
    context: AgentContext,
    deps: RunLoopDeps,
) -> RunLoopOutcome {
    // Append prompts to context + state.
    let mut context = context;
    context.messages.extend(prompts.iter().cloned());

    deps.bus.publish(AgentEvent::AgentStart);
    deps.bus.publish(AgentEvent::TurnStart);

    for msg in &prompts {
        deps.bus.publish(AgentEvent::MessageStart {
            message: msg.clone(),
        });
        deps.bus.publish(AgentEvent::MessageEnd {
            message: msg.clone(),
        });
        deps.state.push_message(msg.clone()).await;
    }

    let mut all_new: Vec<AgentMessage> = prompts.clone();
    let mut tool_calls: Vec<ToolCall> = Vec::new();

    // Parity with pi-agent-core: steering messages queued before/while the
    // prompts were submitted are injected before the first assistant
    // response (the user may have typed while waiting), within the same
    // first turn — no extra `TurnStart`.
    let steering = match deps.steering_mode {
        QueueMode::OneAtATime => match deps.steering.drain_one().await {
            Some(m) => vec![m],
            None => vec![],
        },
        QueueMode::All => deps.steering.drain_all().await,
    };
    for msg in steering {
        deps.bus.publish(AgentEvent::MessageStart {
            message: msg.clone(),
        });
        deps.bus.publish(AgentEvent::MessageEnd {
            message: msg.clone(),
        });
        deps.state.push_message(msg.clone()).await;
        all_new.push(msg);
    }

    // Overrides applied by `prepareNextTurn` (pi `AgentLoopTurnUpdate`).
    let mut override_model: Option<Model> = None;
    let mut override_ctx: Option<AgentContext> = None;

    loop {
        // Abort check (pi `Agent.abort()`): stop before the next turn.
        if let Some(abort) = &deps.abort {
            if *abort.borrow() {
                deps.state.set_error(Some("aborted".into())).await;
                break;
            }
        }

        // Build the wire context and request the provider stream.
        let snap = deps.state.snapshot();
        let model = override_model.clone().unwrap_or_else(|| snap.model.clone());
        let base_ctx = AgentContext {
            system_prompt: snap.system_prompt.clone(),
            messages: snap.messages.clone(),
            tools: snap.tools.clone(),
        };
        let ctx = override_ctx.clone().unwrap_or(base_ctx);
        // pi transformContext runs before convert_to_llm (agent-loop.ts:289).
        let effective: Vec<AgentMessage> = if let Some(tf) = &deps.transform_context {
            tf(ctx.messages.clone()).await
        } else {
            ctx.messages.clone()
        };
        let wire: Vec<WireMessage> = default_convert_to_llm(&effective);

        // Mark streaming.
        deps.state.mark_streaming(true).await;

        let mut receiver = match deps
            .provider
            .start(
                model.clone(),
                AgentContext {
                    system_prompt: ctx.system_prompt.clone(),
                    messages: wire_to_agent(&wire),
                    tools: ctx.tools.clone(),
                },
                None,
            )
            .await
        {
            Some(rx) => rx,
            None => {
                deps.state.mark_streaming(false).await;
                deps.state
                    .set_error(Some("provider: no stream".into()))
                    .await;
                break;
            }
        };

        let mut assistant = AssistantMessage {
            content: vec![],
            // The streaming partial starts in `Pending` (pi proxy.ts:124);
            // `apply_event` replaces it with the final reason on done/error.
            stop_reason: Some(StopReason::Pending),
            model: model.id.clone(),
            api: model.api.clone(),
            provider: model.provider.clone(),
            ..Default::default()
        };

        deps.bus.publish(AgentEvent::MessageStart {
            message: AgentMessage::Assistant(assistant.clone()),
        });

        // Drain events.
        while let Ok(event) = receiver.recv().await {
            apply_event(&mut assistant, event.clone());
            deps.bus.publish(AgentEvent::MessageUpdate {
                message: AgentMessage::Assistant(assistant.clone()),
                event: event.clone(),
            });
            if matches!(
                event,
                AssistantMessageEvent::Done { .. } | AssistantMessageEvent::Error { .. }
            ) {
                break;
            }
        }

        deps.bus.publish(AgentEvent::MessageEnd {
            message: AgentMessage::Assistant(assistant.clone()),
        });
        deps.state.mark_streaming(false).await;
        deps.state
            .push_message(AgentMessage::Assistant(assistant.clone()))
            .await;
        all_new.push(AgentMessage::Assistant(assistant.clone()));

        // Extract tool calls.
        tool_calls = assistant
            .content
            .iter()
            .filter_map(|c| match c {
                crate::types::AssistantContent::ToolCall(tc) => Some(tc.clone()),
                _ => None,
            })
            .collect();

        let assistant_for_event = assistant.clone();
        let plan = decide_next_turn(&deps.state.snapshot(), tool_calls.clone(), false, false);

        // pi `hasMoreToolCalls`: true when a tool batch ran without every
        // result terminating, so the loop streams the next assistant response
        // that consumes the tool results (agent-loop.ts:216).
        let mut has_more_tool_calls = false;
        let mut turn_tool_results: Vec<ToolResultMessage> = Vec::new();

        match plan {
            TurnPlan::ToolBatch { calls } => {
                // Parity with pi-agent-core: a `MaxTokens` stop (the provider
                // was cut off by the output token limit) means every tool call
                // in the message may carry truncated arguments. Fail them all
                // instead of executing potentially borked calls.
                let ToolExecOutcome {
                    tool_results,
                    events,
                    all_terminated,
                } = if matches!(assistant.stop_reason, Some(StopReason::MaxTokens)) {
                    fail_truncated_calls(&calls)
                } else {
                    match deps
                        .tool_executor
                        .execute(
                            assistant_for_event,
                            calls,
                            deps.tool_execution_mode,
                            deps.hooks.clone(),
                        )
                        .await
                    {
                        crate::tools::ToolOutcome::Completed {
                            tool_results,
                            events,
                            all_terminated,
                        } => ToolExecOutcome {
                            tool_results,
                            events,
                            all_terminated,
                        },
                        crate::tools::ToolOutcome::Aborted { reason } => {
                            deps.state.set_error(Some(reason)).await;
                            break;
                        }
                    }
                };

                // Guard against repeatedly streaming the same tool calls from a
                // fixed replay stream: only continue when a batch actually ran.
                has_more_tool_calls = !tool_results.is_empty() && !all_terminated;

                for event in events {
                    deps.bus.publish(event);
                }

                turn_tool_results = tool_results.clone();
                for tr in &tool_results {
                    deps.bus.publish(AgentEvent::MessageStart {
                        message: AgentMessage::ToolResult(tr.clone()),
                    });
                    deps.bus.publish(AgentEvent::MessageEnd {
                        message: AgentMessage::ToolResult(tr.clone()),
                    });
                    deps.state
                        .push_message(AgentMessage::ToolResult(tr.clone()))
                        .await;
                    all_new.push(AgentMessage::ToolResult(tr.clone()));
                }

                deps.bus.publish(AgentEvent::TurnEnd {
                    message: AgentMessage::Assistant(assistant.clone()),
                    tool_results: tool_results.clone(),
                });
            }
            TurnPlan::Stop { .. } => {
                deps.bus.publish(AgentEvent::TurnEnd {
                    message: AgentMessage::Assistant(assistant.clone()),
                    tool_results: vec![],
                });
                break;
            }
            TurnPlan::Continue => {
                deps.bus.publish(AgentEvent::TurnEnd {
                    message: AgentMessage::Assistant(assistant.clone()),
                    tool_results: vec![],
                });
            }
        }

        // pi turn hooks (agent-loop.ts:232,247): run after turn_end, before the
        // steering/follow-up poll.
        let hook_ctx = ShouldStopAfterTurnContext {
            message: assistant.clone(),
            tool_results: turn_tool_results.clone(),
            context: ctx.clone(),
            new_messages: all_new.clone(),
        };
        if let Some(stop) = &deps.turn_hooks.should_stop_after_turn {
            if stop(hook_ctx.clone()) {
                deps.bus.publish(AgentEvent::AgentEnd {
                    messages: all_new.clone(),
                });
                return RunLoopOutcome {
                    new_messages: all_new,
                };
            }
        }
        if let Some(prepare) = &deps.turn_hooks.prepare_next_turn {
            if let Some(update) = prepare(hook_ctx) {
                if let Some(c) = update.context {
                    override_ctx = Some(c);
                }
                if let Some(m) = update.model {
                    override_model = Some(m);
                }
                if let Some(tl) = update.thinking_level {
                    deps.state.set_thinking_level(tl).await;
                }
            }
        }

        // Steering / follow-up drain.
        let steering = match deps.steering_mode {
            QueueMode::OneAtATime => match deps.steering.drain_one().await {
                Some(m) => vec![m],
                None => vec![],
            },
            QueueMode::All => deps.steering.drain_all().await,
        };
        let follow_up = match deps.follow_up_mode {
            QueueMode::OneAtATime => match deps.follow_up.drain_one().await {
                Some(m) => vec![m],
                None => vec![],
            },
            QueueMode::All => deps.follow_up.drain_all().await,
        };

        let steering_empty = steering.is_empty();
        let follow_up_empty = follow_up.is_empty();
        let any_injected = !steering_empty || !follow_up_empty;

        for msg in steering.into_iter().chain(follow_up.into_iter()) {
            deps.bus.publish(AgentEvent::MessageStart {
                message: msg.clone(),
            });
            deps.bus.publish(AgentEvent::MessageEnd {
                message: msg.clone(),
            });
            deps.state.push_message(msg.clone()).await;
            all_new.push(msg);
        }

        if !any_injected && !has_more_tool_calls {
            break;
        }
        deps.bus.publish(AgentEvent::TurnStart);
    }

    deps.bus.publish(AgentEvent::AgentEnd {
        messages: all_new.clone(),
    });
    RunLoopOutcome {
        new_messages: all_new,
    }
}

/// Continue from existing context (no new prompt).
pub async fn run_loop_continue(context: AgentContext, deps: RunLoopDeps) -> RunLoopOutcome {
    let prompts = vec![];
    run_loop(prompts, context, deps).await
}

struct ToolExecOutcome {
    tool_results: Vec<ToolResultMessage>,
    events: Vec<AgentEvent>,
    /// True when every result had `terminate: true` (pi `shouldTerminateToolBatch`).
    all_terminated: bool,
}

/// Synthesize error results for every tool call in a message that was
/// truncated by the output token limit. Mirrors pi-agent-core's
/// `failToolCallsFromTruncatedMessage`: no tool is executed; each call is
/// reported as an error so the caller can re-issue it with complete
/// arguments.
fn fail_truncated_calls(calls: &[ToolCall]) -> ToolExecOutcome {
    let mut tool_results = Vec::with_capacity(calls.len());
    let mut events = Vec::with_capacity(calls.len() * 2);
    for call in calls {
        events.push(AgentEvent::ToolExecutionStart {
            tool_call_id: call.id.clone(),
            tool_name: call.name.clone(),
            args: call.arguments.clone(),
        });
        let reason = format!(
            "Tool call \"{}\" was not executed: the response hit the output token limit, so its \
             arguments may be truncated. Re-issue the tool call with complete arguments.",
            call.name
        );
        let result = AgentToolResult {
            content: vec![ToolResultContent::Text {
                text: reason.clone(),
            }],
            details: serde_json::Value::Null,
            usage: None,
            added_tool_names: vec![],
            terminate: false,
        };
        events.push(AgentEvent::ToolExecutionEnd {
            tool_call_id: call.id.clone(),
            tool_name: call.name.clone(),
            result: serde_json::to_value(&result).unwrap_or_default(),
            is_error: true,
        });
        tool_results.push(ToolResultMessage {
            tool_call_id: call.id.clone(),
            tool_name: call.name.clone(),
            content: result.content.clone(),
            is_error: true,
            ..Default::default()
        });
    }
    ToolExecOutcome {
        tool_results,
        events,
        // pi failToolCallsFromTruncatedMessage returns terminate:false, so the
        // loop continues to a follow-up turn (agent-loop.ts:405).
        all_terminated: false,
    }
}

fn apply_event(assistant: &mut AssistantMessage, event: AssistantMessageEvent) {
    use crate::types::AssistantContent;
    match event {
        AssistantMessageEvent::Start => {}
        AssistantMessageEvent::TextDelta { delta } => {
            push_or_append(assistant, AssistantContent::Text { text: delta });
        }
        AssistantMessageEvent::ThinkingDelta { delta } => {
            push_or_append(assistant, AssistantContent::Thinking { text: delta });
        }
        AssistantMessageEvent::ToolCallDelta { partial, .. } => {
            assistant.content.push(AssistantContent::ToolCall(partial));
        }
        AssistantMessageEvent::Done { stop_reason, usage } => {
            assistant.stop_reason = Some(stop_reason);
            assistant.usage = usage;
        }
        AssistantMessageEvent::Error { error } => {
            assistant.stop_reason = Some(StopReason::Error);
            assistant.error_message = Some(error.clone());
            assistant
                .content
                .push(AssistantContent::Text { text: error });
        }
    }
}

fn push_or_append(assistant: &mut AssistantMessage, content: AssistantContent) {
    use crate::types::AssistantContent;
    match (assistant.content.last_mut(), &content) {
        (Some(AssistantContent::Text { text }), AssistantContent::Text { text: new_text }) => {
            text.push_str(new_text.as_str());
            return;
        }
        (
            Some(AssistantContent::Thinking { text }),
            AssistantContent::Thinking { text: new_text },
        ) => {
            text.push_str(new_text.as_str());
            return;
        }
        _ => {}
    }
    assistant.content.push(content);
}

fn wire_to_agent(wire: &[WireMessage]) -> Vec<AgentMessage> {
    wire.iter()
        .map(|w| match w {
            WireMessage::User { content, timestamp } => {
                AgentMessage::User(crate::types::UserMessage {
                    content: content.clone(),
                    timestamp: *timestamp,
                })
            }
            WireMessage::Assistant {
                content,
                stop_reason,
                model,
                timestamp,
            } => AgentMessage::Assistant(crate::types::AssistantMessage {
                content: content.clone(),
                stop_reason: *stop_reason,
                model: model.clone(),
                timestamp: *timestamp,
                ..Default::default()
            }),
            WireMessage::ToolResult {
                tool_call_id,
                tool_name,
                content,
                is_error,
                timestamp,
            } => AgentMessage::ToolResult(ToolResultMessage {
                tool_call_id: tool_call_id.clone(),
                tool_name: tool_name.clone(),
                content: content.clone(),
                is_error: *is_error,
                timestamp: *timestamp,
                ..Default::default()
            }),
        })
        .collect()
}

#[allow(dead_code)]
fn _tools_marker(_t: &[Arc<dyn AgentTool>]) {}
