use super::{PromptActor, PromptSnapshot, PromptWidget};
use tokio::sync::{watch, watch::Sender};

impl PromptActor {
    pub fn shared_model_snapshot(&self) -> runie_core::SharedSnapshot<PromptSnapshot> {
        self.shared_snapshot.borrow().clone()
    }

    pub fn shared_subscribe(&self) -> watch::Receiver<runie_core::SharedSnapshot<PromptSnapshot>> {
        self.shared_snapshot.clone()
    }
}

pub(super) fn publish_prompt_snapshot(
    snapshot_tx: &Sender<PromptSnapshot>,
    shared_tx: &Sender<runie_core::SharedSnapshot<PromptSnapshot>>,
    prompt: &PromptWidget,
) {
    let snapshot = prompt.model_snapshot();
    runie_core::publish_shared_snapshot(snapshot_tx, shared_tx, snapshot);
}
