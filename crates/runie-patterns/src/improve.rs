//! Improve pattern: iterative improvement with review (PATTERNS.md Phase 3).
//!
//! A leader agent generates a draft, reviews it, and revises until the
//! reviewer replies `APPROVED` or `max_rounds` is exhausted. Every runner
//! call goes through the leader model (`model_for(models, 0)`), is read-only,
//! and is raced against `Context::abort` and the per-call timeout.
//!
//! # Cancellation contract
//!
//! Abort is honored between calls and during calls (via `tokio::select!`);
//! the pattern returns `TerminationReason::Error("aborted")` with the traces
//! and draft collected so far.

use std::time::{Duration, Instant};

use chrono::Utc;

use crate::{
    model_for, AgentTrace, Context, Pattern, PatternOutput, TerminationReason, TraceEvent,
    WorkerTask,
};

/// Iterative improvement with review — generate, evaluate, revise.
pub struct ImprovePattern;

#[async_trait::async_trait]
impl Pattern for ImprovePattern {
    fn name(&self) -> &'static str {
        "improve"
    }

    fn description(&self) -> &str {
        "Iterative improvement with review"
    }

    async fn execute(&self, ctx: &Context, input: &str) -> anyhow::Result<PatternOutput> {
        let max_rounds = ctx.config.max_rounds.max(1);
        let mut traces = Vec::new();
        let mut draft = String::new();
        let mut feedback = String::new();

        for round in 1..=max_rounds {
            if ctx.abort.is_cancelled() {
                return Ok(finish(traces, draft, aborted()));
            }
            match eval_round(ctx, round, input, &draft, &feedback, &mut traces).await {
                RoundVerdict::Approved(final_draft) => {
                    return Ok(finish(traces, final_draft, TerminationReason::Approved));
                }
                RoundVerdict::Revise {
                    draft: new_draft,
                    feedback: new_feedback,
                } => {
                    draft = new_draft;
                    feedback = new_feedback;
                }
                RoundVerdict::Stop {
                    draft: partial,
                    reason,
                } => {
                    return Ok(finish(traces, partial, reason));
                }
            }
        }
        Ok(finish(traces, draft, TerminationReason::MaxRoundsReached))
    }
}

/// Outcome of one generate/revise + review iteration.
enum RoundVerdict {
    /// Reviewer approved; carries the final draft.
    Approved(String),
    /// Reviewer gave feedback; carries the new draft and the feedback.
    Revise { draft: String, feedback: String },
    /// Fatal: the pattern terminates with this reason and partial draft.
    Stop {
        draft: String,
        reason: TerminationReason,
    },
}

/// Whether the draft step of a round generates or revises.
enum DraftStep {
    Generate,
    Revise,
}

/// Outcome of one eval runner call.
enum EvalCall {
    Text(String),
    Failed(String),
    Aborted,
}

fn aborted() -> TerminationReason {
    TerminationReason::Error("aborted".into())
}

fn finish(
    traces: Vec<AgentTrace>,
    result: String,
    termination: TerminationReason,
) -> PatternOutput {
    PatternOutput {
        result,
        termination,
        traces,
    }
}

/// One iteration: produce/revise the draft, then review it.
async fn eval_round(
    ctx: &Context,
    round: usize,
    input: &str,
    draft: &str,
    feedback: &str,
    traces: &mut Vec<AgentTrace>,
) -> RoundVerdict {
    let step = if round == 1 {
        DraftStep::Generate
    } else {
        DraftStep::Revise
    };
    let draft = match draft_call(ctx, step, round, input, draft, feedback, traces).await {
        EvalCall::Text(text) => text,
        EvalCall::Failed(message) => {
            return RoundVerdict::Stop {
                draft: draft.to_string(),
                reason: TerminationReason::Error(message),
            };
        }
        EvalCall::Aborted => {
            return RoundVerdict::Stop {
                draft: draft.to_string(),
                reason: aborted(),
            };
        }
    };

    if ctx.abort.is_cancelled() {
        return RoundVerdict::Stop {
            draft,
            reason: aborted(),
        };
    }
    let id = format!("improve-review-{round}");
    let description = format!("review round {round}");
    match call_eval(
        ctx,
        &id,
        &description,
        build_review_prompt(input, &draft),
        traces,
    )
    .await
    {
        EvalCall::Text(text) if approved(&text) => RoundVerdict::Approved(draft),
        EvalCall::Text(feedback) => RoundVerdict::Revise { draft, feedback },
        EvalCall::Failed(message) => RoundVerdict::Stop {
            draft,
            reason: TerminationReason::Error(message),
        },
        EvalCall::Aborted => RoundVerdict::Stop {
            draft,
            reason: aborted(),
        },
    }
}

/// The draft step of a round: generate in round 1, revise afterwards.
async fn draft_call(
    ctx: &Context,
    step: DraftStep,
    round: usize,
    input: &str,
    draft: &str,
    feedback: &str,
    traces: &mut Vec<AgentTrace>,
) -> EvalCall {
    let (id, description, prompt) = match step {
        DraftStep::Generate => (
            format!("improve-generate-{round}"),
            "generate".to_string(),
            build_generate_prompt(input),
        ),
        DraftStep::Revise => (
            format!("improve-revise-{round}"),
            format!("revise round {round}"),
            build_revise_prompt(input, draft, feedback),
        ),
    };
    call_eval(ctx, &id, &description, prompt, traces).await
}

/// One leader runner call with abort race, per-call timeout, and trace
/// recording. A failed send on the trace channel must not fail the run.
async fn call_eval(
    ctx: &Context,
    id: &str,
    description: &str,
    prompt: String,
    traces: &mut Vec<AgentTrace>,
) -> EvalCall {
    let (provider, model) = model_for(&ctx.models, 0);
    let task = WorkerTask {
        id: id.to_string(),
        prompt,
        provider,
        model,
        read_only: true,
    };
    let start_time = Utc::now();
    let start = Instant::now();
    let timeout = Duration::from_millis(ctx.config.timeout_ms);

    let run = ctx.runner.run(task);
    tokio::pin!(run);
    let mut events = Vec::new();
    let call = tokio::select! {
        () = ctx.abort.cancelled() => {
            events.push(TraceEvent::Termination { reason: aborted() });
            EvalCall::Aborted
        }
        outcome = tokio::time::timeout(timeout, &mut run) => match outcome {
            Ok(Ok(text)) => {
                events.push(TraceEvent::Termination { reason: TerminationReason::Completed });
                EvalCall::Text(text)
            }
            Ok(Err(error)) => {
                let message = error.to_string();
                events.push(TraceEvent::Error { error: message.clone() });
                events.push(TraceEvent::Termination {
                    reason: TerminationReason::Error(message.clone()),
                });
                EvalCall::Failed(message)
            }
            Err(_) => {
                let message = format!("timed out after {} ms", ctx.config.timeout_ms);
                events.push(TraceEvent::Error { error: message.clone() });
                events.push(TraceEvent::Termination {
                    reason: TerminationReason::Error(message.clone()),
                });
                EvalCall::Failed(message)
            }
        },
    };

    let trace = AgentTrace {
        agent_id: id.to_string(),
        description: description.to_string(),
        output: match &call {
            EvalCall::Text(text) => text.clone(),
            EvalCall::Failed(message) => message.clone(),
            EvalCall::Aborted => String::new(),
        },
        start_time,
        duration_ms: start.elapsed().as_millis() as u64,
        events,
    };
    // Observers may have gone away; a failed send must not fail the run.
    let _ = ctx.trace_tx.send(trace.clone());
    traces.push(trace);
    call
}

/// The reviewer approves by starting its reply with `APPROVED`
/// (trimmed, case-insensitive).
fn approved(response: &str) -> bool {
    response.trim().to_uppercase().starts_with("APPROVED")
}

fn build_generate_prompt(input: &str) -> String {
    format!(
        "[improve-generate]\nProduce the best possible answer to the task below.\n\nTask:\n{input}"
    )
}

fn build_revise_prompt(input: &str, draft: &str, feedback: &str) -> String {
    format!(
        "[improve-revise]\nTask:\n{input}\n\nCurrent draft:\n{draft}\n\nReviewer feedback:\n{feedback}\n\n\
         Revise the draft, addressing all reviewer feedback."
    )
}

fn build_review_prompt(input: &str, draft: &str) -> String {
    format!(
        "[improve-review]\nTask:\n{input}\n\nDraft under review:\n{draft}\n\n\
         Reply with exactly APPROVED if the draft fully satisfies the task; \
         otherwise reply with concise, actionable feedback."
    )
}
