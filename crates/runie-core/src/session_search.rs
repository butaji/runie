//! Pure session-search projections. Storage and actor orchestration stay at
//! the caller boundary; ranking is just data in, data out.

use serde::{Deserialize, Serialize};

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
            let score = if id == query || name == query {
                3
            } else if id.contains(&query) || name.contains(&query) {
                2
            } else if preview.contains(&query) {
                1
            } else {
                return None;
            };
            Some(SessionSearchResult {
                id: document.id.clone(),
                name: document.name.clone(),
                score,
            })
        })
        .collect::<Vec<_>>();
    results.sort_by(|left, right| right.score.cmp(&left.score).then(left.id.cmp(&right.id)));
    results
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
        assert_eq!(results[1].id, "b");
        assert_eq!(results[1].score, 1);
        assert!(search_sessions(&documents, " ").is_empty());
    }
}
