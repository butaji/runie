//! Single pattern: direct execution — one agent handles the task end to end.

use std::time::{Duration, Instant};

use chrono::Utc;

use crate::{
    model_for, AgentTrace, Context, Pattern, PatternOutput, TerminationReason, TraceEvent,
    WorkerTask,
};

/// Direct execution — one agent handles the task end to end.
pub struct SinglePattern;

#[async_trait::async_trait]
impl Pattern for SinglePattern {
    fn name(&self) -> &'static str {
        "single"
    }

    fn description(&self) -> &str {
        "Direct execution — one agent handles the task end to end"
    }

    async fn execute(&self, ctx: &Context, input: &str) -> anyhow::Result<PatternOutput> {
        let start_time = Utc::now();
        let start = Instant::now();

        // A pre-aborted context must never start the worker.
        let (result, termination) = if ctx.abort.is_cancelled() {
            (String::new(), TerminationReason::Error("aborted".into()))
        } else {
            let (provider, model) = model_for(&ctx.models, 0);
            let task = WorkerTask {
                id: "leader".into(),
                prompt: input.to_string(),
                provider,
                model,
                read_only: false,
            };

            // Race the runner against abort and the per-task timeout;
            // whichever fires first decides the termination reason.
            let timeout = Duration::from_millis(ctx.config.timeout_ms);
            let run = ctx.runner.run(task);
            tokio::pin!(run);
            tokio::select! {
                () = ctx.abort.cancelled() => {
                    (String::new(), TerminationReason::Error("aborted".into()))
                }
                outcome = tokio::time::timeout(timeout, &mut run) => match outcome {
                    Ok(Ok(text)) => (text, TerminationReason::Completed),
                    Ok(Err(err)) => (String::new(), TerminationReason::Error(err.to_string())),
                    Err(_) => (String::new(), TerminationReason::Timeout),
                },
            }
        };

        let trace = AgentTrace {
            agent_id: "leader".into(),
            start_time,
            duration_ms: start.elapsed().as_millis() as u64,
            events: vec![TraceEvent::Termination {
                reason: termination.clone(),
            }],
        };
        // Observers may have gone away; a failed send must not fail the run.
        let _ = ctx.trace_tx.send(trace.clone());

        Ok(PatternOutput {
            result,
            termination,
            traces: vec![trace],
        })
    }
}
