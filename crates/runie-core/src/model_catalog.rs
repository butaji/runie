//! Pure Pi-compatible model catalog and scoped-model selection semantics.
//!
//! Refreshing the catalog and committing a selection belong to an owning
//! actor. This module only reduces immutable catalog inputs, so it is safe to
//! use from actors, YAML replay, and the TUI's declarative selector.

use crate::task_owner::{mailbox_call, spawn_actor_worker, TaskOwner};
use crate::types::Model;
use std::sync::Arc;
use tokio::sync::{mpsc, watch};

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
    _worker: Arc<TaskOwner>,
}

impl ModelCatalogActor {
    pub fn new() -> Self {
        let (snapshot_tx, snapshot) = watch::channel(ModelCatalogSnapshot::default());
        let (tx, worker) = spawn_actor_worker!(256, move |rx| async move {
            run_model_catalog_worker(rx, snapshot_tx).await;
        });
        Self {
            tx,
            snapshot,
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
}

impl Default for ModelCatalogActor {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(clippy::too_many_lines)]
async fn run_model_catalog_worker(
    mut rx: mpsc::Receiver<ModelCatalogCommand>,
    snapshot_tx: watch::Sender<ModelCatalogSnapshot>,
) {
    let mut snapshot = ModelCatalogSnapshot::default();
    while let Some(command) = rx.recv().await {
        let (event, reply) = match command {
            ModelCatalogCommand::Load(models, reply) => {
                snapshot.catalog.available = models;
                snapshot.results = snapshot
                    .catalog
                    .search(&snapshot.query, snapshot.scoped_only);
                (
                    ModelCatalogEvent::CatalogLoaded {
                        count: snapshot.catalog.available.len(),
                    },
                    Either::Unit(reply),
                )
            }
            ModelCatalogCommand::SetScope(models, reply) => {
                snapshot.catalog.scoped = models;
                snapshot.results = snapshot
                    .catalog
                    .search(&snapshot.query, snapshot.scoped_only);
                (
                    ModelCatalogEvent::ScopeChanged {
                        count: snapshot.catalog.scoped.len(),
                    },
                    Either::Unit(reply),
                )
            }
            ModelCatalogCommand::Search(query, scoped_only, reply) => {
                snapshot.query = query;
                snapshot.scoped_only = scoped_only;
                snapshot.results = snapshot.catalog.search(&snapshot.query, scoped_only);
                (
                    ModelCatalogEvent::SearchChanged {
                        query: snapshot.query.clone(),
                        result_count: snapshot.results.len(),
                    },
                    Either::Unit(reply),
                )
            }
            ModelCatalogCommand::Cycle(direction, reply) => {
                let selected = snapshot
                    .catalog
                    .cycle(snapshot.selected.as_ref(), direction);
                snapshot.selected = selected.clone().map(|item| item.model);
                (
                    ModelCatalogEvent::SelectionChanged {
                        model: snapshot.selected.clone(),
                    },
                    Either::Model(reply, snapshot.selected.clone()),
                )
            }
            ModelCatalogCommand::Select(model, reply) => {
                snapshot.selected = Some(model.clone());
                (
                    ModelCatalogEvent::SelectionChanged {
                        model: Some(model.clone()),
                    },
                    Either::Model(reply, Some(model)),
                )
            }
        };
        snapshot.last_event = Some(event);
        let _ = snapshot_tx.send(snapshot.clone());
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
}
