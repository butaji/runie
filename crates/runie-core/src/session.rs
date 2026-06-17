//! Legacy session persistence — monolithic JSON files in ~/.runie/sessions/
//!
//! Deprecated: `/save` and `/load` now use `SessionStore` (redb) with durable
//! events. `Store::save`/`Store::load` remain for import/export compatibility
//! and read-only access to existing JSON sessions. Migrate old sessions by
//! loading them with `/load` and saving with `/save`.

use crate::model::ChatMessage;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Session snapshot — serializable conversation state
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Session {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub created_at: f64,
    pub updated_at: f64,
    pub messages: Vec<ChatMessage>,
    pub provider: String,
    pub model: String,
    pub theme_name: String,
    pub thinking_level: crate::model::ThinkingLevel,
    pub read_only: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_tree: Option<crate::session_tree::SessionTree>,
}

impl Session {
    /// Build a JSON session snapshot from the current application state.
    pub fn from_state(state: &crate::model::AppState, name: String) -> Self {
        Self {
            name,
            display_name: state.session.session_display_name.clone(),
            created_at: state.session.session_created_at,
            updated_at: crate::model::now(),
            messages: state.session.messages.clone(),
            provider: state.config.current_provider.clone(),
            model: state.config.current_model.clone(),
            theme_name: state.config.theme_name.clone(),
            thinking_level: state.config.thinking_level,
            read_only: state.config.read_only,
            session_tree: state.session.session_tree.clone(),
        }
    }
}

/// Session store — handles save/load/list/delete
#[derive(Debug, Clone)]
pub struct Store {
    pub dir: PathBuf,
}

impl Store {
    /// Default store — uses OS data dir (~/.local/share/runie/sessions on Linux)
    pub fn default_store() -> Option<Self> {
        dirs::data_dir().map(|d| Self::new(d.join("runie").join("sessions")))
    }

    /// Store with explicit directory (for testing)
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    pub fn path(&self, name: &str) -> PathBuf {
        self.dir.join(format!("{}.json", name))
    }

    fn ensure_dir(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.dir)
    }

    /// Save session to JSON file
    pub fn save(&self, name: &str, session: &Session) -> anyhow::Result<()> {
        self.ensure_dir()?;
        let json = serde_json::to_string_pretty(session)?;
        std::fs::write(self.path(name), json)?;
        Ok(())
    }

    /// Load session from JSON file
    pub fn load(&self, name: &str) -> anyhow::Result<Session> {
        let json = std::fs::read_to_string(self.path(name))?;
        let session: Session = serde_json::from_str(&json)?;
        Ok(session)
    }

    /// List all saved session names (sorted)
    pub fn list(&self) -> anyhow::Result<Vec<String>> {
        if !self.dir.exists() {
            return Ok(Vec::new());
        }
        let mut names = Vec::new();
        for entry in std::fs::read_dir(&self.dir)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some(stem) = name.strip_suffix(".json") {
                names.push(stem.to_string());
            }
        }
        names.sort();
        Ok(names)
    }

    /// Delete a session file
    pub fn delete(&self, name: &str) -> anyhow::Result<()> {
        std::fs::remove_file(self.path(name))?;
        Ok(())
    }
}

pub fn default_store() -> Option<Store> {
    if let Ok(dir) = std::env::var("RUNIE_SESSIONS_DIR") {
        return Some(Store::new(PathBuf::from(dir)));
    }
    Store::default_store()
}

pub fn save(name: &str, session: &Session) -> anyhow::Result<()> {
    default_store()
        .ok_or_else(|| anyhow::anyhow!("No data directory"))?
        .save(name, session)
}

pub fn load(name: &str) -> anyhow::Result<Session> {
    default_store()
        .ok_or_else(|| anyhow::anyhow!("No data directory"))?
        .load(name)
}

pub fn list() -> anyhow::Result<Vec<String>> {
    default_store()
        .ok_or_else(|| anyhow::anyhow!("No data directory"))?
        .list()
}

pub fn delete(name: &str) -> anyhow::Result<()> {
    default_store()
        .ok_or_else(|| anyhow::anyhow!("No data directory"))?
        .delete(name)
}

/// Format a slice of chat messages as Markdown for export or sharing.
pub fn format_as_markdown(messages: &[ChatMessage], display_name: Option<&str>) -> String {
    let mut lines = Vec::new();
    let title = display_name.unwrap_or("Session");
    lines.push(format!("# {}\n", title));

    for msg in messages {
        let role_label = match msg.role {
            crate::model::Role::User => "User",
            crate::model::Role::Assistant => "Assistant",
            crate::model::Role::System => "System",
            crate::model::Role::Tool => "Tool",
            crate::model::Role::Thought => "Thought",
            crate::model::Role::TurnComplete => continue,
        };
        lines.push(format!("## {}\n", role_label));
        lines.push(msg.content.clone());
        lines.push(String::new());
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ChatMessage, Role};
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn tmp_store() -> Store {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("runie_test_{}_{}", std::process::id(), n));
        let _ = std::fs::remove_dir_all(&dir);
        Store::new(dir)
    }

    fn sample_session(name: &str) -> Session {
        Session {
            name: name.to_string(),
            display_name: None,
            created_at: 1.0,
            updated_at: 2.0,
            messages: vec![
                ChatMessage {
                    role: Role::User,
                    content: "hi".into(),
                    timestamp: 1.0,
                    id: "req.0".into(),
                    ..Default::default()
                },
                ChatMessage {
                    role: Role::Assistant,
                    content: "hello".into(),
                    timestamp: 2.0,
                    id: "resp.0".into(),
                    ..Default::default()
                },
            ],
            provider: "mock".into(),
            model: "echo".into(),
            theme_name: "runie".into(),
            thinking_level: crate::model::ThinkingLevel::Off,
            read_only: false,
            session_tree: None,
        }
    }

    #[test]
    fn save_creates_json_file() {
        let store = tmp_store();
        let session = sample_session("test1");
        store.save("test1", &session).unwrap();
        assert!(store.path("test1").exists(), "JSON file should exist");
    }

    #[test]
    fn load_roundtrip() {
        let store = tmp_store();
        let original = sample_session("roundtrip");
        store.save("roundtrip", &original).unwrap();
        let loaded = store.load("roundtrip").unwrap();
        assert_eq!(loaded.name, "roundtrip");
        assert_eq!(loaded.messages.len(), 2);
        assert_eq!(loaded.messages[0].role, Role::User);
        assert_eq!(loaded.messages[0].content, "hi");
        assert_eq!(loaded.provider, "mock");
        assert_eq!(loaded.model, "echo");
    }

    #[test]
    fn list_returns_sorted_names() {
        let store = tmp_store();
        store.save("beta", &sample_session("beta")).unwrap();
        store.save("alpha", &sample_session("alpha")).unwrap();
        store.save("gamma", &sample_session("gamma")).unwrap();
        let names = store.list().unwrap();
        assert_eq!(names, vec!["alpha", "beta", "gamma"]);
    }

    #[test]
    fn list_empty_dir_returns_empty() {
        let store = tmp_store();
        let names = store.list().unwrap();
        assert!(names.is_empty());
    }

    #[test]
    fn load_missing_session_fails() {
        let store = tmp_store();
        let result = store.load("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn delete_removes_session() {
        let store = tmp_store();
        store
            .save("to_delete", &sample_session("to_delete"))
            .unwrap();
        assert!(store.path("to_delete").exists());
        store.delete("to_delete").unwrap();
        assert!(!store.path("to_delete").exists());
    }

    #[test]
    fn serialize_role_roundtrip() {
        let role = Role::Assistant;
        let json = serde_json::to_string(&role).unwrap();
        let decoded: Role = serde_json::from_str(&json).unwrap();
        assert_eq!(role, decoded);
    }

    #[test]
    fn serialize_chat_message_roundtrip() {
        let msg = ChatMessage {
            role: Role::User,
            content: "test".into(),
            timestamp: 1.5,
            id: "req.1".into(),
            ..Default::default()
        };
        let json = serde_json::to_string(&msg).unwrap();
        let decoded: ChatMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(msg.role, decoded.role);
        assert_eq!(msg.content, decoded.content);
        assert_eq!(msg.timestamp, decoded.timestamp);
        assert_eq!(msg.id, decoded.id);
    }

    #[test]
    fn serialize_session_full_roundtrip() {
        let session = sample_session("full");
        let json = serde_json::to_string_pretty(&session).unwrap();
        let decoded: Session = serde_json::from_str(&json).unwrap();
        assert_eq!(session.name, decoded.name);
        assert_eq!(session.messages.len(), decoded.messages.len());
        assert_eq!(session.theme_name, decoded.theme_name);
        assert_eq!(session.thinking_level, decoded.thinking_level);
    }

    #[test]
    fn session_persists_provider() {
        let mut session = sample_session("provider_test");
        session.messages[1].provider = "openai".to_string();
        let store = tmp_store();
        store.save("provider_test", &session).unwrap();
        let loaded = store.load("provider_test").unwrap();
        assert_eq!(loaded.messages[1].provider, "openai");
    }

    #[test]
    fn format_session_markdown_includes_title() {
        let session = sample_session("md_test");
        let md = format_as_markdown(&session.messages, Some("My Session"));
        assert!(md.starts_with("# My Session\n"));
    }

    #[test]
    fn format_session_markdown_includes_roles() {
        let session = sample_session("md_test");
        let md = format_as_markdown(&session.messages, None);
        assert!(md.contains("## User\n"));
        assert!(md.contains("## Assistant\n"));
    }

    #[test]
    fn format_session_markdown_skips_turn_complete() {
        let mut session = sample_session("md_test");
        session.messages.push(ChatMessage {
            role: Role::TurnComplete,
            content: String::new(),
            timestamp: 3.0,
            id: "tc".into(),
            ..Default::default()
        });
        let md = format_as_markdown(&session.messages, None);
        assert!(!md.contains("TurnComplete"));
    }
}
