//! Cross-session memory system.
//!
//! Provides persistent memory storage with search capabilities:
//! - Markdown file storage under `~/.runie/memory/`
//! - In-memory keyword-based search index
//! - MMR (Maximum Marginal Relevance) search for diverse results
//! - Text chunking for efficient indexing
//! - Session summarization on context near limit
//!
//! # Storage Layout
//!
//! ```text
//! ~/.runie/memory/
//! ├── MEMORY.md                          # Global curated knowledge
//! └── {workspace_hash}/                 # Per-workspace (blake3(cwd)[..16])
//!     ├── MEMORY.md                      # Project-level knowledge
//!     └── sessions/
//!         └── YYYY-MM-DD-{slug}-{sid8}.md  # Session logs
//! ```

pub mod store;
pub mod storage;
pub mod index;
pub mod chunker;
pub mod search;
pub mod mmr;

// Re-export from storage module
pub use storage::{MemoryStorage, MemoryEntry, MemoryScope};

// Re-export from index module
pub use index::{MemoryIndex, IndexedDocument, IndexedResult};

// Re-export from chunker module
pub use chunker::{TextChunker, TextChunk, ChunkConfig};

// Re-export from store module (backward compatibility)
pub use store::{MemoryStore, MemoryEntry as LegacyMemoryEntry, MemorySource};

// Re-export from search module
pub use search::{SearchQuery, SearchResult};
pub use mmr::mmr_rerank;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Memory configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Enable memory system.
    pub enabled: bool,
    /// Memory storage directory.
    pub storage_dir: PathBuf,
    /// Maximum entries to keep per workspace.
    pub max_workspace_entries: usize,
    /// Embedding model to use (or None for keyword search only).
    pub embedding_model: Option<String>,
    /// MMR diversity factor (0.0 to 1.0).
    pub mmr_lambda: f32,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            storage_dir: dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".runie")
                .join("memory"),
            max_workspace_entries: 1000,
            embedding_model: None,
            mmr_lambda: 0.7,
        }
    }
}
