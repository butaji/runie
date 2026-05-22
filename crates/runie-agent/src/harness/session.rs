use super::types::*;
use crate::events::AgentMessage;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Session not found")]
    NotFound,
}

pub struct JsonlSessionRepo {
    base_path: PathBuf,
}

impl JsonlSessionRepo {
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self { base_path: base_path.into() }
    }
    
    pub fn create(&self, cwd: String) -> anyhow::Result<Session> {
        let id = uuid::Uuid::new_v4().to_string();
        let path = self.base_path.join(format!("{}.jsonl", id));
        
        let metadata = SessionMetadata {
            id: id.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
            cwd: cwd.clone(),
            path: path.to_string_lossy().to_string(),
            parent_session_path: None,
        };
        
        // Create file
        File::create(&path)?;
        
        Ok(Session::new(metadata))
    }
    
    pub fn open(&self, metadata: SessionMetadata) -> anyhow::Result<Session> {
        Ok(Session::new(metadata))
    }
}

pub struct Session {
    pub metadata: SessionMetadata,
    entries: Vec<SessionTreeEntry>,
    leaf_id: Option<String>,
    messages: Vec<AgentMessage>,
}

impl Session {
    pub fn new(metadata: SessionMetadata) -> Self {
        Self {
            metadata,
            entries: Vec::new(),
            leaf_id: None,
            messages: Vec::new(),
        }
    }
    
    pub fn append_entry(&mut self, entry: SessionTreeEntry) -> anyhow::Result<()> {
        // Serialize and append to JSONL file
        let json = serde_json::to_string(&entry)?;
        let mut file = OpenOptions::new()
            .append(true)
            .open(&self.metadata.path)?;
        writeln!(file, "{}", json)?;
        
        self.entries.push(entry);
        Ok(())
    }
    
    pub fn add_message(&mut self, message: AgentMessage) {
        self.messages.push(message);
    }
    
    pub fn get_entries(&self) -> &[SessionTreeEntry] {
        &self.entries
    }
    
    pub fn messages(&self) -> &[AgentMessage] {
        &self.messages
    }
    
    pub fn file_path(&self) -> &str {
        &self.metadata.path
    }
    
    pub fn get_path_to_root(&self, leaf_id: Option<&str>) -> Vec<&SessionTreeEntry> {
        let mut path = Vec::new();
        
        let start_id = leaf_id.or_else(|| {
            self.entries.last().map(|e| e.id())
        });
        
        if let Some(start) = start_id {
            let mut current = start;
            loop {
                if let Some(entry) = self.entries.iter().find(|e| e.id() == current) {
                    path.push(entry);
                    match entry.parent_id() {
                        Some(parent) => current = parent,
                        None => break,
                    }
                } else {
                    break;
                }
            }
        }
        
        path.reverse();
        path
    }
    
    pub fn set_leaf_id(&mut self, leaf_id: String) {
        self.leaf_id = Some(leaf_id);
    }
    
    pub fn load_entries(&self) -> Result<Vec<SessionTreeEntry>, SessionError> {
        let path = self.file_path();
        if !std::path::Path::new(path).exists() {
            return Ok(Vec::new());
        }
        
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut entries = Vec::new();
        
        for (line_num, line) in reader.lines().enumerate() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<SessionTreeEntry>(&line) {
                Ok(entry) => entries.push(entry),
                Err(e) => tracing::warn!("Failed to parse line {}: {}", line_num + 1, e),
            }
        }
        
        Ok(entries)
    }
    
    pub fn load_session(&mut self) -> Result<(), SessionError> {
        let entries = self.load_entries()?;
        
        for entry in entries {
            if let SessionTreeEntry::Message { message, .. } = entry {
                self.add_message(message);
            }
        }
        
        Ok(())
    }
}

impl SessionTreeEntry {
    /// Returns the type tag for this entry (e.g., "Message", "Compaction", etc.)
    pub fn entry_type(&self) -> &'static str {
        match self {
            SessionTreeEntry::Message { .. } => "Message",
            SessionTreeEntry::Compaction { .. } => "Compaction",
            SessionTreeEntry::BranchSummary { .. } => "BranchSummary",
            SessionTreeEntry::Label { .. } => "Label",
            SessionTreeEntry::Leaf { .. } => "Leaf",
        }
    }
    
    /// Returns the ID of this entry
    pub fn id(&self) -> &str {
        match self {
            SessionTreeEntry::Message { id, .. } => id,
            SessionTreeEntry::Compaction { id, .. } => id,
            SessionTreeEntry::BranchSummary { id, .. } => id,
            SessionTreeEntry::Label { id, .. } => id,
            SessionTreeEntry::Leaf { id, .. } => id,
        }
    }
    
    /// Returns the parent ID of this entry, if any
    pub fn parent_id(&self) -> Option<&str> {
        match self {
            SessionTreeEntry::Message { parent_id, .. } => parent_id.as_deref(),
            SessionTreeEntry::Compaction { parent_id, .. } => parent_id.as_deref(),
            SessionTreeEntry::BranchSummary { parent_id, .. } => parent_id.as_deref(),
            SessionTreeEntry::Label { parent_id, .. } => parent_id.as_deref(),
            SessionTreeEntry::Leaf { parent_id, .. } => parent_id.as_deref(),
        }
    }
}
