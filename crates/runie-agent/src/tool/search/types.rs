use git2::Status as GitStatus;
use serde::Serialize;

/// Default max results per search.
pub(crate) const DEFAULT_LIMIT: usize = 50;

/// Default max matches per file for content search.
pub(crate) const DEFAULT_MAX_MATCHES: usize = 10;

/// Search result payload.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub(crate) items: Vec<SearchItem>,
    pub(crate) total: usize,
    pub(crate) indexed: bool,
}

/// Single search result entry.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchItem {
    pub(crate) path: String,
    pub(crate) line: Option<u64>,
    pub(crate) col: Option<usize>,
    pub(crate) content: Option<String>,
    pub(crate) score: f64,
    pub(crate) git_status: Option<String>,
}

/// Search mode selector.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum SearchMode {
    #[default]
    Files,
    Content,
    Mixed,
    Glob,
}

impl SearchMode {
    pub(crate) fn from_str(s: &str) -> Self {
        match s {
            "content" => SearchMode::Content,
            "mixed" => SearchMode::Mixed,
            "glob" => SearchMode::Glob,
            _ => SearchMode::Files,
        }
    }
}

/// Build a file search item with formatted git status.
pub fn build_search_item(path: String, git_status: Option<GitStatus>, score: f64) -> SearchItem {
    let git_status = git_status.map(format_git_status).filter(|s| !s.is_empty());
    SearchItem {
        path,
        line: None,
        col: None,
        content: None,
        score,
        git_status,
    }
}

/// Map a git status to a short label.
pub fn format_git_status(status: GitStatus) -> String {
    if status.contains(GitStatus::WT_NEW) || status.contains(GitStatus::INDEX_NEW) {
        return "untracked".to_owned();
    }
    if status.contains(GitStatus::WT_MODIFIED) || status.contains(GitStatus::INDEX_MODIFIED) {
        return "modified".to_owned();
    }
    if status.contains(GitStatus::WT_DELETED) || status.contains(GitStatus::INDEX_DELETED) {
        return "deleted".to_owned();
    }
    if status.contains(GitStatus::WT_RENAMED) || status.contains(GitStatus::INDEX_RENAMED) {
        return "renamed".to_owned();
    }
    "clean".to_owned()
}
