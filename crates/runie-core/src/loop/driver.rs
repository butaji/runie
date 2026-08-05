//! Agent loop driver. Pure `async fn` — no actor state. The owning
//! `LoopActor` spawns this and joins it via `JoinHandle`.
//!
//! Event sequence matches the TS README exactly.

use std::sync::Arc;

use crate::convert::default_convert_to_llm;
use crate::events::EventBus;
use crate::provider::ProviderActor;
use crate::queues::{FollowUpQueueActor, SteeringQueueActor};
use crate::r#loop::turn::{decide_next_turn, TurnPlan};
use crate::state::AgentStateActor;
use crate::tools::executor::ToolExecHooks;
use crate::tools::ToolExecutorActor;
use crate::types::{
    AgentContext, AgentEvent, AgentMessage, AgentTool, AssistantContent, AssistantMessage,
    AssistantMessageEvent, QueueMode, StopReason, ToolCall, ToolExecutionMode, ToolResultMessage,
    WireMessage,
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

    loop {
        // Build the wire context and request the provider stream.
        let snap = deps.state.snapshot();
        let model = snap.model.clone();
        let ctx = AgentContext {
            system_prompt: snap.system_prompt.clone(),
            messages: snap.messages.clone(),
            tools: snap.tools.clone(),
        };
        let wire: Vec<WireMessage> = default_convert_to_llm(&ctx.messages);

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
            stop_reason: None,
            model: model.id.clone(),
            timestamp: 0,
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

        match plan {
            TurnPlan::ToolBatch { calls } => {
                let outcome = deps
                    .tool_executor
                    .execute(
                        assistant_for_event,
                        calls,
                        deps.tool_execution_mode,
                        deps.hooks.clone(),
                    )
                    .await;
                let ToolExecOutcome {
                    tool_results,
                    events,
                } = match outcome {
                    crate::tools::ToolOutcome::Completed {
                        tool_results,
                        events,
                        ..
                    } => ToolExecOutcome {
                        tool_results,
                        events,
                    },
                    crate::tools::ToolOutcome::Aborted { reason } => {
                        deps.state.set_error(Some(reason)).await;
                        break;
                    }
                };

                for event in events {
                    deps.bus.publish(event);
                }

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

        if !any_injected {
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
        AssistantMessageEvent::Done {
            stop_reason,
            usage: _,
        } => {
            assistant.stop_reason = Some(stop_reason);
        }
        AssistantMessageEvent::Error { error } => {
            assistant.stop_reason = Some(StopReason::Error);
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
            }),
        })
        .collect()
}

#[allow(dead_code)]
fn _tools_marker(_t: &[Arc<dyn AgentTool>]) {}
