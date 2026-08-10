//! Pure session-search projections. Storage and actor orchestration stay at
//! the caller boundary; ranking is just data in, data out.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};

use crate::session::{SessionSnapshot, SessionStorageRow};
use crate::task_owner::{spawn_actor_worker, TaskOwner};

const MAX_SESSION_PREVIEW_CHARS: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionSearchDocument {
    pub id: String,
    pub name: Option<String>,
    pub preview: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionSearchResult {
    pub id: String,
    pub name: Option<String>,
    pub score: u8,
    pub matched_by: SessionMatchField,
}

/// Filter storage metadata for a picker without coupling search to a widget.
/// Empty queries preserve discovery order; non-empty queries match id, label,
/// or cwd case-insensitively and remain deterministic.
pub fn filter_storage_rows(rows: &[SessionStorageRow], query: &str) -> Vec<SessionStorageRow> {
    let query = query.trim().to_ascii_lowercase();
    rows.iter()
        .filter(|row| {
            query.is_empty()
                || row.session_id.to_ascii_lowercase().contains(&query)
                || row.label.to_ascii_lowercase().contains(&query)
                || row.cwd.to_ascii_lowercase().contains(&query)
        })
        .cloned()
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionMatchField {
    Id,
    Name,
    Preview,
}

pub fn document_from_snapshot(
    id: impl Into<String>,
    snapshot: &SessionSnapshot,
) -> SessionSearchDocument {
    let preview = snapshot
        .branch_context_messages()
        .into_iter()
        .rev()
        .find_map(|message| serde_json::to_string(&message).ok())
        .unwrap_or_default();
    SessionSearchDocument {
        id: id.into(),
        name: snapshot.name(),
        preview: preview.chars().take(MAX_SESSION_PREVIEW_CHARS).collect(),
    }
}

enum SearchMessage {
    Upsert(SessionSearchDocument),
    Remove(String),
    Search {
        query: String,
        reply: oneshot::Sender<Vec<SessionSearchResult>>,
    },
}

#[derive(Clone)]
pub struct SessionSearchIndex {
    tx: mpsc::Sender<SearchMessage>,
    _owner: Arc<TaskOwner>,
}

impl SessionSearchIndex {
    pub fn new() -> Self {
        let (task_tx, task_owner) = spawn_actor_worker!(128, move |mut input: mpsc::Receiver<
            SearchMessage,
        >| async move {
            let mut documents = BTreeMap::new();
            while let Some(message) = input.recv().await {
                match message {
                    SearchMessage::Upsert(document) => {
                        documents.insert(document.id.clone(), document);
                    }
                    SearchMessage::Remove(id) => {
                        documents.remove(&id);
                    }
                    SearchMessage::Search { query, reply } => {
                        let _ = reply.send(search_sessions(
                            &documents.values().cloned().collect::<Vec<_>>(),
                            &query,
                        ));
                    }
                }
            }
        });
        Self {
            tx: task_tx,
            _owner: task_owner,
        }
    }

    pub async fn upsert(&self, document: SessionSearchDocument) {
        let _ = self.tx.send(SearchMessage::Upsert(document)).await;
    }

    pub async fn upsert_snapshot(&self, id: impl Into<String>, snapshot: &SessionSnapshot) {
        self.upsert(document_from_snapshot(id, snapshot)).await;
    }

    pub async fn remove(&self, id: impl Into<String>) {
        let _ = self.tx.send(SearchMessage::Remove(id.into())).await;
    }

    pub async fn search(&self, query: impl Into<String>) -> Vec<SessionSearchResult> {
        let (reply, result) = oneshot::channel();
        if self
            .tx
            .send(SearchMessage::Search {
                query: query.into(),
                reply,
            })
            .await
            .is_err()
        {
            return Vec::new();
        }
        result.await.unwrap_or_default()
    }
}

impl Default for SessionSearchIndex {
    fn default() -> Self {
        Self::new()
    }
}

pub fn search_sessions(
    documents: &[SessionSearchDocument],
    query: &str,
) -> Vec<SessionSearchResult> {
    let query = query.trim().to_ascii_lowercase();
    if query.is_empty() {
        return Vec::new();
    }
    let mut results = documents
        .iter()
        .filter_map(|document| {
            let id = document.id.to_ascii_lowercase();
            let name = document
                .name
                .as_deref()
                .unwrap_or_default()
                .to_ascii_lowercase();
            let preview = document.preview.to_ascii_lowercase();
            let (score, matched_by) = match_field(&id, &name, &preview, &query)?;
            Some(SessionSearchResult {
                id: document.id.clone(),
                name: document.name.clone(),
                score,
                matched_by,
            })
        })
        .collect::<Vec<_>>();
    results.sort_by(|left, right| right.score.cmp(&left.score).then(left.id.cmp(&right.id)));
    results
}

fn match_field(
    id: &str,
    name: &str,
    preview: &str,
    query: &str,
) -> Option<(u8, SessionMatchField)> {
    if id == query {
        Some((3, SessionMatchField::Id))
    } else if name == query {
        Some((3, SessionMatchField::Name))
    } else if id.contains(query) {
        Some((2, SessionMatchField::Id))
    } else if name.contains(query) {
        Some((2, SessionMatchField::Name))
    } else if preview.contains(query) {
        Some((1, SessionMatchField::Preview))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_ranks_name_id_and_preview_matches_deterministically() {
        let documents = vec![
            SessionSearchDocument {
                id: "a".into(),
                name: Some("Deploy".into()),
                preview: "release notes".into(),
            },
            SessionSearchDocument {
                id: "b".into(),
                name: Some("Notes".into()),
                preview: "deploy checklist".into(),
            },
        ];
        let results = search_sessions(&documents, "deploy");
        assert_eq!(results[0].id, "a");
        assert_eq!(results[0].score, 3);
        assert_eq!(results[0].matched_by, SessionMatchField::Name);
        assert_eq!(results[1].id, "b");
        assert_eq!(results[1].score, 1);
        assert_eq!(results[1].matched_by, SessionMatchField::Preview);
        assert!(search_sessions(&documents, " ").is_empty());
    }

    #[tokio::test]
    async fn index_reduces_upsert_remove_and_search_events() {
        let index = SessionSearchIndex::new();
        index
            .upsert(SessionSearchDocument {
                id: "one".into(),
                name: Some("First".into()),
                preview: "hello".into(),
            })
            .await;
        assert_eq!(index.search("first").await[0].id, "one");
        index.remove("one").await;
        assert!(index.search("first").await.is_empty());
    }

    #[test]
    fn snapshot_projection_keeps_name_and_bounds_preview() {
        let snapshot = SessionSnapshot {
            config_records: vec![crate::session::SessionConfigEntry {
                id: "name".into(),
                parent_id: None,
                seq: 1,
                timestamp: 0,
                lane: "main".into(),
                record: crate::session::SessionConfigRecord::NameChanged {
                    name: "Deploy".into(),
                },
            }],
            ..SessionSnapshot::default()
        };
        let document = document_from_snapshot("session-1", &snapshot);
        assert_eq!(document.name.as_deref(), Some("Deploy"));
        assert!(document.preview.chars().count() <= MAX_SESSION_PREVIEW_CHARS);
    }

    #[test]
    fn storage_rows_filter_by_id_label_or_cwd_without_reordering() {
        let rows = vec![
            SessionStorageRow {
                path: "/tmp/one.jsonl".into(),
                session_id: "one".into(),
                label: "Deploy".into(),
                cwd: "/work/a".into(),
                created_at: 1,
            },
            SessionStorageRow {
                path: "/tmp/two.jsonl".into(),
                session_id: "two".into(),
                label: "Notes".into(),
                cwd: "/work/b".into(),
                created_at: 2,
            },
        ];
        assert_eq!(filter_storage_rows(&rows, "deploy")[0].session_id, "one");
        assert_eq!(filter_storage_rows(&rows, "/WORK/B")[0].session_id, "two");
        assert_eq!(filter_storage_rows(&rows, "").len(), 2);
    }
}
