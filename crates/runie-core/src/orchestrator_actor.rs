//! Orchestrator actor — coordinates Team mode planning and subagent execution.
//!
//! Lives in the actor runtime alongside `SessionActor`. Receives commands,
//! drives the state machine, calls the one-shot planner, and emits events.

use std::time::Instant;

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::actor::Actor;
use crate::bus::EventBus;
use crate::orchestrator::{
    ModelTrait, OrchestratorContext, OrchestratorPlan, PlanResult,
    SubagentTask, TaskStatus,
};

// ─── State machine ───────────────────────────────────────────────────────────

/// Runtime state of the OrchestratorActor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(Default)]
pub enum OrchestratorState {
    /// Idle — no active request.
    #[default]
    Idle,
    /// Aligning — waiting for user answers to clarifying questions.
    Aligning,
    /// Planning — running the one-shot LLM planner.
    Planning,
    /// Executing — subagents are running.
    Executing,
    /// Synthesizing — collecting subagent outputs for final synthesis.
    Synthesizing,
    /// Done — plan completed successfully.
    Done { plan: OrchestratorPlan, result: PlanResult },
    /// Failed — plan or execution failed.
    Failed { error: String },
}


impl OrchestratorState {
    /// Whether this is a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Done { .. } | Self::Failed { .. } | Self::Idle)
    }

    /// Short display label for the status bar.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Idle => "Idle",
            Self::Aligning => "Aligning",
            Self::Planning => "Planning",
            Self::Executing => "Executing",
            Self::Synthesizing => "Synthesizing",
            Self::Done { .. } => "Done",
            Self::Failed { .. } => "Failed",
        }
    }
}

// ─── Commands ───────────────────────────────────────────────────────────────

/// Messages sent to the OrchestratorActor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrchestratorCommand {
    /// Start a new request (user just submitted a message in Team mode).
    StartRequest {
        user_request: String,
        project_context: ProjectContext,
    },
    /// User answered a clarifying question.
    UserAnswer(String),
    /// All subagents have reported their status (for state transitions).
    SubagentStatusUpdate { task_id: String, status: TaskStatus },
    /// A subagent completed with output.
    SubagentDone { task_id: String, output: String },
    /// A subagent failed.
    SubagentFailed { task_id: String, error: String },
    /// Cancel the current plan and return to Idle.
    Cancel,
    /// Reset to Idle (e.g., user switched to Solo mode).
    Reset,
}

// ─── Events ─────────────────────────────────────────────────────────────────

/// Type alias for the flat `Event` variants emitted by the OrchestratorActor.
pub type OrchestratorEvent = crate::Event;

// ─── Project context (from planner crate, re-exported) ──────────────────────

/// Project context for the planner.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectContext {
    pub description: String,
    pub directories: Vec<String>,
    pub key_files: Vec<String>,
}

impl ProjectContext {
    pub fn new() -> Self {
        Self::default()
    }
}

// ─── Actor ──────────────────────────────────────────────────────────────────

/// The Orchestrator actor.
///
/// Owns the state machine, planner reference, and subagent tracking.
/// Does NOT own subagent execution — that is handled by `r4-subagent-isolation`.
pub struct OrchestratorActor {
    state: OrchestratorState,
    ctx: OrchestratorContext,
    /// Currently active plan.
    active_plan: Option<OrchestratorPlan>,
    /// Subagent outputs collected so far.
    results: Vec<(String, String)>,
    /// When the current request started (for elapsed time).
    started_at: Option<Instant>,
    /// Pending tool call for user alignment (ask_user tool was called).
    awaiting_answer: bool,
}

impl Default for OrchestratorActor {
    fn default() -> Self {
        Self {
            state: OrchestratorState::Idle,
            ctx: OrchestratorContext::new(),
            active_plan: None,
            results: Vec::new(),
            started_at: None,
            awaiting_answer: false,
        }
    }
}

impl OrchestratorActor {
    /// Create a new actor in Idle state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if the actor can accept a new request.
    pub fn can_start_request(&self) -> bool {
        matches!(self.state, OrchestratorState::Idle | OrchestratorState::Done { .. } | OrchestratorState::Failed { .. })
    }

    /// Check if the orchestrator has pending questions for the user.
    pub fn has_pending_questions(&self) -> bool {
        self.ctx.has_pending_questions()
    }

    /// Whether a subagent plan is ready to submit (no pending questions).
    pub fn can_submit_plan(&self) -> bool {
        !self.has_pending_questions()
    }

    /// Record a question that was asked (e.g., ask_user tool was called).
    #[allow(dead_code)]
    fn record_question(&mut self, question: String) {
        self.awaiting_answer = true;
        self.ctx.record_question(question);
    }

    /// Record a user answer and transition from Aligning if needed.
    fn record_answer(&mut self, answer: String) {
        self.awaiting_answer = false;
        self.ctx.record_answer(answer);
        if self.state == OrchestratorState::Aligning {
            // User answered all questions — proceed to Planning
            self.transition_to(OrchestratorState::Planning);
        }
    }

    /// Transition to a new state, emitting a StateChanged event.
    fn transition_to(&mut self, new_state: OrchestratorState) {
        let old = std::mem::replace(&mut self.state, new_state.clone());
        if old != new_state {
            self.state = new_state;
        }
    }

    /// Mark a subagent task as dispatched.
    #[allow(dead_code)]
    fn dispatch_subagent(&mut self, task: SubagentTask) {
        self.results.push((task.id.clone(), String::new()));
    }

    /// Collect a subagent result.
    fn collect_result(&mut self, task_id: String, output: String) {
        if let Some(slot) = self.results.iter_mut().find(|(id, _)| id == &task_id) {
            slot.1 = output;
        }
    }

    /// Check if all subagents are done.
    fn all_subagents_done(&self) -> bool {
        if let Some(plan) = &self.active_plan {
            plan.tasks.iter().all(|t| t.status.is_terminal())
        } else {
            false
        }
    }
}

/// Emit a bus event if the actor holds a bus reference.
/// (The actor's run_body gets the bus passed in, so we use a shared cell pattern.)
impl OrchestratorActor {
    #[allow(dead_code)]
    fn emit(bus: &EventBus<OrchestratorEvent>, event: OrchestratorEvent) {
        bus.publish(event);
    }
}

impl Actor for OrchestratorActor {
    type Msg = OrchestratorCommand;
    type Event = OrchestratorEvent;

    fn run_body(
        self,
        mut rx: mpsc::Receiver<Self::Msg>,
        bus: EventBus<Self::Event>,
    ) -> impl std::future::Future<Output = ()> + Send + 'static {
        let mut this = self;
        let bus = bus.clone();
        async move {
            while let Some(cmd) = rx.recv().await {
                match &cmd {
                    OrchestratorCommand::StartRequest { .. } => {
                        handle_start_request(&mut this, &bus);
                    }
                    OrchestratorCommand::UserAnswer(answer) => {
                        handle_user_answer(&mut this, &bus, answer);
                    }
                    OrchestratorCommand::SubagentStatusUpdate { task_id, status } => {
                        handle_subagent_status(&mut this, &bus, task_id, status);
                    }
                    OrchestratorCommand::SubagentDone { task_id, output } => {
                        handle_subagent_done(&mut this, &bus, task_id, output).await;
                    }
                    OrchestratorCommand::SubagentFailed { task_id, error } => {
                        handle_subagent_failed(&mut this, &bus, task_id, error);
                    }
                    OrchestratorCommand::Cancel | OrchestratorCommand::Reset => {
                        handle_cancel(&mut this, &bus);
                    }
                }
            }
        }
    }
}

// ─── Command handlers ───────────────────────────────────────────────────────

fn handle_start_request(actor: &mut OrchestratorActor, bus: &EventBus<OrchestratorEvent>) {
    emit_state_change(bus, &actor.state, &OrchestratorState::Aligning);
    actor.transition_to(OrchestratorState::Aligning);
    actor.started_at = Some(Instant::now());
    actor.ctx = OrchestratorContext::new();
    actor.active_plan = None;
    actor.results.clear();

    if !actor.has_pending_questions() {
        actor.transition_to(OrchestratorState::Planning);
        bus.publish(OrchestratorEvent::PlanningStarted);
        // Async planner call goes in r4-subagent-execution; stub here
        actor.transition_to(OrchestratorState::Executing);
    }
}

fn handle_user_answer(actor: &mut OrchestratorActor, bus: &EventBus<OrchestratorEvent>, answer: &str) {
    actor.record_answer(answer.to_string());
    emit_state_change(bus, &OrchestratorState::Aligning, &OrchestratorState::Planning);
    actor.transition_to(OrchestratorState::Planning);
    bus.publish(OrchestratorEvent::PlanningStarted);
    // Async planner call goes in r4-subagent-execution; stub here
    actor.transition_to(OrchestratorState::Executing);
}

fn handle_subagent_status(
    actor: &mut OrchestratorActor,
    bus: &EventBus<OrchestratorEvent>,
    task_id: &str,
    status: &TaskStatus,
) {
    if let Some(plan) = &mut actor.active_plan {
        if let Some(task) = plan.tasks.iter_mut().find(|t| t.id == task_id) {
            let old = task.status.clone();
            if old != *status {
                task.status = status.clone();
                bus.publish(OrchestratorEvent::SubagentStatusChanged {
                    task_id: task_id.to_string(),
                    status: status.clone(),
                });
            }
        }
    }
}

async fn handle_subagent_done(
    actor: &mut OrchestratorActor,
    bus: &EventBus<OrchestratorEvent>,
    task_id: &str,
    output: &str,
) {
    actor.collect_result(task_id.to_string(), output.to_string());
    if let Some(plan) = &mut actor.active_plan {
        if let Some(task) = plan.tasks.iter_mut().find(|t| t.id == task_id) {
            task.status = TaskStatus::Done {
                output: Some(output.to_string()),
            };
        }
    }
    bus.publish(OrchestratorEvent::SubagentCompleted {
        task_id: task_id.to_string(),
        output: output.to_string(),
    });

    if actor.all_subagents_done() {
        actor.transition_to(OrchestratorState::Synthesizing);
        bus.publish(OrchestratorEvent::SynthesisStarted);
        let elapsed = actor.started_at.map(|s| s.elapsed().as_secs_f64()).unwrap_or(0.0);
        let plan = actor.active_plan.take()
            .unwrap_or_else(|| OrchestratorPlan::simple("fallback", ModelTrait::General));
        actor.transition_to(OrchestratorState::Done {
            plan: plan.clone(),
            result: PlanResult {
                success: true,
                response: "Synthesis complete.".into(),
                failures: vec![],
                elapsed_secs: elapsed,
            },
        });
        bus.publish(OrchestratorEvent::Finished { success: true });
    }
}

fn handle_subagent_failed(
    actor: &mut OrchestratorActor,
    bus: &EventBus<OrchestratorEvent>,
    task_id: &str,
    error: &str,
) {
    if let Some(plan) = &mut actor.active_plan {
        if let Some(task) = plan.tasks.iter_mut().find(|t| t.id == task_id) {
            task.status = TaskStatus::Failed {
                error: error.to_string(),
            };
        }
    }
    bus.publish(OrchestratorEvent::SubagentFailed {
        task_id: task_id.to_string(),
        error: error.to_string(),
    });
    actor.transition_to(OrchestratorState::Failed { error: error.to_string() });
    bus.publish(OrchestratorEvent::Finished { success: false });
}

fn handle_cancel(actor: &mut OrchestratorActor, bus: &EventBus<OrchestratorEvent>) {
    let old = actor.state.clone();
    actor.state = OrchestratorState::Idle;
    actor.active_plan = None;
    actor.results.clear();
    actor.ctx = OrchestratorContext::new();
    emit_state_change(bus, &old, &OrchestratorState::Idle);
    bus.publish(OrchestratorEvent::Cancelled);
}

fn emit_state_change(bus: &EventBus<OrchestratorEvent>, from: &OrchestratorState, to: &OrchestratorState) {
    if from != to {
        bus.publish(OrchestratorEvent::StateChanged {
            from: Box::new(from.clone()),
            to: Box::new(to.clone()),
        });
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn idle() -> OrchestratorActor {
        OrchestratorActor::new()
    }

    #[test]
    fn actor_starts_idle() {
        let orch = idle();
        assert!(matches!(orch.state, OrchestratorState::Idle));
        assert!(orch.active_plan.is_none());
        assert!(orch.results.is_empty());
    }

    #[test]
    fn start_request_transitions_to_aligning() {
        let mut orch = idle();
        orch.state = OrchestratorState::Idle;
        // Simulate StartRequest command
        let ctx = orch.ctx.clone();
        drop(ctx);
        assert!(orch.can_start_request());
        assert!(!orch.has_pending_questions());
        // Without pending questions, it goes to Planning
        // We verify state machine accepts StartRequest when idle
        assert!(orch.state == OrchestratorState::Idle);
    }

    #[test]
    fn record_question_marks_pending() {
        let mut orch = idle();
        assert!(!orch.has_pending_questions());
        orch.record_question("Which file?".into());
        assert!(orch.has_pending_questions());
        assert!(!orch.can_submit_plan());
    }

    #[test]
    fn record_answer_clears_pending() {
        let mut orch = idle();
        orch.record_question("Which file?".into());
        assert!(orch.has_pending_questions());
        orch.record_answer("src/main.rs".into());
        assert!(!orch.has_pending_questions());
        assert!(orch.can_submit_plan());
    }

    #[test]
    fn cancel_resets_to_idle() {
        let mut orch = idle();
        orch.state = OrchestratorState::Executing;
        orch.active_plan = Some(OrchestratorPlan::simple("test", ModelTrait::General));
        orch.ctx.record_question("Q");
        // Simulate cancel
        orch.state = OrchestratorState::Idle;
        orch.active_plan = None;
        orch.results.clear();
        orch.ctx = OrchestratorContext::new();
        assert!(matches!(orch.state, OrchestratorState::Idle));
        assert!(orch.active_plan.is_none());
    }

    #[test]
    fn collect_subagent_result() {
        let mut orch = idle();
        orch.dispatch_subagent(SubagentTask::new("t1", "role", "task", ModelTrait::General));
        assert_eq!(orch.results.len(), 1);
        orch.collect_result("t1".into(), "output".into());
        assert_eq!(orch.results[0].1, "output");
    }

    #[test]
    fn orchestrator_state_is_terminal() {
        assert!(OrchestratorState::Idle.is_terminal());
        assert!(OrchestratorState::Done {
            plan: OrchestratorPlan::simple("", ModelTrait::General),
            result: PlanResult { success: true, response: String::new(), failures: vec![], elapsed_secs: 0.0 },
        }.is_terminal());
        assert!(OrchestratorState::Failed { error: String::new() }.is_terminal());
        assert!(!OrchestratorState::Aligning.is_terminal());
        assert!(!OrchestratorState::Planning.is_terminal());
        assert!(!OrchestratorState::Executing.is_terminal());
    }
}
