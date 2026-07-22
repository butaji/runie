//! Memory storage implementation.
//!
//! Provides markdown-based I/O for cross-session memory persistence.
//! Storage layout:
//! - `~/.runie/memory/MEMORY.md` — global curated knowledge
//! - `~/.runie/memory/{workspace_hash}/MEMORY.md` — workspace-specific knowledge
//! - `~/.runie/memory/{workspace_hash}/sessions/` — per-session logs

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::{debug, info};

/// Memory scope (Global, Workspace, or Session).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum MemoryScope {
    /// Global memory (applies to all workspaces).
    Global,
    /// Workspace-specific memory.
    #[default]
    Workspace,
    /// Session-specific memory (ephemeral, per-conversation).
    Session,
}

impl std::fmt::Display for MemoryScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryScope::Global => write!(f, "global"),
            MemoryScope::Workspace => write!(f, "workspace"),
            MemoryScope::Session => write!(f, "session"),
        }
    }
}

/// Frontmatter metadata for memory entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryFrontmatter {
    /// Unique identifier.
    pub id: Option<String>,
    /// Memory scope.
    #[serde(default)]
    pub scope: MemoryScope,
    /// Workspace hash (for workspace/session scope).
    pub workspace: Option<String>,
    /// Tags for categorization.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Creation timestamp.
    pub timestamp: DateTime<Utc>,
    /// Importance score (0.0 to 1.0).
    #[serde(default = "default_importance")]
    pub importance: f32,
    /// Source description.
    #[serde(default)]
    pub source: String,
}

fn default_importance() -> f32 {
    0.5
}

/// A memory entry with content and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// Frontmatter metadata.
    #[serde(flatten)]
    pub frontmatter: MemoryFrontmatter,
    /// Content (markdown).
    pub content: String,
}

impl MemoryEntry {
    /// Create a new memory entry.
    pub fn new(content: impl Into<String>, scope: MemoryScope) -> Self {
        Self {
            frontmatter: MemoryFrontmatter {
                id: Some(uuid::Uuid::new_v4().to_string()),
                scope,
                workspace: None,
                tags: Vec::new(),
                timestamp: Utc::now(),
                importance: 0.5,
                source: String::new(),
            },
            content: content.into(),
        }
    }

    /// Set workspace hash.
    pub fn with_workspace(mut self, workspace: impl Into<String>) -> Self {
        self.frontmatter.workspace = Some(workspace.into());
        self
    }

    /// Set tags.
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.frontmatter.tags = tags;
        self
    }

    /// Set importance score.
    pub fn with_importance(mut self, importance: f32) -> Self {
        self.frontmatter.importance = importance.clamp(0.0, 1.0);
        self
    }

    /// Set source description.
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.frontmatter.source = source.into();
        self
    }

    /// Get the entry ID.
    pub fn id(&self) -> &str {
        self.frontmatter.id.as_deref().unwrap_or("unknown")
    }

    /// Convert entry to markdown string with frontmatter.
    pub fn to_markdown(&self) -> String {
        let fm = &self.frontmatter;
        let mut s = String::new();

        s.push_str("---\n");
        if let Some(ref id) = fm.id {
            s.push_str(&format!("id: {}\n", id));
        }
        s.push_str(&format!("scope: {}\n", fm.scope));
        if let Some(ref ws) = fm.workspace {
            s.push_str(&format!("workspace: {}\n", ws));
        }
        if !fm.tags.is_empty() {
            s.push_str(&format!("tags: [{}]\n", fm.tags.join(", ")));
        }
        s.push_str(&format!("timestamp: {}\n", fm.timestamp.to_rfc3339()));
        s.push_str(&format!("importance: {:.2}\n", fm.importance));
        if !fm.source.is_empty() {
            s.push_str(&format!("source: {}\n", fm.source));
        }
        s.push_str("---\n\n");
        s.push_str(&self.content);

        s
    }

    /// Parse entry from markdown string.
    pub fn from_markdown(markdown: &str) -> Result<Self> {
        let Some((frontmatter_section, body)) = markdown.split_once("\n---\n") else {
            // No frontmatter, treat entire content as body
            return Ok(Self {
                frontmatter: MemoryFrontmatter {
                    id: Some(uuid::Uuid::new_v4().to_string()),
                    scope: MemoryScope::Workspace,
                    workspace: None,
                    tags: Vec::new(),
                    timestamp: Utc::now(),
                    importance: 0.5,
                    source: String::new(),
                },
                content: markdown.trim().to_string(),
            });
        };

        let fm = parse_frontmatter(frontmatter_section)?;
        Ok(Self {
            frontmatter: fm,
            content: body.trim().to_string(),
        })
    }
}

/// Parse frontmatter from section.
#[allow(clippy::too_many_lines)]
fn parse_frontmatter(section: &str) -> Result<MemoryFrontmatter> {
    let mut id = None;
    let mut scope = MemoryScope::Workspace;
    let mut workspace = None;
    let mut tags = Vec::new();
    let mut timestamp = Utc::now();
    let mut importance = 0.5;
    let mut source = String::new();

    for line in section.lines() {
        let line = line.trim();
        if line.is_empty() || line == "---" {
            continue;
        }

        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim().trim_matches(|c| c == '"' || c == '\'');

            match key {
                "id" => id = Some(value.to_string()),
                "scope" => {
                    scope = match value.to_lowercase().as_str() {
                        "global" => MemoryScope::Global,
                        "workspace" => MemoryScope::Workspace,
                        "session" => MemoryScope::Session,
                        _ => MemoryScope::Workspace,
                    };
                }
                "workspace" => workspace = Some(value.to_string()),
                "tags" => {
                    let value = value.trim_matches(|c| c == '[' || c == ']');
                    tags = value
                        .split(',')
                        .map(|s| s.trim().trim_matches('"').to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
                "timestamp" => {
                    timestamp = DateTime::parse_from_rfc3339(value)
                        .map(|d| d.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now());
                }
                "importance" => {
                    importance = value.parse().unwrap_or(0.5);
                }
                "source" => source = value.to_string(),
                _ => {}
            }
        }
    }

    Ok(MemoryFrontmatter {
        id,
        scope,
        workspace,
        tags,
        timestamp,
        importance,
        source,
    })
}

/// Memory storage backed by markdown files.
#[derive(Debug)]
pub struct MemoryStorage {
    /// Base storage directory (~/.runie/memory).
    base_dir: PathBuf,
}

impl MemoryStorage {
    /// Create a new memory storage instance.
    pub fn new() -> Result<Self> {
        let base_dir = dirs::data_dir()
            .context("failed to get data directory")?
            .join("runie")
            .join("memory");

        std::fs::create_dir_all(&base_dir).context("failed to create memory directory")?;

        debug!("Memory storage initialized at: {}", base_dir.display());
        Ok(Self { base_dir })
    }

    /// Create with a custom base directory.
    pub fn with_base_dir(base_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&base_dir).context("failed to create memory directory")?;
        Ok(Self { base_dir })
    }

    /// Get the base directory.
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// Compute workspace hash from path.
    pub fn workspace_hash(workspace_path: &Path) -> String {
        let canonical = std::fs::canonicalize(workspace_path)
            .unwrap_or_else(|_| workspace_path.to_path_buf());
        let hash = blake3::hash(canonical.to_string_lossy().as_bytes());
        hash.to_hex()[..16].to_string()
    }

    /// Get the global memory file path.
    pub fn global_memory_path(&self) -> PathBuf {
        self.base_dir.join("MEMORY.md")
    }

    /// Get the workspace memory file path.
    pub fn workspace_memory_path(&self, workspace_hash: &str) -> PathBuf {
        self.base_dir
            .join(workspace_hash)
            .join("MEMORY.md")
    }

    /// Get the sessions directory for a workspace.
    pub fn sessions_dir(&self, workspace_hash: &str) -> PathBuf {
        self.base_dir.join(workspace_hash).join("sessions")
    }

    /// Get a session file path.
    pub fn session_path(&self, workspace_hash: &str, date: &str, slug: &str, session_id: &str) -> PathBuf {
        self.sessions_dir(workspace_hash)
            .join(format!("{}-{}-{}.md", date, slug, &session_id[..8.min(session_id.len())]))
    }

    /// Read global memory entries.
    pub fn read_global_memory(&self) -> Result<Vec<MemoryEntry>> {
        self.read_memory_file(&self.global_memory_path())
    }

    /// Read workspace-specific memory entries.
    pub fn read_workspace_memory(&self, workspace_hash: &str) -> Result<Vec<MemoryEntry>> {
        self.read_memory_file(&self.workspace_memory_path(workspace_hash))
    }

    /// Read memory from a file (split by entries separated by ## ).
    fn read_memory_file(&self, path: &Path) -> Result<Vec<MemoryEntry>> {
        if !path.exists() {
            return Ok(Vec::new());
        }

        let content = std::fs::read_to_string(path).context("failed to read memory file")?;
        let entries: Vec<MemoryEntry> = content
            .split("\n## ")
            .filter(|s| !s.trim().is_empty())
            .map(|s| MemoryEntry::from_markdown(s.strip_prefix("## ").unwrap_or(s)))
            .filter_map(|e| e.ok())
            .collect();

        Ok(entries)
    }

    /// Write global memory.
    pub fn write_global_memory(&self, entries: &[MemoryEntry]) -> Result<()> {
        self.write_memory_file(&self.global_memory_path(), entries, MemoryScope::Global)
    }

    /// Write workspace memory.
    pub fn write_workspace_memory(&self, workspace_hash: &str, entries: &[MemoryEntry]) -> Result<()> {
        let path = self.workspace_memory_path(workspace_hash);
        self.write_memory_file(&path, entries, MemoryScope::Workspace)
    }

    /// Write entries to a memory file.
    fn write_memory_file(&self, path: &Path, entries: &[MemoryEntry], default_scope: MemoryScope) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context("failed to create memory directory")?;
        }

        let mut content = String::new();
        content.push_str("# Memory\n\n");

        for entry in entries {
            let mut entry = entry.clone();
            if entry.frontmatter.scope == MemoryScope::Workspace && default_scope == MemoryScope::Global {
                entry.frontmatter.scope = default_scope;
            }

            content.push_str("## ");
            if let Some(ref id) = entry.frontmatter.id {
                content.push_str(id);
            } else {
                content.push_str(&uuid::Uuid::new_v4().to_string());
            }
            content.push('\n');
            content.push_str(&entry.to_markdown());
            content.push_str("\n\n");
        }

        std::fs::write(path, content).context("failed to write memory file")?;
        info!("Wrote {} entries to {}", entries.len(), path.display());
        Ok(())
    }

    /// Append a session entry.
    pub fn append_session(
        &self,
        workspace_hash: &str,
        date: &str,
        slug: &str,
        session_id: &str,
        content: &str,
    ) -> Result<PathBuf> {
        let path = self.session_path(workspace_hash, date, slug, session_id);

        // Ensure sessions directory exists
        std::fs::create_dir_all(path.parent().unwrap()).context("failed to create sessions directory")?;

        let mut entry = MemoryEntry::new(content, MemoryScope::Session);
        entry.frontmatter.source = format!("session: {}-{}", date, slug);

        let markdown = format!(
            "---\nid: {}\nscope: session\nworkspace: {}\ntimestamp: {}\nimportance: 0.3\n---\n\n{}",
            session_id,
            workspace_hash,
            Utc::now().to_rfc3339(),
            content
        );

        std::fs::write(&path, markdown).context("failed to write session entry")?;
        info!("Appended session entry: {}", path.display());

        Ok(path)
    }

    /// List all session files for a workspace.
    pub fn list_sessions(&self, workspace_hash: &str) -> Result<Vec<PathBuf>> {
        let sessions_dir = self.sessions_dir(workspace_hash);
        if !sessions_dir.exists() {
            return Ok(Vec::new());
        }

        let mut paths: Vec<PathBuf> = std::fs::read_dir(&sessions_dir)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "md"))
            .collect();

        paths.sort();
        Ok(paths)
    }

    /// Read all session entries for a workspace.
    pub fn read_sessions(&self, workspace_hash: &str) -> Result<Vec<MemoryEntry>> {
        let paths = self.list_sessions(workspace_hash)?;
        let mut entries = Vec::new();

        for path in paths {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(entry) = MemoryEntry::from_markdown(&content) {
                    entries.push(entry);
                }
            }
        }

        Ok(entries)
    }

    /// Search memories across all scopes.
    pub fn search(&self, workspace_hash: Option<&str>, query: &str) -> Result<Vec<MemoryEntry>> {
        let mut results = Vec::new();

        // Search global memory
        if let Ok(entries) = self.read_global_memory() {
            results.extend(entries);
        }

        // Search workspace memory if provided
        if let Some(hash) = workspace_hash {
            if let Ok(entries) = self.read_workspace_memory(hash) {
                results.extend(entries);
            }
            if let Ok(entries) = self.read_sessions(hash) {
                results.extend(entries);
            }
        }

        // Filter by query relevance
        let query_lower = query.to_lowercase();
        results.retain(|entry| {
            entry.content.to_lowercase().contains(&query_lower)
                || entry.frontmatter.tags.iter().any(|t| t.to_lowercase().contains(&query_lower))
        });

        Ok(results)
    }
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new().expect("failed to create memory storage")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_hash_deterministic() {
        let path = Path::new("/test/workspace");
        let hash1 = MemoryStorage::workspace_hash(path);
        let hash2 = MemoryStorage::workspace_hash(path);
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 16);
    }

    #[test]
    fn test_memory_entry_roundtrip() {
        let entry = MemoryEntry::new("Test content", MemoryScope::Workspace)
            .with_tags(vec!["test".to_string()])
            .with_importance(0.8);

        let markdown = entry.to_markdown();
        let parsed = MemoryEntry::from_markdown(&markdown).unwrap();

        assert_eq!(parsed.content, entry.content);
        assert_eq!(parsed.frontmatter.importance, entry.frontmatter.importance);
    }

    #[test]
    fn test_storage_paths() {
        let temp_dir = tempfile::tempdir().unwrap();
        let storage = MemoryStorage::with_base_dir(temp_dir.path().to_path_buf()).unwrap();

        assert_eq!(storage.global_memory_path(), temp_dir.path().join("MEMORY.md"));

        let hash = "abc123";
        assert_eq!(
            storage.workspace_memory_path(hash),
            temp_dir.path().join("abc123").join("MEMORY.md")
        );
        assert_eq!(
            storage.session_path(hash, "2026-07-22", "test", "session-id-12345"),
            temp_dir
                .path()
                .join("abc123")
                .join("sessions")
                .join("2026-07-22-test-session-.md") // session_id truncated to 8 chars
        );
    }
}
