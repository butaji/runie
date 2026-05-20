//! Session management — inspired by pi's SessionManager
//!
//! Session is stored as JSON lines with tree structure:
//! - Each entry has id/parentId for tree navigation
//! - Supports branching (forking sessions)
//! - Compaction to stay within context limits

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use uuid::Uuid;
use chrono::Utc;

/// Session version for migrations
const CURRENT_VERSION: u32 = 1;

/// Session header (first line)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionHeader {
    #[serde(rename = "type")]
    pub entry_type: String,
    pub version: u32,
    pub id: String,
    pub timestamp: String,
    pub cwd: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_session: Option<String>,
}

impl SessionHeader {
    pub fn new(cwd: PathBuf, parent_session: Option<String>) -> Self {
        Self {
            entry_type: "session".to_string(),
            version: CURRENT_VERSION,
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now().to_rfc3339(),
            cwd: cwd.to_string_lossy().to_string(),
            parent_session,
        }
    }
}

/// Base fields for all entries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryBase {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    pub timestamp: String,
}

/// User or assistant message entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEntry {
    #[serde(flatten)]
    pub base: EntryBase,
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
}

impl MessageEntry {
    pub fn user(content: String, parent_id: Option<String>) -> Self {
        Self {
            base: EntryBase {
                id: Uuid::new_v4().to_string(),
                parent_id,
                timestamp: Utc::now().to_rfc3339(),
            },
            role: "user".to_string(),
            content,
            tool_call_id: None,
            tool_name: None,
        }
    }

    pub fn assistant(content: String, parent_id: Option<String>) -> Self {
        Self {
            base: EntryBase {
                id: Uuid::new_v4().to_string(),
                parent_id,
                timestamp: Utc::now().to_rfc3339(),
            },
            role: "assistant".to_string(),
            content,
            tool_call_id: None,
            tool_name: None,
        }
    }
}

/// Compaction entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionEntry {
    #[serde(flatten)]
    pub base: EntryBase,
    pub summary: String,
    pub first_kept_entry_id: String,
    pub tokens_before: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    #[serde(default)]
    pub from_hook: bool,
}

/// Branch summary entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchSummaryEntry {
    #[serde(flatten)]
    pub base: EntryBase,
    pub from_id: String,
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    #[serde(default)]
    pub from_hook: bool,
}

/// Custom entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomEntry {
    #[serde(flatten)]
    pub base: EntryBase,
    pub custom_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Custom message entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomMessageEntry {
    #[serde(flatten)]
    pub base: EntryBase,
    pub custom_type: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    #[serde(default)]
    pub display: bool,
}

/// Thinking level change entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingLevelChangeEntry {
    #[serde(flatten)]
    pub base: EntryBase,
    pub thinking_level: String,
}

/// Model change entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelChangeEntry {
    #[serde(flatten)]
    pub base: EntryBase,
    pub provider: String,
    pub model_id: String,
}

/// Label entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelEntry {
    #[serde(flatten)]
    pub base: EntryBase,
    pub target_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// Session info entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfoEntry {
    #[serde(flatten)]
    pub base: EntryBase,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Entry types for the session log
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SessionEntry {
    Message(MessageEntry),
    Compaction(CompactionEntry),
    BranchSummary(BranchSummaryEntry),
    Custom(CustomEntry),
    CustomMessage(CustomMessageEntry),
    ThinkingLevelChange(ThinkingLevelChangeEntry),
    ModelChange(ModelChangeEntry),
    Label(LabelEntry),
    SessionInfo(SessionInfoEntry),
}

impl SessionEntry {
    pub fn id(&self) -> &str {
        match self {
            SessionEntry::Message(e) => &e.base.id,
            SessionEntry::Compaction(e) => &e.base.id,
            SessionEntry::BranchSummary(e) => &e.base.id,
            SessionEntry::Custom(e) => &e.base.id,
            SessionEntry::CustomMessage(e) => &e.base.id,
            SessionEntry::ThinkingLevelChange(e) => &e.base.id,
            SessionEntry::ModelChange(e) => &e.base.id,
            SessionEntry::Label(e) => &e.base.id,
            SessionEntry::SessionInfo(e) => &e.base.id,
        }
    }

    pub fn parent_id(&self) -> Option<&str> {
        match self {
            SessionEntry::Message(e) => e.base.parent_id.as_deref(),
            SessionEntry::Compaction(e) => e.base.parent_id.as_deref(),
            SessionEntry::BranchSummary(e) => e.base.parent_id.as_deref(),
            SessionEntry::Custom(e) => e.base.parent_id.as_deref(),
            SessionEntry::CustomMessage(e) => e.base.parent_id.as_deref(),
            SessionEntry::ThinkingLevelChange(e) => e.base.parent_id.as_deref(),
            SessionEntry::ModelChange(e) => e.base.parent_id.as_deref(),
            SessionEntry::Label(e) => e.base.parent_id.as_deref(),
            SessionEntry::SessionInfo(e) => e.base.parent_id.as_deref(),
        }
    }

    pub fn timestamp(&self) -> &str {
        match self {
            SessionEntry::Message(e) => &e.base.timestamp,
            SessionEntry::Compaction(e) => &e.base.timestamp,
            SessionEntry::BranchSummary(e) => &e.base.timestamp,
            SessionEntry::Custom(e) => &e.base.timestamp,
            SessionEntry::CustomMessage(e) => &e.base.timestamp,
            SessionEntry::ThinkingLevelChange(e) => &e.base.timestamp,
            SessionEntry::ModelChange(e) => &e.base.timestamp,
            SessionEntry::Label(e) => &e.base.timestamp,
            SessionEntry::SessionInfo(e) => &e.base.timestamp,
        }
    }
}

/// Events emitted by Session
#[derive(Debug, Clone)]
pub enum SessionEvent {
    EntryAdded(SessionEntry),
    CompactionStarted { reason: CompactionReason },
    CompactionComplete { summary: String, tokens_saved: usize },
    SessionForked { new_session_id: String },
    SessionRenamed { name: Option<String> },
    LabelSet { target_id: String, label: Option<String> },
}

/// Reason for compaction
#[derive(Debug, Clone)]
pub enum CompactionReason {
    Manual,
    Threshold,
    Overflow,
}

/// Session listener callback
pub type SessionListener = Box<dyn Fn(SessionEvent) + Send + Sync>;

/// Session — event-driven session management
pub struct Session {
    id: String,
    cwd: PathBuf,
    path: PathBuf,
    header: SessionHeader,
    entries: Vec<SessionEntry>,
    tree: HashMap<String, Vec<String>>,
    listeners: Vec<SessionListener>,
    current_parent: Option<String>,
    labels: HashMap<String, Option<String>>,
}

impl Session {
    /// Create or open a session
    pub fn new(cwd: PathBuf, path: Option<PathBuf>) -> std::io::Result<Self> {
        let path = path.unwrap_or_else(|| {
            let sessions_dir = cwd.join(".anvil/sessions");
            std::fs::create_dir_all(&sessions_dir).ok();
            let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
            sessions_dir.join(format!("session_{}.jsonl", timestamp))
        });

        let header = SessionHeader::new(cwd.clone(), None);
        let id = header.id.clone();

        let mut session = Self {
            id,
            cwd,
            path: path.clone(),
            header,
            entries: Vec::new(),
            tree: HashMap::new(),
            listeners: Vec::new(),
            current_parent: None,
            labels: HashMap::new(),
        };

        if path.exists() {
            session.load()?;
        } else {
            session.write_header()?;
        }

        Ok(session)
    }

    /// Subscribe to session events
    pub fn subscribe<F>(&mut self, listener: F)
    where
        F: Fn(SessionEvent) + Send + Sync + 'static,
    {
        self.listeners.push(Box::new(listener));
    }

    /// Emit an event to all listeners
    fn emit(&self, event: SessionEvent) {
        for listener in &self.listeners {
            listener(event.clone());
        }
    }

    /// Load session from disk
    fn load(&mut self) -> std::io::Result<()> {
        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            if let Ok(entry) = serde_json::from_str::<SessionEntry>(&line) {
                if let Some(parent_id) = entry.parent_id() {
                    self.tree.entry(parent_id.to_string())
                        .or_insert_with(Vec::new)
                        .push(entry.id().to_string());
                }
                self.entries.push(entry);
            } else if let Ok(header) = serde_json::from_str::<SessionHeader>(&line) {
                self.header = header.clone();
                self.id = header.id.clone();
            }
        }

        self.current_parent = self.entries.last().map(|e| e.id().to_string());
        Ok(())
    }

    /// Write header to file
    fn write_header(&self) -> std::io::Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.path)?;

        let json = serde_json::to_string(&self.header)?;
        writeln!(file, "{}", json)?;
        Ok(())
    }

    /// Append an entry to the session file
    fn append_entry(&self, entry: &SessionEntry) -> std::io::Result<()> {
        let mut file = OpenOptions::new()
            .append(true)
            .open(&self.path)?;

        let json = serde_json::to_string(entry)?;
        writeln!(file, "{}", json)?;
        Ok(())
    }

    /// Add a user message
    pub fn add_user_message(&mut self, content: String) {
        let entry = SessionEntry::Message(MessageEntry::user(content, self.current_parent.clone()));
        self.add_entry(entry);
    }

    /// Add an assistant message
    pub fn add_assistant_message(&mut self, content: String) {
        let entry = SessionEntry::Message(MessageEntry::assistant(content, self.current_parent.clone()));
        self.add_entry(entry);
    }

    /// Add a custom message (for extensions)
    pub fn add_custom_message(&mut self, custom_type: String, content: String, display: bool) {
        let entry = SessionEntry::CustomMessage(CustomMessageEntry {
            base: EntryBase {
                id: Uuid::new_v4().to_string(),
                parent_id: self.current_parent.clone(),
                timestamp: Utc::now().to_rfc3339(),
            },
            custom_type,
            content,
            details: None,
            display,
        });
        self.add_entry(entry);
    }

    /// Add any entry type
    fn add_entry(&mut self, entry: SessionEntry) {
        let parent_id = entry.parent_id().map(String::from);

        if let Some(ref pid) = parent_id {
            self.tree.entry(pid.clone())
                .or_insert_with(Vec::new)
                .push(entry.id().to_string());
        }

        self.current_parent = Some(entry.id().to_string());
        self.entries.push(entry.clone());

        if let Err(e) = self.append_entry(&entry) {
            eprintln!("[session] Failed to persist entry: {}", e);
        }

        self.emit(SessionEvent::EntryAdded(entry));
    }

    /// Fork a new session from current position
    pub fn fork(&self, new_path: PathBuf) -> std::io::Result<Session> {
        let mut new_session = Self::new(self.cwd.clone(), Some(new_path))?;
        new_session.header.parent_session = Some(self.id.clone());

        let summary = self.build_context_summary();
        new_session.add_entry(SessionEntry::BranchSummary(BranchSummaryEntry {
            base: EntryBase {
                id: Uuid::new_v4().to_string(),
                parent_id: None,
                timestamp: Utc::now().to_rfc3339(),
            },
            from_id: self.entries.last().map(|e| e.id().to_string()).unwrap_or_default(),
            summary,
            details: None,
            from_hook: false,
        }));

        Ok(new_session)
    }

    /// Build a summary of current context (for compaction)
    fn build_context_summary(&self) -> String {
        let recent: Vec<_> = self.entries.iter().rev().take(10).collect();
        let messages: Vec<_> = recent.iter()
            .filter_map(|e| {
                if let SessionEntry::Message(m) = e {
                    let preview = if m.content.len() > 100 {
                        format!("{}...", &m.content[..100])
                    } else {
                        m.content.clone()
                    };
                    Some(format!("[{}] {}: {}", m.role, preview, if m.content.len() > 100 { "..." } else { "" }))
                } else {
                    None
                }
            })
            .collect();

        messages.join("\n")
    }

    /// Compact the session (summarize older entries)
    pub fn compact(&mut self, reason: CompactionReason) {
        self.emit(SessionEvent::CompactionStarted { reason: reason.clone() });

        let summary = self.build_context_summary();
        let first_kept_id = self.entries.first()
            .map(|e| e.id().to_string())
            .unwrap_or_default();

        let old_count = self.entries.len().saturating_sub(10);
        let tokens_saved = old_count * 500;

        // Retain last 10 entries
        let retained: Vec<SessionEntry> = self.entries.clone().into_iter().skip(old_count).collect();
        self.entries = retained;

        let entry = SessionEntry::Compaction(CompactionEntry {
            base: EntryBase {
                id: Uuid::new_v4().to_string(),
                parent_id: self.current_parent.clone(),
                timestamp: Utc::now().to_rfc3339(),
            },
            summary: summary.clone(),
            first_kept_entry_id: first_kept_id,
            tokens_before: self.entries.len() * 500,
            details: None,
            from_hook: false,
        });

        self.add_entry(entry);
        self.emit(SessionEvent::CompactionComplete { summary: summary.clone(), tokens_saved });
    }

    /// Set a label on an entry
    pub fn set_label(&mut self, target_id: String, label: Option<String>) {
        self.labels.insert(target_id.clone(), label.clone());
        self.add_entry(SessionEntry::Label(LabelEntry {
            base: EntryBase {
                id: Uuid::new_v4().to_string(),
                parent_id: self.current_parent.clone(),
                timestamp: Utc::now().to_rfc3339(),
            },
            target_id,
            label,
        }));
    }

    /// Get session ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get session path
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Get all entries
    pub fn entries(&self) -> &[SessionEntry] {
        &self.entries
    }

    /// Build messages for LLM context
    pub fn build_context(&self) -> Vec<(String, String)> {
        self.entries.iter()
            .filter_map(|e| {
                match e {
                    SessionEntry::Message(m) => Some((m.role.clone(), m.content.clone())),
                    SessionEntry::CustomMessage(m) if m.display => {
                        Some(("user".to_string(), m.content.clone()))
                    }
                    _ => None,
                }
            })
            .collect()
    }

    /// Get entry count
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Get label for an entry
    pub fn get_label(&self, entry_id: &str) -> Option<&Option<String>> {
        self.labels.get(entry_id)
    }

    /// Navigate to parent entry
    pub fn go_to_parent(&mut self) {
        if let Some(current) = &self.current_parent {
            for entry in &self.entries {
                if entry.id() == current {
                    self.current_parent = entry.parent_id().map(String::from);
                    break;
                }
            }
        }
    }

    /// Get children of current entry
    pub fn get_children(&self) -> Vec<&SessionEntry> {
        let parent = self.current_parent.as_deref().unwrap_or("");
        self.tree.get(parent)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.entries.iter().find(|e| e.id() == id))
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_session_create_and_persist() {
        let dir = tempdir().unwrap();
        let session = Session::new(dir.path().to_path_buf(), None).unwrap();
        
        assert!(session.path().exists());
        assert_eq!(session.entry_count(), 0);
    }

    #[test]
    fn test_add_user_message() {
        let dir = tempdir().unwrap();
        let mut session = Session::new(dir.path().to_path_buf(), None).unwrap();
        
        session.add_user_message("Hello, world!".to_string());
        
        assert_eq!(session.entry_count(), 1);
        let ctx = session.build_context();
        assert_eq!(ctx.len(), 1);
        assert_eq!(ctx[0].0, "user");
        assert_eq!(ctx[0].1, "Hello, world!");
    }

    #[test]
    fn test_compaction() {
        let dir = tempdir().unwrap();
        let mut session = Session::new(dir.path().to_path_buf(), None).unwrap();
        
        for i in 0..15 {
            session.add_user_message(format!("Message {}", i));
            session.add_assistant_message(format!("Response {}", i));
        }
        
        assert!(session.entry_count() > 10);
        session.compact(CompactionReason::Manual);
        
        assert!(session.entry_count() <= 15);
    }

    #[test]
    fn test_labels() {
        let dir = tempdir().unwrap();
        let mut session = Session::new(dir.path().to_path_buf(), None).unwrap();
        
        session.add_user_message("Test".to_string());
        let entry_id = session.entries()[0].id().to_string();
        
        session.set_label(entry_id.clone(), Some("important".to_string()));
        
        assert_eq!(session.get_label(&entry_id), Some(&Some("important".to_string())));
    }

    #[test]
    fn test_fork_session() {
        let dir = tempdir().unwrap();
        let mut session = Session::new(dir.path().to_path_buf(), None).unwrap();
        
        session.add_user_message("Original".to_string());
        
        let fork_path = dir.path().join("fork.jsonl");
        let fork = session.fork(fork_path.clone()).unwrap();
        
        assert_ne!(fork.id(), session.id());
        assert_eq!(fork.header.parent_session, Some(session.id().to_string()));
        assert!(fork_path.exists());
    }
}
