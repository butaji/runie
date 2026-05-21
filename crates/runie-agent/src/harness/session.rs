use super::types::*;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

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
}

impl Session {
    pub fn new(metadata: SessionMetadata) -> Self {
        Self {
            metadata,
            entries: Vec::new(),
            leaf_id: None,
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
    
    pub fn get_entries(&self) -> &[SessionTreeEntry] {
        &self.entries
    }
    
    pub fn get_path_to_root(&self, leaf_id: Option<&str>) -> Vec<&SessionTreeEntry> {
        // Build parent-child map and trace back from leaf
        let mut path = vec![];
        // Simple implementation: just return all entries for now
        self.entries.iter().for_each(|e| path.push(e));
        path
    }
    
    pub fn set_leaf_id(&mut self, leaf_id: String) {
        self.leaf_id = Some(leaf_id);
    }
}