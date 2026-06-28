//! Ractor-based PlanActor implementation.
//!
//! Migrated from custom Actor trait to ractor for consistency with the rest
//! of the actor system.

use std::sync::Mutex;

use ractor::{Actor, ActorProcessingErr, ActorRef};

use crate::actors::ractor_adapter::{spawn_ractor, EventBusBridge};
use crate::bus::EventBus;
use crate::event::plan::PlanEvent;
use crate::Event;

use super::messages::PlanMsg;
use super::state::{PlanState, PlanStatus, PlanStepStatus};

/// Ractor-based PlanActor handle.
#[derive(Clone, Debug)]
pub struct RactorPlanHandle {
    inner: crate::actors::ractor_adapter::RactorHandle<PlanMsg>,
}

impl RactorPlanHandle {
    /// Create a new handle wrapping the inner RactorHandle.
    pub fn new(inner: crate::actors::ractor_adapter::RactorHandle<PlanMsg>) -> Self {
        Self { inner }
    }

    /// Send a message to the actor (fire-and-forget).
    pub async fn send(&self, msg: PlanMsg) {
        let _ = self.inner.send(msg).await;
    }

    /// Try to send a message (non-blocking).
    pub fn try_send(&self, msg: PlanMsg) -> Result<(), ractor::MessagingErr<PlanMsg>> {
        self.inner.try_send(msg)
    }
}

/// PlanActor state for ractor.
pub struct RactorPlanActor {
    /// The authoritative plan state.
    state: Mutex<PlanState>,
    /// Bridge to the event bus for publishing facts.
    bus_bridge: EventBusBridge<Event>,
}

impl RactorPlanActor {
    fn new(bus: EventBus<Event>) -> Self {
        Self {
            state: Mutex::new(PlanState::default()),
            bus_bridge: EventBusBridge::new(bus),
        }
    }

    fn emit(&self, event: PlanEvent) {
        self.bus_bridge.publish(Event::Plan(event));
    }

    fn handle_msg(&self, msg: PlanMsg) {
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

    fn handle_create_plan(&self, id: String, title: String) {
        let mut state = self.state.lock().unwrap();
        *state = PlanState::new(id.clone(), title.clone());
        drop(state);
        self.emit(PlanEvent::PlanCreated {
            id,
            title,
            steps: self.state.lock().unwrap().steps.clone(),
        });
    }

    fn handle_add_step(&self, description: String, depends_on: Vec<usize>) {
        let step_id = {
            let mut state = self.state.lock().unwrap();
            state.add_step(description.clone(), depends_on.clone())
        };
        self.emit(PlanEvent::PlanChanged {
            plan: self.state.lock().unwrap().clone(),
        });
        self.emit(PlanEvent::PlanStepAdded {
            id: step_id,
            description,
            depends_on,
        });
    }

    fn handle_submit_plan(&self) {
        {
            let mut state = self.state.lock().unwrap();
            state.status = PlanStatus::PendingApproval;
        }
        self.emit(PlanEvent::PlanChanged {
            plan: self.state.lock().unwrap().clone(),
        });
        self.emit(PlanEvent::PlanSubmitted);
    }

    fn handle_approve_plan(&self) {
        {
            let mut state = self.state.lock().unwrap();
            state.approve();
            let plan = state.clone();
            drop(state);
            self.emit(PlanEvent::PlanApproved { plan });
        }
        self.emit(PlanEvent::PlanChanged {
            plan: self.state.lock().unwrap().clone(),
        });
    }

    fn handle_reject_plan(&self) {
        {
            let mut state = self.state.lock().unwrap();
            state.reject();
            let plan = state.clone();
            drop(state);
            self.emit(PlanEvent::PlanRejected { plan });
        }
        self.emit(PlanEvent::PlanChanged {
            plan: self.state.lock().unwrap().clone(),
        });
    }

    fn handle_update_step(&self, step_id: usize, status: PlanStepStatus) {
        if {
            let mut state = self.state.lock().unwrap();
            state.update_step_status(step_id, status.clone())
        } {
            self.emit(PlanEvent::PlanStepUpdated { step_id, status });
            self.emit(PlanEvent::PlanChanged {
                plan: self.state.lock().unwrap().clone(),
            });
            self.check_completion();
        }
    }

    fn handle_start_step(&self, step_id: usize) {
        self.handle_update_step(step_id, PlanStepStatus::Executing);
    }

    fn handle_complete_step(&self, step_id: usize) {
        self.handle_update_step(step_id, PlanStepStatus::Completed);
        self.approve_ready_steps();
    }

    fn handle_fail_step(&self, step_id: usize, error: String) {
        {
            let mut state = self.state.lock().unwrap();
            state.status = PlanStatus::Failed { error: error.clone() };
        }
        self.handle_update_step(step_id, PlanStepStatus::Failed { error });
    }

    fn handle_clear_plan(&self) {
        {
            let mut state = self.state.lock().unwrap();
            *state = PlanState::default();
        }
        self.emit(PlanEvent::PlanChanged {
            plan: self.state.lock().unwrap().clone(),
        });
        self.emit(PlanEvent::PlanCleared);
    }

    fn handle_check_status(&self) {
        self.emit(PlanEvent::PlanChanged {
            plan: self.state.lock().unwrap().clone(),
        });
    }

    /// Approve steps whose dependencies are now met.
    fn approve_ready_steps(&self) {
        let ready_ids: Vec<usize> = {
            let state = self.state.lock().unwrap();
            state
                .steps
                .iter()
                .filter(|s| {
                    matches!(s.status, PlanStepStatus::Pending) && state.can_execute(s.id)
                })
                .map(|s| s.id)
                .collect()
        };

        for step_id in ready_ids {
            {
                let mut state = self.state.lock().unwrap();
                if let Some(step) = state.steps.get_mut(step_id) {
                    step.status = PlanStepStatus::Approved;
                }
            }
            self.emit(PlanEvent::PlanStepUpdated {
                step_id,
                status: PlanStepStatus::Approved,
            });
        }
    }

    /// Check if all steps are done and finalize the plan.
    fn check_completion(&self) {
        if {
            let state = self.state.lock().unwrap();
            state.status == PlanStatus::Approved && state.all_steps_complete()
        } {
            {
                let mut state = self.state.lock().unwrap();
                state.complete();
            }
            self.emit(PlanEvent::PlanCompleted {
                plan: self.state.lock().unwrap().clone(),
            });
        }
    }
}

#[ractor::async_trait]
impl Actor for RactorPlanActor {
    type Msg = PlanMsg;
    type State = ();
    type Arguments = EventBus<Event>;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        _args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(())
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        msg: Self::Msg,
        _state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        self.handle_msg(msg);
        Ok(())
    }
}

impl RactorPlanActor {
    /// Spawn a `RactorPlanActor` on the given event bus.
    pub async fn spawn(bus: EventBus<Event>) -> (RactorPlanHandle, ractor::ActorCell) {
        let actor = Self::new(bus.clone());
        let (handle, _join, cell) = spawn_ractor(None, actor, bus).await.unwrap();
        (RactorPlanHandle::new(handle), cell)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn create_plan_emits_plan_created() {
        let bus = EventBus::<Event>::new(16);
        let (handle, _cell) = RactorPlanActor::spawn(bus.clone()).await;
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
        let bus = EventBus::<Event>::new(16);
        let (handle, _cell) = RactorPlanActor::spawn(bus.clone()).await;
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
        let bus = EventBus::<Event>::new(16);
        let (handle, _cell) = RactorPlanActor::spawn(bus.clone()).await;
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
        let bus = EventBus::<Event>::new(16);
        let (handle, _cell) = RactorPlanActor::spawn(bus.clone()).await;
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
        let bus = EventBus::<Event>::new(16);
        let (handle, _cell) = RactorPlanActor::spawn(bus.clone()).await;
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
}
