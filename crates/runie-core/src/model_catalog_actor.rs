use super::*;
use crate::SharedSnapshot;

impl ModelCatalogActor {
    pub fn new() -> Self {
        let (snapshot_tx, snapshot) = watch::channel(ModelCatalogSnapshot::default());
        let (shared_tx, shared_snapshot) =
            watch::channel(SharedSnapshot::new(ModelCatalogSnapshot::default()));
        let (tx, worker) = spawn_actor_worker!(256, move |rx| async move {
            run_model_catalog_worker(rx, snapshot_tx, shared_tx).await;
        });
        Self {
            tx,
            snapshot,
            shared_snapshot,
            _worker: worker,
        }
    }
    pub async fn load(&self, models: Vec<Model>) {
        mailbox_call!(
            self.tx,
            |reply| ModelCatalogCommand::Load(models, reply),
            ()
        );
    }
    pub async fn refresh(&self, result: Result<Vec<Model>, String>) {
        mailbox_call!(
            self.tx,
            |reply| ModelCatalogCommand::Refresh(result, reply),
            ()
        );
    }
    pub async fn set_scope(&self, models: Vec<ScopedModel>) {
        mailbox_call!(
            self.tx,
            |reply| ModelCatalogCommand::SetScope(models, reply),
            ()
        );
    }
    pub async fn search(&self, query: String, scoped_only: bool) {
        mailbox_call!(
            self.tx,
            |reply| ModelCatalogCommand::Search(query, scoped_only, reply),
            ()
        );
    }
    pub async fn cycle(&self, direction: CycleDirection) -> Option<Model> {
        mailbox_call!(
            self.tx,
            |reply| ModelCatalogCommand::Cycle(direction, reply),
            None
        )
    }
    pub async fn select(&self, model: Model) -> Option<Model> {
        mailbox_call!(
            self.tx,
            |reply| ModelCatalogCommand::Select(model, reply),
            None
        )
    }
    pub fn snapshot(&self) -> ModelCatalogSnapshot {
        self.snapshot.borrow().clone()
    }
    pub fn subscribe(&self) -> watch::Receiver<ModelCatalogSnapshot> {
        self.snapshot.clone()
    }
    pub fn shared_snapshot(&self) -> SharedSnapshot<ModelCatalogSnapshot> {
        self.shared_snapshot.borrow().clone()
    }
    pub fn shared_subscribe(&self) -> watch::Receiver<SharedSnapshot<ModelCatalogSnapshot>> {
        self.shared_snapshot.clone()
    }
}

impl Default for ModelCatalogActor {
    fn default() -> Self {
        Self::new()
    }
}
