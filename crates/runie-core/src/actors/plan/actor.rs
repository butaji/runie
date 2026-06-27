//! PlanActor — owns the plan graph and enforces plan-first execution.

use tokio::sync::mpsc;

use crate::actors::plan::messages::PlanMsg;
use crate::actors::{spawn_actor, Actor, ActorHandle};
use crate::bus::EventBus;
use crate::event::plan::PlanEvent;
use crate::Event;

use super::state::{PlanState, PlanStatus, PlanStepStatus};

/// PlanActor owns the plan graph for plan-first execution mode.
pub struct PlanActor {
    bus: EventBus<Event>,
    state: PlanState,
}

impl PlanActor {
    /// Spawn a new PlanActor.
    pub fn spawn(bus: EventBus<Event>) -> (PlanActorHandle, ActorHandle) {
        let actor = Self {
            bus: bus.clone(),
            state: PlanState::default(),
        };
        let (tx, handle) = spawn_actor(actor, bus);
        (PlanActorHandle::new(tx), handle)
    }

    fn emit(&self, event: PlanEvent) {
        let _ = self.bus.publish(Event::Plan(event));
    }
}

/// Ergonomic handle for sending commands to PlanActor.
#[derive(Clone, Debug)]
pub struct PlanActorHandle {
    tx: std::sync::Arc<mpsc::Sender<PlanMsg>>,
}

impl PlanActorHandle {
    pub fn new(tx: mpsc::Sender<PlanMsg>) -> Self {
        Self { tx: std::sync::Arc::new(tx) }
    }

    /// Send a message to the actor.
    pub async fn send(&self, msg: PlanMsg) {
        let _ = self.tx.send(msg).await;
    }

    /// Try to send a message (non-blocking).
    pub fn try_send(&self, msg: PlanMsg) {
        let _ = self.tx.try_send(msg);
    }
}

impl Actor for PlanActor {
    type Msg = PlanMsg;
    type Event = Event;

    async fn run_body(mut self, mut rx: mpsc::Receiver<Self::Msg>, _bus: EventBus<Event>) {
        while let Some(msg) = rx.recv().await {
            self.handle_msg(msg).await;
        }
    }
}

impl PlanActor {
    async fn handle_msg(&mut self, msg: PlanMsg) {
        match msg {
            PlanMsg::CreatePlan { id, title } => self.handle_create_plan(id, title),
            PlanMsg::AddStep { description, depends_on } => {
                self.handle_add_step(description, depends_on)
            }
            PlanMsg::SubmitPlan => self.handle_submit_plan(),
            PlanMsg::ApprovePlan => self.handle_approve_plan(),
            PlanMsg::RejectPlan => self.handle_reject_plan(),
            PlanMsg::UpdateStep { step_id, status } => {
                self.handle_update_step(step_id, status)
            }
            PlanMsg::StartStep { step_id } => self.handle_start_step(step_id),
            PlanMsg::CompleteStep { step_id } => self.handle_complete_step(step_id),
            PlanMsg::FailStep { step_id, error } => self.handle_fail_step(step_id, error),
            PlanMsg::ClearPlan => self.handle_clear_plan(),
            PlanMsg::CheckStatus => self.handle_check_status(),
        }
    }

    fn handle_create_plan(&mut self, id: String, title: String) {
        self.state = PlanState::new(id.clone(), title.clone());
        self.emit(PlanEvent::PlanCreated {
            id,
            title,
            steps: self.state.steps.clone(),
        });
    }

    fn handle_add_step(&mut self, description: String, depends_on: Vec<usize>) {
        let step_id = self.state.add_step(description.clone(), depends_on.clone());
        self.emit(PlanEvent::PlanChanged {
            plan: self.state.clone(),
        });
        self.emit(PlanEvent::PlanStepAdded {
            id: step_id,
            description,
            depends_on,
        });
    }

    fn handle_submit_plan(&mut self) {
        self.state.status = PlanStatus::PendingApproval;
        self.emit(PlanEvent::PlanChanged {
            plan: self.state.clone(),
        });
        self.emit(PlanEvent::PlanSubmitted);
    }

    fn handle_approve_plan(&mut self) {
        self.state.approve();
        self.emit(PlanEvent::PlanApproved {
            plan: self.state.clone(),
        });
        self.emit(PlanEvent::PlanChanged {
            plan: self.state.clone(),
        });
    }

    fn handle_reject_plan(&mut self) {
        self.state.reject();
        self.emit(PlanEvent::PlanRejected {
            plan: self.state.clone(),
        });
        self.emit(PlanEvent::PlanChanged {
            plan: self.state.clone(),
        });
    }

    fn handle_update_step(&mut self, step_id: usize, status: PlanStepStatus) {
        if self.state.update_step_status(step_id, status.clone()) {
            self.emit(PlanEvent::PlanStepUpdated { step_id, status });
            self.emit(PlanEvent::PlanChanged {
                plan: self.state.clone(),
            });
            self.check_completion();
        }
    }

    fn handle_start_step(&mut self, step_id: usize) {
        self.handle_update_step(step_id, PlanStepStatus::Executing);
    }

    fn handle_complete_step(&mut self, step_id: usize) {
        self.handle_update_step(step_id, PlanStepStatus::Completed);
        self.approve_ready_steps();
    }

    fn handle_fail_step(&mut self, step_id: usize, error: String) {
        self.state.status = PlanStatus::Failed { error: error.clone() };
        self.handle_update_step(step_id, PlanStepStatus::Failed { error });
    }

    fn handle_clear_plan(&mut self) {
        self.state = PlanState::default();
        self.emit(PlanEvent::PlanChanged {
            plan: self.state.clone(),
        });
        self.emit(PlanEvent::PlanCleared);
    }

    fn handle_check_status(&mut self) {
        self.emit(PlanEvent::PlanChanged {
            plan: self.state.clone(),
        });
    }

    /// Approve steps whose dependencies are now met.
    fn approve_ready_steps(&mut self) {
        // Collect ready step IDs first to avoid borrow issues
        let ready_ids: Vec<usize> = self
            .state
            .steps
            .iter()
            .filter(|s| {
                matches!(s.status, PlanStepStatus::Pending) && self.state.can_execute(s.id)
            })
            .map(|s| s.id)
            .collect();

        // Then update each ready step
        for step_id in ready_ids {
            if let Some(step) = self.state.steps.get_mut(step_id) {
                step.status = PlanStepStatus::Approved;
            }
            self.emit(PlanEvent::PlanStepUpdated {
                step_id,
                status: PlanStepStatus::Approved,
            });
        }
    }

    /// Check if all steps are done and finalize the plan.
    fn check_completion(&mut self) {
        if self.state.status == PlanStatus::Approved && self.state.all_steps_complete() {
            self.state.complete();
            self.emit(PlanEvent::PlanCompleted {
                plan: self.state.clone(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (EventBus<Event>, PlanActorHandle) {
        let bus = EventBus::new(16);
        let (handle, _actor) = PlanActor::spawn(bus.clone());
        (bus, handle)
    }

    #[tokio::test]
    async fn create_plan_emits_plan_created() {
        let bus = EventBus::new(16);
        let (handle, _actor) = PlanActor::spawn(bus.clone());
        let mut sub = bus.subscribe();

        handle
            .send(PlanMsg::CreatePlan {
                id: "p1".into(),
                title: "Test plan".into(),
            })
            .await;

        let mut found = false;
        while let Ok(evt) = sub.recv().await {
            if let Event::Plan(PlanEvent::PlanCreated { .. }) = evt {
                found = true;
                break;
            }
        }
        assert!(found, "PlanCreated should be emitted");
    }

    #[tokio::test]
    async fn approve_plan_unblocks_writes() {
        let bus = EventBus::new(16);
        let (handle, _actor) = PlanActor::spawn(bus.clone());
        let mut sub = bus.subscribe();

        handle
            .send(PlanMsg::CreatePlan {
                id: "p1".into(),
                title: "Test".into(),
            })
            .await;
        handle
            .send(PlanMsg::AddStep {
                description: "Step 1".into(),
                depends_on: vec![],
            })
            .await;
        handle.send(PlanMsg::ApprovePlan).await;

        let mut found = false;
        while let Ok(evt) = sub.recv().await {
            if let Event::Plan(PlanEvent::PlanApproved { ref plan }) = evt {
                if !plan.write_blocked {
                    found = true;
                    break;
                }
            }
        }
        assert!(found, "PlanApproved with write_blocked=false should be emitted");
    }

    #[tokio::test]
    async fn reject_plan_blocks_writes() {
        let bus = EventBus::new(16);
        let (handle, _actor) = PlanActor::spawn(bus.clone());
        let mut sub = bus.subscribe();

        handle
            .send(PlanMsg::CreatePlan {
                id: "p1".into(),
                title: "Test".into(),
            })
            .await;
        handle
            .send(PlanMsg::AddStep {
                description: "Step 1".into(),
                depends_on: vec![],
            })
            .await;
        handle.send(PlanMsg::ApprovePlan).await;
        handle.send(PlanMsg::RejectPlan).await;

        let mut found = false;
        while let Ok(evt) = sub.recv().await {
            if let Event::Plan(PlanEvent::PlanRejected { ref plan }) = evt {
                if plan.write_blocked {
                    found = true;
                    break;
                }
            }
        }
        assert!(found, "PlanRejected with write_blocked=true should be emitted");
    }

    #[tokio::test]
    async fn complete_step_emits_plan_step_updated() {
        let bus = EventBus::new(16);
        let (handle, _actor) = PlanActor::spawn(bus.clone());
        let mut sub = bus.subscribe();

        handle
            .send(PlanMsg::CreatePlan {
                id: "p1".into(),
                title: "Test".into(),
            })
            .await;
        handle
            .send(PlanMsg::AddStep {
                description: "Step 1".into(),
                depends_on: vec![],
            })
            .await;
        handle.send(PlanMsg::ApprovePlan).await;
        handle.send(PlanMsg::CompleteStep { step_id: 0 }).await;

        let mut found = false;
        while let Ok(evt) = sub.recv().await {
            if let Event::Plan(PlanEvent::PlanStepUpdated {
                step_id,
                status: PlanStepStatus::Completed,
            }) = evt
            {
                if step_id == 0 {
                    found = true;
                    break;
                }
            }
        }
        assert!(found, "PlanStepUpdated with Completed status should be emitted");
    }

    #[tokio::test]
    async fn clear_plan_resets_state() {
        let bus = EventBus::new(16);
        let (handle, _actor) = PlanActor::spawn(bus.clone());
        let mut sub = bus.subscribe();

        handle
            .send(PlanMsg::CreatePlan {
                id: "p1".into(),
                title: "Test".into(),
            })
            .await;
        handle
            .send(PlanMsg::AddStep {
                description: "Step 1".into(),
                depends_on: vec![],
            })
            .await;
        handle.send(PlanMsg::ClearPlan).await;

        let mut found = false;
        while let Ok(evt) = sub.recv().await {
            if let Event::Plan(PlanEvent::PlanCleared) = evt {
                found = true;
                break;
            }
        }
        assert!(found, "PlanCleared should be emitted");
    }

    #[tokio::test]
    async fn all_steps_complete_triggers_plan_completed() {
        let bus = EventBus::new(16);
        let (handle, _actor) = PlanActor::spawn(bus.clone());
        let mut sub = bus.subscribe();

        handle
            .send(PlanMsg::CreatePlan {
                id: "p1".into(),
                title: "Test".into(),
            })
            .await;
        handle
            .send(PlanMsg::AddStep {
                description: "Step 1".into(),
                depends_on: vec![],
            })
            .await;
        handle.send(PlanMsg::ApprovePlan).await;
        handle.send(PlanMsg::CompleteStep { step_id: 0 }).await;

        let mut found = false;
        while let Ok(evt) = sub.recv().await {
            if let Event::Plan(PlanEvent::PlanCompleted { .. }) = evt {
                found = true;
                break;
            }
        }
        assert!(found, "PlanCompleted should be emitted when all steps complete");
    }
}
