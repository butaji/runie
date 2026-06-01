use runie_core::{Session, MessageNode};
use std::path::PathBuf;
use thiserror::Error;
use tokio::fs;

pub struct SessionManager {
    sessions_dir: PathBuf,
}

#[derive(Debug, Error)]
pub enum SessionManagerError {
    #[error("session not found: {0}")]
    NotFound(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl SessionManager {

    #[must_use]
    #[must_use]
    pub fn new(sessions_dir: PathBuf) -> Self {
        Self { sessions_dir }
    }

    pub async fn load(&self, session_id: &str) -> Result<Session, SessionManagerError> {
        let path = self.sessions_dir.join(format!("{}.jsonl", session_id));
        let content = fs::read_to_string(&path).await?;
        
        // Parse JSONL - each line is a MessageNode
        let mut messages = Vec::new();
        for line in content.lines() {
            if !line.trim().is_empty() {
                let node: MessageNode = serde_json::from_str(line)?;
                messages.push(node);
            }
        }
        
        let mut session = Session::new(session_id.to_string());
        session.messages = messages;
        Ok(session)
    }

    pub async fn save(&self, session: &Session) -> Result<(), SessionManagerError> {
        fs::create_dir_all(&self.sessions_dir).await?;
        let path = self.sessions_dir.join(format!("{}.jsonl", session.id));
        
        let mut content = String::new();
        for node in &session.messages {
            let line = serde_json::to_string(node)?;
            content.push_str(&line);
            content.push('\n');
        }
        
        fs::write(&path, content).await?;
        Ok(())
    }

    pub async fn list(&self) -> Result<Vec<String>, SessionManagerError> {
        let mut sessions = Vec::new();
        if self.sessions_dir.exists() {
            let mut entries = fs::read_dir(&self.sessions_dir).await?;
            while let Some(entry) = entries.next_entry().await? {
                if let Some(name) = entry.file_name().to_str() {
                    if name.ends_with(".jsonl") {
                        sessions.push(name.trim_end_matches(".jsonl").to_string());
                    }
                }
            }
        }
        Ok(sessions)
    }

    pub async fn create_branch(
        &self,
        session_id: &str,
        parent_message_id: &str,
    ) -> Result<Session, SessionManagerError> {
        let parent_session = self.load(session_id).await?;
        let mut new_session = Session::new(format!("{}-{}", session_id, parent_message_id));
        
        // Copy messages up to the branch point
        // In real impl, walk the tree and copy relevant branch
        new_session.messages = parent_session.messages.clone();
        
        Ok(new_session)
    }
}
