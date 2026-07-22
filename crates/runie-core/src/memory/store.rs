//! Memory storage implementation.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::info;

/// Source of a memory entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemorySource {
    /// Global memory (applies to all workspaces).
    Global,
    /// Workspace-specific memory.
    Workspace,
    /// Session-specific memory (ephemeral).
    Session,
}

impl std::fmt::Display for MemorySource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemorySource::Global => write!(f, "global"),
            MemorySource::Workspace => write!(f, "workspace"),
            MemorySource::Session => write!(f, "session"),
        }
    }
}

/// A memory entry with content and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// Unique identifier.
    pub id: String,
    /// Content (markdown).
    pub content: String,
    /// Source type.
    pub source: MemorySource,
    /// Associated workspace (if any).
    pub workspace: Option<String>,
    /// Tags for categorization.
    pub tags: Vec<String>,
    /// Created timestamp.
    pub created_at: DateTime<Utc>,
    /// Last accessed timestamp.
    pub accessed_at: DateTime<Utc>,
    /// Access count.
    pub access_count: u64,
    /// Importance score (0.0 to 1.0).
    pub importance: f32,
}

impl MemoryEntry {
    /// Create a new memory entry.
    pub fn new(content: impl Into<String>, source: MemorySource) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            content: content.into(),
            source,
            workspace: None,
            tags: Vec::new(),
            created_at: now,
            accessed_at: now,
            access_count: 0,
            importance: 0.5,
        }
    }

    /// Set workspace for this entry.
    pub fn with_workspace(mut self, workspace: impl Into<String>) -> Self {
        self.workspace = Some(workspace.into());
        self
    }

    /// Set tags for this entry.
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Record an access.
    pub fn record_access(&mut self) {
        self.access_count += 1;
        self.accessed_at = Utc::now();
    }

    /// Set importance score.
    pub fn with_importance(mut self, importance: f32) -> Self {
        self.importance = importance.clamp(0.0, 1.0);
        self
    }
}

/// Memory storage backed by markdown files.
#[derive(Debug)]
pub struct MemoryStore {
    /// Base storage directory.
    base_dir: PathBuf,
    /// Workspace-specific directory.
    workspace_dir: Option<PathBuf>,
    /// Current workspace path (for context).
    #[allow(dead_code)]
    workspace_path: Option<PathBuf>,
}

impl MemoryStore {
    /// Create a new memory store.
    pub fn new(base_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&base_dir).context("failed to create memory base directory")?;
        Ok(Self {
            base_dir,
            workspace_dir: None,
            workspace_path: None,
        })
    }

    /// Create a memory store for a specific workspace.
    pub fn with_workspace(base_dir: PathBuf, workspace_path: &Path) -> Result<Self> {
        let workspace_slug = workspace_slug(workspace_path);
        let workspace_dir = base_dir.join(&workspace_slug);

        std::fs::create_dir_all(&workspace_dir).context("failed to create workspace memory directory")?;

        Ok(Self {
            base_dir,
            workspace_dir: Some(workspace_dir.clone()),
            workspace_path: Some(workspace_path.to_path_buf()),
        })
    }

    /// Store a memory entry.
    pub fn store(&self, entry: &MemoryEntry) -> Result<PathBuf> {
        let dir = match entry.source {
            MemorySource::Global => self.base_dir.join("global"),
            MemorySource::Workspace => self
                .workspace_dir
                .as_ref()
                .context("workspace memory store required")?
                .clone(),
            MemorySource::Session => self
                .workspace_dir
                .as_ref()
                .context("workspace memory store required for session")?
                .join("sessions"),
        };

        std::fs::create_dir_all(&dir).context("failed to create memory directory")?;

        let filename = format!("{}.md", entry.id);
        let path = dir.join(&filename);

        let content = self.entry_to_markdown(entry);
        std::fs::write(&path, content).context("failed to write memory entry")?;

        info!("Stored memory entry: {}", path.display());
        Ok(path)
    }

    /// Search memories by content.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<MemoryEntry>> {
        let mut entries = Vec::new();

        // Search global memories
        let global_dir = self.base_dir.join("global");
        if global_dir.exists() {
            self.search_dir(&global_dir, query, &mut entries);
        }

        // Search workspace memories
        if let Some(ws_dir) = &self.workspace_dir {
            self.search_dir(ws_dir, query, &mut entries);
        }

        // Sort by relevance (simple: count occurrences)
        entries.sort_by(|a, b| {
            let score_a = self.relevance_score(a, query);
            let score_b = self.relevance_score(b, query);
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        entries.truncate(limit);
        Ok(entries)
    }

    /// Get all memories from a directory.
    fn search_dir(&self, dir: &Path, query: &str, entries: &mut Vec<MemoryEntry>) {
        if let Ok(read_dir) = std::fs::read_dir(dir) {
            for entry in read_dir.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "md") {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        if content.to_lowercase().contains(&query.to_lowercase()) {
                            if let Ok(entry) = self.parse_entry(&path, &content) {
                                entries.push(entry);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Calculate relevance score for an entry.
    fn relevance_score(&self, entry: &MemoryEntry, query: &str) -> f32 {
        let query_lower = query.to_lowercase();
        let content_lower = entry.content.to_lowercase();

        let mut score = 0.0;

        // Count query occurrences
        let count = content_lower.matches(&query_lower).count();
        score += count as f32 * 0.5;

        // Boost by importance
        score += entry.importance * 0.3;

        // Boost by access count (logarithmic)
        score += (entry.access_count as f32).ln() * 0.2;

        score
    }

    /// Convert entry to markdown with frontmatter.
    fn entry_to_markdown(&self, entry: &MemoryEntry) -> String {
        let mut content = String::new();

        // Frontmatter
        content.push_str("---\n");
        content.push_str(&format!("id: {}\n", entry.id));
        content.push_str(&format!("source: {}\n", entry.source.to_string().to_lowercase()));
        if let Some(ref ws) = entry.workspace {
            content.push_str(&format!("workspace: {}\n", ws));
        }
        if !entry.tags.is_empty() {
            content.push_str(&format!("tags: [{}]\n", entry.tags.join(", ")));
        }
        content.push_str(&format!(
            "created_at: {}\n",
            entry.created_at.to_rfc3339()
        ));
        content.push_str(&format!(
            "accessed_at: {}\n",
            entry.accessed_at.to_rfc3339()
        ));
        content.push_str(&format!("access_count: {}\n", entry.access_count));
        content.push_str(&format!("importance: {:.2}\n", entry.importance));
        content.push_str("---\n\n");

        // Content
        content.push_str(&entry.content);

        content
    }

    /// Parse a markdown entry from file.
    fn parse_entry(&self, path: &Path, content: &str) -> Result<MemoryEntry> {
        let Some((frontmatter, body)) = content.split_once("---") else {
            return self.parse_entry_without_frontmatter(path, content);
        };
        self.parse_entry_with_frontmatter(path, body.trim(), frontmatter)
    }

    fn parse_entry_without_frontmatter(&self, path: &Path, content: &str) -> Result<MemoryEntry> {
        let id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        Ok(MemoryEntry {
            id,
            content: content.to_string(),
            source: MemorySource::Session,
            workspace: None,
            tags: Vec::new(),
            created_at: Utc::now(),
            accessed_at: Utc::now(),
            access_count: 0,
            importance: 0.5,
        })
    }

    fn parse_entry_with_frontmatter(&self, _path: &Path, body: &str, frontmatter: &str) -> Result<MemoryEntry> {
        let id = extract_field(frontmatter, "id").unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let source = extract_field(frontmatter, "source")
            .map(|s| match s.to_lowercase().as_str() {
                "global" => MemorySource::Global,
                "workspace" => MemorySource::Workspace,
                _ => MemorySource::Session,
            })
            .unwrap_or(MemorySource::Session);

        Ok(MemoryEntry {
            id,
            content: body.to_string(),
            source,
            workspace: extract_field(frontmatter, "workspace"),
            tags: extract_field(frontmatter, "tags")
                .map(|s| s.split(',').map(|t| t.trim().to_string()).collect())
                .unwrap_or_default(),
            created_at: extract_field(frontmatter, "created_at")
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(Utc::now),
            accessed_at: Utc::now(),
            access_count: extract_field(frontmatter, "access_count")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0),
            importance: extract_field(frontmatter, "importance")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.5),
        })
    }

    /// Delete a memory entry.
    pub fn delete(&self, id: &str) -> Result<()> {
        // Search in all directories
        for dir in [
            self.base_dir.join("global"),
            self.base_dir.join("sessions"),
        ] {
            if self.workspace_dir.is_some() {
                // Also check workspace
            }
            let path = dir.join(format!("{}.md", id));
            if path.exists() {
                std::fs::remove_file(&path).context("failed to delete memory entry")?;
                info!("Deleted memory entry: {}", path.display());
                return Ok(());
            }
        }

        if let Some(ws_dir) = &self.workspace_dir {
            for dir in [ws_dir.clone(), ws_dir.join("sessions")] {
                let path = dir.join(format!("{}.md", id));
                if path.exists() {
                    std::fs::remove_file(&path).context("failed to delete memory entry")?;
                    info!("Deleted memory entry: {}", path.display());
                    return Ok(());
                }
            }
        }

        anyhow::bail!("memory entry not found: {}", id);
    }

    /// List all memory entries.
    pub fn list(&self, source: Option<MemorySource>) -> Result<Vec<MemoryEntry>> {
        let dirs = self.list_dirs(source)?;
        let mut entries = Vec::new();

        for (dir, src) in dirs {
            if dir.exists() {
                self.read_entries_from_dir(&dir, src, &mut entries);
            }
        }

        entries.sort_by_key(|e| std::cmp::Reverse(e.accessed_at));
        Ok(entries)
    }

    fn list_dirs(&self, source: Option<MemorySource>) -> Result<Vec<(PathBuf, MemorySource)>> {
        match source {
            Some(MemorySource::Global) => Ok(vec![(self.base_dir.join("global"), MemorySource::Global)]),
            Some(MemorySource::Workspace) => {
                if let Some(ref ws) = self.workspace_dir {
                    Ok(vec![(ws.clone(), MemorySource::Workspace)])
                } else {
                    Ok(Vec::new())
                }
            }
            Some(MemorySource::Session) => {
                if let Some(ref ws) = self.workspace_dir {
                    Ok(vec![(ws.join("sessions"), MemorySource::Session)])
                } else {
                    Ok(Vec::new())
                }
            }
            None => {
                let mut dirs = vec![(self.base_dir.join("global"), MemorySource::Global)];
                if let Some(ref ws) = self.workspace_dir {
                    dirs.push((ws.clone(), MemorySource::Workspace));
                    dirs.push((ws.join("sessions"), MemorySource::Session));
                }
                Ok(dirs)
            }
        }
    }

    fn read_entries_from_dir(&self, dir: &Path, src: MemorySource, entries: &mut Vec<MemoryEntry>) {
        let Ok(read_dir) = std::fs::read_dir(dir) else { return };
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "md") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(mut entry) = self.parse_entry(&path, &content) {
                        entry.source = src;
                        entries.push(entry);
                    }
                }
            }
        }
    }
}

/// Generate a workspace slug from path.
fn workspace_slug(path: &Path) -> String {
    let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());

    let name = canonical
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    // Create a short hash of the full path for uniqueness
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    canonical.to_string_lossy().hash(&mut hasher);
    let hash = format!("{:08x}", hasher.finish());

    format!("{}-{}", name.to_lowercase().replace(' ', "-"), &hash[..8])
}

/// Extract a field from frontmatter.
fn extract_field(frontmatter: &str, field: &str) -> Option<String> {
    for line in frontmatter.lines() {
        if line.starts_with(&format!("{}:", field)) {
            let value = line.split_once(':')?.1.trim();
            return Some(value.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_entry_creation() {
        let entry = MemoryEntry::new("Test content", MemorySource::Global);
        assert_eq!(entry.content, "Test content");
        assert_eq!(entry.source, MemorySource::Global);
        assert!(!entry.id.is_empty());
    }

    #[test]
    fn memory_entry_with_workspace() {
        let entry = MemoryEntry::new("Test", MemorySource::Workspace)
            .with_workspace("/project")
            .with_tags(vec!["important".to_string()]);

        assert_eq!(entry.workspace, Some("/project".to_string()));
        assert_eq!(entry.tags, vec!["important"]);
    }

    #[test]
    fn memory_store_roundtrip() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let store = MemoryStore::new(dir.path().join("memory"))?;

        let entry = MemoryEntry::new("Test memory content", MemorySource::Global);
        let path = store.store(&entry)?;
        assert!(path.exists());

        let results = store.search("Test memory", 10)?;
        assert!(!results.is_empty());

        store.delete(&entry.id)?;
        assert!(!path.exists());

        Ok(())
    }

    #[test]
    fn workspace_slug_generation() {
        let slug = workspace_slug(Path::new("/Users/test/project"));
        assert!(slug.contains("project"));
        assert!(slug.len() < 50);
    }
}
