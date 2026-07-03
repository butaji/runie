//! Ractor-based `FffIndexerActor` implementation using `ignore` + `sublime_fuzzy`.
//!
//! Migrated from `fff-search` to `ignore::WalkBuilder` + `sublime_fuzzy` + `notify` 7.0.

use std::path::PathBuf;

use ractor::{async_trait, Actor, ActorProcessingErr, ActorRef};
use tracing::instrument;

use crate::actors::ractor_adapter::spawn_ractor;
use crate::bus::EventBus;
use crate::event::Event;
use crate::model::FffFileEntry;

use super::{
    FileSearchResult, FffFileItem, FffSearchRequest,
    FffSearchResultPayload, FffSearchState, MAX_FILE_SIZE, SearchIndex, SearchIndexStateInner,
    DEFAULT_LIMIT,
};

/// Ractor-based FffIndexerActor handle.
#[derive(Clone, Debug)]
pub struct RactorFffIndexerHandle {
    inner: ActorRef<FffSearchRequest>,
}

impl RactorFffIndexerHandle {
    /// Create a new handle wrapping an ActorRef.
    pub fn new(inner: ActorRef<FffSearchRequest>) -> Self {
        Self { inner }
    }

    /// Send a search request to the indexer.
    pub async fn search(&self, request: FffSearchRequest) {
        let _ = self.inner.send_message(request);
    }

    /// Try to send a search request (non-blocking).
    pub fn try_search(&self, request: FffSearchRequest) {
        let _ = self.inner.send_message(request);
    }

    /// Try to send a message (non-blocking).
    pub fn try_send(
        &self,
        msg: FffSearchRequest,
    ) -> Result<(), ractor::MessagingErr<FffSearchRequest>> {
        self.inner.send_message(msg)
    }
}

/// Ractor-based FffIndexerActor.
pub struct RactorFffIndexerActor {
    root: PathBuf,
    bus: EventBus<Event>,
    index: SearchIndex,
    indexed: bool,
    init_done: bool,
}

/// Ractor State for FffIndexerActor — tracks init and indexing status.
pub struct FffIndexerActorState {
    pub indexed: bool,
    pub init_done: bool,
}

impl RactorFffIndexerActor {
    fn new(root: PathBuf, _data_dir: PathBuf, bus: EventBus<Event>) -> Self {
        Self {
            root,
            bus,
            index: SearchIndex::new(),
            indexed: false,
            init_done: false,
        }
    }

    /// Spawn a `RactorFffIndexerActor` and return a handle + cell + join.
    /// Initializes the index and waits for initial scan before returning.
    pub async fn spawn(
        root: PathBuf,
        data_dir: PathBuf,
        bus: EventBus<Event>,
    ) -> Result<(RactorFffIndexerHandle, ractor::ActorCell, tokio::task::JoinHandle<()>), ractor::SpawnErr> {
        let mut actor = Self::new(root, data_dir, bus.clone());
        // Initialize the search index on a blocking thread before spawning.
        let index = actor.index.clone();
        let scan_root = actor.root.clone();
        tokio::task::spawn_blocking(move || {
            index.build(&scan_root);
        })
        .await
        .map_err(|e| {
            let msg = format!("scan task failed: {}", e);
            ractor::SpawnErr::StartupPanic(msg.into())
        })?;

        actor.indexed = true;
        actor.init_done = true;

        // Register the index globally.
        let global_state = FffSearchState {
            project_path: actor.root.clone(),
            index: actor.index.clone(),
            indexed: true,
        };
        *super::search_index_state().write() = Some(SearchIndexStateInner {
            state: global_state,
        });

        tracing::debug!("search indexer: initial scan complete ({} files)", actor.index.file_count());

        let (handle, join, cell) = spawn_ractor(None, actor, bus).await?;
        Ok((RactorFffIndexerHandle::new(handle), cell, join))
    }
}

#[async_trait]
impl Actor for RactorFffIndexerActor {
    type Msg = FffSearchRequest;
    type State = FffIndexerActorState;
    type Arguments = EventBus<Event>;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        _args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(FffIndexerActorState {
            indexed: self.indexed,
            init_done: self.init_done,
        })
    }

    #[instrument(name = "fff_indexer", skip_all, fields(msg = ?msg))]
    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        msg: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        let indexed = state.indexed;
        let payload = self.handle_search(msg, indexed).await;
        let entries = payload
            .items
            .iter()
            .map(|item| FffFileEntry {
                name: std::path::Path::new(&item.relative_path)
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| item.relative_path.clone()),
                path: item.relative_path.clone(),
                is_dir: item.relative_path.ends_with('/'),
                score: item.score,
                git_status: item.git_status.clone(),
            })
            .collect();
        self.emit(Event::FffSearchResult {
            request_id: payload.request_id,
            entries,
            query: payload.query,
            indexed: payload.indexed,
        });
        Ok(())
    }
}

impl RactorFffIndexerActor {
    /// Handle a search request.
    async fn handle_search(
        &self,
        req: FffSearchRequest,
        indexed: bool,
    ) -> FffSearchResultPayload {
        let limit = req.limit.unwrap_or(DEFAULT_LIMIT);

        // Parse the query to determine the search type.
        let parsed = crate::location::parse_search_query(&req.query);

        let items: Vec<FffFileItem>;
        let total_matched: usize;

        if parsed.location.is_some() && !parsed.text.is_empty() {
            // Content search for location queries.
            let matches = self.index.grep(&req.query, MAX_FILE_SIZE, 10, limit);
            total_matched = matches.len();
            items = Vec::new(); // Location results use a different format.
        } else if parsed.globs().next().is_some() {
            // Glob search.
            let results = self.index.glob_search(&req.query, limit);
            total_matched = results.len();
            items = results
                .into_iter()
                .map(|r| convert_file_result(&req, r))
                .collect();
        } else {
            // Fuzzy file search.
            let results = self.index.fuzzy_search(&req.query, limit);
            total_matched = results.len();
            items = results
                .into_iter()
                .map(|r| convert_file_result(&req, r))
                .collect();
        }

        FffSearchResultPayload {
            request_id: req.request_id,
            query: req.query.clone(),
            items,
            total_matched,
            indexed,
        }
    }

    fn emit(&self, event: Event) {
        self.bus.publish(event);
    }
}

/// Convert a file search result to an `FffFileItem`.
fn convert_file_result(_req: &FffSearchRequest, result: FileSearchResult) -> FffFileItem {
    #[cfg(feature = "git")]
    let git_status = result.git_status;
    #[cfg(feature = "git")]
    let git_tracked = git_status.is_some();
    #[cfg(not(feature = "git"))]
    let git_tracked = false;
    FffFileItem {
        relative_path: result.relative_path.clone(),
        absolute_path: result.absolute_path.to_string_lossy().into_owned(),
        score: result.score,
        git_tracked,
        git_status: {
            #[cfg(feature = "git")]
            {
                use super::format_git_status;
                git_status.map(|s| format_git_status(s).to_owned())
            }
            #[cfg(not(feature = "git"))]
            {
                None
            }
        },
    }
}

/// Format a git2 Status as a human-readable string (returns `"clean"` for clean state).
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn ractor_fff_indexer_actor_spawns() {
        use crate::bus::EventBus;
        use crate::event::Event;

        FffSearchState::reset_for_test();
        let bus = EventBus::<Event>::new(16);
        let temp_dir = tempfile::tempdir().unwrap();
        let result = RactorFffIndexerActor::spawn(
            temp_dir.path().to_path_buf(),
            temp_dir.path().to_path_buf(),
            bus,
        )
        .await;
        assert!(result.is_ok());
    }
}
