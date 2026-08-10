//! Pure Pi-compatible model catalog and scoped-model selection semantics.
//! Refreshing the catalog and committing a selection belong to an owning
//! use from actors, YAML replay, and the TUI's declarative selector.

use crate::task_owner::{mailbox_call, spawn_actor_worker, TaskOwner};
use crate::types::Model;
use std::sync::Arc;
use tokio::sync::{mpsc, watch};

#[path = "model_catalog_actor.rs"]
mod model_catalog_actor;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CycleDirection {
    Forward,
    Backward,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopedModel {
    pub model: Model,
    pub thinking_level: Option<crate::types::ThinkingLevel>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelCatalog {
    pub available: Vec<Model>,
    pub scoped: Vec<ScopedModel>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)]
pub enum ModelCatalogEvent {
    CatalogLoaded { count: usize },
    ScopeChanged { count: usize },
    SearchChanged { query: String, result_count: usize },
    SelectionChanged { model: Option<Model> },
    RefreshFailed { message: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelCatalogSnapshot {
    pub catalog: ModelCatalog,
    pub query: String,
    pub scoped_only: bool,
    pub results: Vec<Model>,
    pub selected: Option<Model>,
    pub last_event: Option<ModelCatalogEvent>,
}

impl Default for ModelCatalogSnapshot {
    fn default() -> Self {
        let catalog = ModelCatalog::new(Vec::new(), Vec::new());
        Self {
            catalog,
            query: String::new(),
            scoped_only: false,
            results: Vec::new(),
            selected: None,
            last_event: None,
        }
    }
}

#[allow(clippy::large_enum_variant)]
enum ModelCatalogCommand {
    Load(Vec<Model>, mpsc::Sender<()>),
    Refresh(Result<Vec<Model>, String>, mpsc::Sender<()>),
    SetScope(Vec<ScopedModel>, mpsc::Sender<()>),
    Search(String, bool, mpsc::Sender<()>),
    Select(Model, mpsc::Sender<Option<Model>>),
    Cycle(CycleDirection, mpsc::Sender<Option<Model>>),
}

/// SSOT actor for model catalog and selector state. Transport refreshers send
/// immutable catalog results here; TUI consumers read the acknowledged watch
/// snapshot and never mutate catalog state directly.
#[derive(Clone)]
pub struct ModelCatalogActor {
    tx: mpsc::Sender<ModelCatalogCommand>,
    snapshot: watch::Receiver<ModelCatalogSnapshot>,
    shared_snapshot: watch::Receiver<crate::SharedSnapshot<ModelCatalogSnapshot>>,
    _worker: Arc<TaskOwner>,
}

async fn run_model_catalog_worker(
    mut rx: mpsc::Receiver<ModelCatalogCommand>,
    snapshot_tx: watch::Sender<ModelCatalogSnapshot>,
    shared_tx: watch::Sender<crate::SharedSnapshot<ModelCatalogSnapshot>>,
) {
    let mut snapshot = ModelCatalogSnapshot::default();
    while let Some(command) = rx.recv().await {
        let (event, reply) = reduce_catalog_command(&mut snapshot, command);
        snapshot.last_event = Some(event);
        crate::publish_shared_snapshot(&snapshot_tx, &shared_tx, snapshot.clone());
        match reply {
            Either::Unit(reply) => {
                let _ = reply.send(()).await;
            }
            Either::Model(reply, model) => {
                let _ = reply.send(model).await;
            }
        }
    }
}

fn reduce_catalog_command(
    snapshot: &mut ModelCatalogSnapshot,
    command: ModelCatalogCommand,
) -> (ModelCatalogEvent, Either) {
    match command {
        ModelCatalogCommand::Load(models, reply) => reduce_load(snapshot, models, reply),
        ModelCatalogCommand::Refresh(result, reply) => reduce_refresh(snapshot, result, reply),
        ModelCatalogCommand::SetScope(models, reply) => reduce_scope(snapshot, models, reply),
        ModelCatalogCommand::Search(query, scoped_only, reply) => {
            reduce_search(snapshot, query, scoped_only, reply)
        }
        ModelCatalogCommand::Cycle(direction, reply) => reduce_cycle(snapshot, direction, reply),
        ModelCatalogCommand::Select(model, reply) => reduce_select(snapshot, model, reply),
    }
}

fn refresh_results(snapshot: &mut ModelCatalogSnapshot) {
    snapshot.results = snapshot
        .catalog
        .search(&snapshot.query, snapshot.scoped_only);
}

fn reduce_load(
    snapshot: &mut ModelCatalogSnapshot,
    models: Vec<Model>,
    reply: mpsc::Sender<()>,
) -> (ModelCatalogEvent, Either) {
    snapshot.catalog.available = models;
    refresh_results(snapshot);
    (
        ModelCatalogEvent::CatalogLoaded {
            count: snapshot.catalog.available.len(),
        },
        Either::Unit(reply),
    )
}

fn reduce_refresh(
    snapshot: &mut ModelCatalogSnapshot,
    result: Result<Vec<Model>, String>,
    reply: mpsc::Sender<()>,
) -> (ModelCatalogEvent, Either) {
    match result {
        Ok(models) => reduce_load(snapshot, models, reply),
        Err(message) => (
            ModelCatalogEvent::RefreshFailed { message },
            Either::Unit(reply),
        ),
    }
}

fn reduce_scope(
    snapshot: &mut ModelCatalogSnapshot,
    models: Vec<ScopedModel>,
    reply: mpsc::Sender<()>,
) -> (ModelCatalogEvent, Either) {
    snapshot.catalog.scoped = models;
    refresh_results(snapshot);
    (
        ModelCatalogEvent::ScopeChanged {
            count: snapshot.catalog.scoped.len(),
        },
        Either::Unit(reply),
    )
}

fn reduce_search(
    snapshot: &mut ModelCatalogSnapshot,
    query: String,
    scoped_only: bool,
    reply: mpsc::Sender<()>,
) -> (ModelCatalogEvent, Either) {
    snapshot.query = query;
    snapshot.scoped_only = scoped_only;
    refresh_results(snapshot);
    (
        ModelCatalogEvent::SearchChanged {
            query: snapshot.query.clone(),
            result_count: snapshot.results.len(),
        },
        Either::Unit(reply),
    )
}

fn reduce_cycle(
    snapshot: &mut ModelCatalogSnapshot,
    direction: CycleDirection,
    reply: mpsc::Sender<Option<Model>>,
) -> (ModelCatalogEvent, Either) {
    snapshot.selected = snapshot
        .catalog
        .cycle(snapshot.selected.as_ref(), direction)
        .map(|item| item.model);
    let selected = snapshot.selected.clone();
    (
        ModelCatalogEvent::SelectionChanged {
            model: selected.clone(),
        },
        Either::Model(reply, selected),
    )
}

fn reduce_select(
    snapshot: &mut ModelCatalogSnapshot,
    model: Model,
    reply: mpsc::Sender<Option<Model>>,
) -> (ModelCatalogEvent, Either) {
    snapshot.selected = Some(model.clone());
    (
        ModelCatalogEvent::SelectionChanged {
            model: Some(model.clone()),
        },
        Either::Model(reply, Some(model)),
    )
}

#[allow(clippy::large_enum_variant)]
enum Either {
    Unit(mpsc::Sender<()>),
    Model(mpsc::Sender<Option<Model>>, Option<Model>),
}

impl ModelCatalog {
    pub fn new(available: Vec<Model>, scoped: Vec<ScopedModel>) -> Self {
        Self { available, scoped }
    }

    /// Pi's selector searches provider/id/name, preserving catalog order.
    pub fn search(&self, query: &str, scoped_only: bool) -> Vec<Model> {
        let query = query.trim().to_ascii_lowercase();
        let models: Vec<&Model> = if scoped_only {
            self.scoped
                .iter()
                .filter(|item| self.is_available(&item.model))
                .map(|item| &item.model)
                .collect()
        } else {
            self.available.iter().collect()
        };
        models
            .into_iter()
            .filter(|model| {
                query.is_empty()
                    || model.id.to_ascii_lowercase().contains(&query)
                    || model.name.to_ascii_lowercase().contains(&query)
                    || model.provider.to_ascii_lowercase().contains(&query)
            })
            .cloned()
            .collect()
    }

    /// Cycle through the effective scoped set, or all available models when
    /// no scope is configured. A missing current model starts at the first
    /// model, matching Pi's initial-selection behavior.
    pub fn cycle(&self, current: Option<&Model>, direction: CycleDirection) -> Option<ScopedModel> {
        let candidates: Vec<ScopedModel> = if self.scoped.is_empty() {
            self.available
                .iter()
                .cloned()
                .map(|model| ScopedModel {
                    model,
                    thinking_level: None,
                })
                .collect()
        } else {
            self.scoped
                .iter()
                .filter(|item| self.is_available(&item.model))
                .cloned()
                .collect()
        };
        if candidates.len() <= 1 {
            return None;
        }
        let current_index = current
            .and_then(|model| {
                candidates
                    .iter()
                    .position(|item| same_model(&item.model, model))
            })
            .unwrap_or(0);
        let next = match direction {
            CycleDirection::Forward => (current_index + 1) % candidates.len(),
            CycleDirection::Backward => {
                current_index.checked_sub(1).unwrap_or(candidates.len() - 1)
            }
        };
        Some(candidates[next].clone())
    }

    fn is_available(&self, model: &Model) -> bool {
        self.available
            .iter()
            .any(|candidate| same_model(candidate, model))
    }
}

fn same_model(left: &Model, right: &Model) -> bool {
    left.provider == right.provider && left.id == right.id
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model(provider: &str, id: &str, name: &str) -> Model {
        Model {
            provider: provider.into(),
            id: id.into(),
            name: name.into(),
            ..Model::default()
        }
    }

    #[test]
    fn search_is_case_insensitive_and_preserves_catalog_order() {
        let catalog = ModelCatalog::new(
            vec![
                model("openai", "gpt-5", "GPT Five"),
                model("xai", "grok", "Grok"),
            ],
            vec![],
        );
        assert_eq!(
            catalog
                .search("GPT", false)
                .into_iter()
                .map(|model| model.id)
                .collect::<Vec<_>>(),
            vec!["gpt-5"]
        );
    }

    #[test]
    fn cycle_wraps_and_drops_unavailable_scoped_models() {
        let gpt = model("openai", "gpt-5", "GPT Five");
        let grok = model("xai", "grok", "Grok");
        let catalog = ModelCatalog::new(
            vec![gpt.clone(), grok.clone()],
            vec![
                ScopedModel {
                    model: gpt.clone(),
                    thinking_level: Some(crate::types::ThinkingLevel::High),
                },
                ScopedModel {
                    model: model("missing", "gone", "Gone"),
                    thinking_level: None,
                },
                ScopedModel {
                    model: grok.clone(),
                    thinking_level: None,
                },
            ],
        );
        assert_eq!(
            catalog
                .cycle(Some(&gpt), CycleDirection::Forward)
                .unwrap()
                .model,
            grok
        );
        assert_eq!(
            catalog
                .cycle(Some(&gpt), CycleDirection::Backward)
                .unwrap()
                .model,
            grok
        );
    }

    #[tokio::test]
    async fn actor_publishes_acknowledged_catalog_events_and_selection() {
        let gpt = model("openai", "gpt-5", "GPT Five");
        let grok = model("xai", "grok", "Grok");
        let actor = ModelCatalogActor::new();
        actor.load(vec![gpt.clone(), grok.clone()]).await;
        actor.search("grok".into(), false).await;
        assert_eq!(actor.snapshot().results, vec![grok.clone()]);
        actor
            .set_scope(vec![
                ScopedModel {
                    model: gpt,
                    thinking_level: None,
                },
                ScopedModel {
                    model: grok.clone(),
                    thinking_level: None,
                },
            ])
            .await;
        assert_eq!(actor.snapshot().catalog.scoped.len(), 2);
        assert_eq!(actor.snapshot().catalog.available.len(), 2);
        assert!(actor
            .snapshot()
            .catalog
            .cycle(None, CycleDirection::Forward)
            .is_some());
        let selected = actor.cycle(CycleDirection::Forward).await;
        assert_eq!(selected, Some(grok.clone()));
        assert_eq!(
            actor.snapshot().last_event,
            Some(ModelCatalogEvent::SelectionChanged { model: Some(grok) })
        );
    }

    #[tokio::test]
    async fn actor_shares_immutable_catalog_projection() {
        let actor = ModelCatalogActor::new();
        actor.load(vec![model("xai", "grok", "Grok")]).await;

        let shared = actor.shared_snapshot();
        assert_eq!(shared.get().catalog.available.len(), 1);
        assert_eq!(shared.strong_count(), 2);
        assert_eq!(
            actor
                .shared_subscribe()
                .borrow()
                .get()
                .catalog
                .available
                .len(),
            1
        );
    }

    #[tokio::test]
    async fn failed_refresh_preserves_catalog_and_publishes_typed_failure() {
        let gpt = model("openai", "gpt-5", "GPT Five");
        let actor = ModelCatalogActor::new();
        actor.refresh(Ok(vec![gpt.clone()])).await;
        actor.refresh(Err("catalog unavailable".into())).await;
        let snapshot = actor.snapshot();
        assert_eq!(snapshot.catalog.available, vec![gpt]);
        assert_eq!(
            snapshot.last_event,
            Some(ModelCatalogEvent::RefreshFailed {
                message: "catalog unavailable".into()
            })
        );
    }
}
