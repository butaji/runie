//! Agent profile loading and saving from TOML files in `~/.runie/agents/`.
//!
//! Each profile specifies:
//! - name (filename stem)
//! - description
//! - system_prompt
//! - tools (allowed tool names)
//! - max_turns (optional)
//! - allowlist_tools / denylist_tools (optional)
//!
//! This module is a slim copy of `runie_agent::profiles` so that
//! `runie-core` doesn't need to depend on `runie-agent` (which
//! depends back on it). Both implementations must stay in sync.

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentProfile {
    pub name: String,
    pub description: String,
    pub system_prompt: String,
    pub tools: Vec<String>,
    #[serde(default)]
    pub max_turns: Option<u32>,
    #[serde(default)]
    pub allowlist_tools: Option<Vec<String>>,
    #[serde(default)]
    pub denylist_tools: Option<Vec<String>>,
}

impl AgentProfile {
    pub fn new(name: impl Into<String>, system_prompt: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            system_prompt: system_prompt.into(),
            tools: Vec::new(),
            max_turns: None,
            allowlist_tools: None,
            denylist_tools: None,
        }
    }

    /// Check if a tool is allowed by this profile.
    pub fn is_tool_allowed(&self, tool_name: &str) -> bool {
        if let Some(denylist) = &self.denylist_tools {
            if denylist.iter().any(|t| t == tool_name) {
                return false;
            }
        }
        if let Some(allowlist) = &self.allowlist_tools {
            return allowlist.iter().any(|t| t == tool_name);
        }
        self.tools.iter().any(|t| t == tool_name)
    }

    /// Get the effective max_turns (defaults to None = unlimited).
    pub fn effective_max_turns(&self) -> Option<u32> {
        self.max_turns
    }
}

/// Default profiles directory.
pub fn profiles_dir() -> std::path::PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".runie")
        .join("agents")
}

/// Parse a profile from a TOML string.
pub fn parse_profile(toml_str: &str) -> Result<AgentProfile, ProfileError> {
    toml::from_str(toml_str).map_err(|e| ProfileError::Parse(e.to_string()))
}

/// Load a single profile from a file path.
pub fn load_profile_from_file(path: &Path) -> Result<AgentProfile, ProfileError> {
    let content = std::fs::read_to_string(path).map_err(|e| ProfileError::Io(e.to_string()))?;
    parse_profile(&content)
}

/// Load all profiles from a directory (sorted by name).
pub fn load_profiles_from_dir(dir: &Path) -> Result<Vec<AgentProfile>, ProfileError> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut profiles = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(|e| ProfileError::Io(e.to_string()))? {
        let entry = entry.map_err(|e| ProfileError::Io(e.to_string()))?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("toml") {
            if let Ok(p) = load_profile_from_file(&path) {
                profiles.push(p);
            }
        }
    }
    profiles.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(profiles)
}

/// Save a profile to disk.
pub fn save_profile(profile: &AgentProfile) -> Result<std::path::PathBuf, ProfileError> {
    let dir = profiles_dir();
    std::fs::create_dir_all(&dir).map_err(|e| ProfileError::Io(e.to_string()))?;
    let path = dir.join(format!("{}.toml", profile.name));
    let toml_str =
        toml::to_string_pretty(profile).map_err(|e| ProfileError::Parse(e.to_string()))?;
    std::fs::write(&path, toml_str).map_err(|e| ProfileError::Io(e.to_string()))?;
    Ok(path)
}

/// Delete a profile from disk.
pub fn delete_profile(name: &str) -> Result<(), ProfileError> {
    let path = profiles_dir().join(format!("{}.toml", name));
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| ProfileError::Io(e.to_string()))?;
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProfileError {
    Io(String),
    Parse(String),
}

impl std::fmt::Display for ProfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProfileError::Io(msg) => write!(f, "I/O error: {}", msg),
            ProfileError::Parse(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

impl std::error::Error for ProfileError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_profile() {
        let toml_str = r#"
            name = "test"
            description = "A test profile"
            system_prompt = "You are a test agent."
            tools = ["read", "write"]
        "#;
        let profile = parse_profile(toml_str).unwrap();
        assert_eq!(profile.name, "test");
        assert_eq!(profile.description, "A test profile");
        assert_eq!(profile.system_prompt, "You are a test agent.");
        assert_eq!(profile.tools, vec!["read", "write"]);
    }

    #[test]
    fn round_trip() {
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("HOME", dir.path());

        let original = AgentProfile {
            name: "rt".into(),
            description: "d".into(),
            system_prompt: "p".into(),
            tools: vec!["read".into()],
            max_turns: Some(50),
            allowlist_tools: None,
            denylist_tools: None,
        };

        let path = save_profile(&original).unwrap();
        let loaded = load_profile_from_file(&path).unwrap();
        assert_eq!(loaded, original);
    }

    #[test]
    fn tool_allowed_logic() {
        let p = AgentProfile {
            name: "x".into(),
            description: "".into(),
            system_prompt: "".into(),
            tools: vec!["read".into()],
            max_turns: None,
            allowlist_tools: None,
            denylist_tools: Some(vec!["read".into()]),
        };
        assert!(!p.is_tool_allowed("read"));
    }

    #[test]
    fn parse_profile_with_optional_fields() {
        let toml_str = r#"
            name = "advanced"
            description = "An advanced profile"
            system_prompt = "You are advanced."
            tools = ["read", "write", "bash"]
            max_turns = 50
            allowlist_tools = ["read", "write"]
        "#;
        let profile = parse_profile(toml_str).unwrap();
        assert_eq!(profile.max_turns, Some(50));
        assert_eq!(
            profile.allowlist_tools,
            Some(vec!["read".to_string(), "write".to_string()])
        );
    }

    #[test]
    fn parse_profile_with_denylist() {
        let toml_str = r#"
            name = "safe"
            description = "Safe profile"
            system_prompt = "Be safe."
            tools = ["read", "write", "bash"]
            denylist_tools = ["bash"]
        "#;
        let profile = parse_profile(toml_str).unwrap();
        assert_eq!(profile.denylist_tools, Some(vec!["bash".to_string()]));
    }

    #[test]
    fn parse_invalid_toml_returns_error() {
        let result = parse_profile("this is = not valid toml ===");
        assert!(result.is_err());
    }

    #[test]
    fn missing_required_field_returns_error() {
        let toml_str = r#"
            description = "Missing name"
            system_prompt = "x"
            tools = []
        "#;
        assert!(parse_profile(toml_str).is_err());
    }

    #[test]
    fn new_profile_has_empty_tools() {
        let p = AgentProfile::new("test", "prompt");
        assert!(p.tools.is_empty());
        assert_eq!(p.max_turns, None);
    }

    #[test]
    fn tool_allowed_in_tools_list() {
        let p = AgentProfile {
            name: "test".into(),
            description: "".into(),
            system_prompt: "".into(),
            tools: vec!["read".into(), "write".into()],
            max_turns: None,
            allowlist_tools: None,
            denylist_tools: None,
        };
        assert!(p.is_tool_allowed("read"));
        assert!(!p.is_tool_allowed("bash"));
    }

    #[test]
    fn tool_denied_by_denylist() {
        let p = AgentProfile {
            name: "test".into(),
            description: "".into(),
            system_prompt: "".into(),
            tools: vec!["read".into(), "bash".into()],
            max_turns: None,
            allowlist_tools: None,
            denylist_tools: Some(vec!["bash".into()]),
        };
        assert!(p.is_tool_allowed("read"));
        assert!(!p.is_tool_allowed("bash"));
    }

    #[test]
    fn tool_allowlist_overrides_tools_list() {
        let p = AgentProfile {
            name: "test".into(),
            description: "".into(),
            system_prompt: "".into(),
            tools: vec!["read".into(), "write".into(), "bash".into()],
            max_turns: None,
            allowlist_tools: Some(vec!["read".into()]),
            denylist_tools: None,
        };
        assert!(p.is_tool_allowed("read"));
        assert!(!p.is_tool_allowed("write"));
    }

    #[test]
    fn denylist_overrides_allowlist() {
        let p = AgentProfile {
            name: "test".into(),
            description: "".into(),
            system_prompt: "".into(),
            tools: vec![],
            max_turns: None,
            allowlist_tools: Some(vec!["read".into(), "bash".into()]),
            denylist_tools: Some(vec!["bash".into()]),
        };
        assert!(p.is_tool_allowed("read"));
        assert!(!p.is_tool_allowed("bash"));
    }

    #[test]
    fn effective_max_turns() {
        let mut p = AgentProfile::new("test", "");
        assert_eq!(p.effective_max_turns(), None);
        p.max_turns = Some(100);
        assert_eq!(p.effective_max_turns(), Some(100));
    }

    #[test]
    fn load_profile_from_file_works() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.toml");
        std::fs::write(
            &path,
            r#"
            name = "fromfile"
            description = "Loaded from file"
            system_prompt = "Hi"
            tools = ["read"]
        "#,
        )
        .unwrap();

        let profile = load_profile_from_file(&path).unwrap();
        assert_eq!(profile.name, "fromfile");
    }

    #[test]
    fn load_profiles_from_dir_with_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("a.toml"),
            r#"
            name = "alpha"
            description = "First"
            system_prompt = "x"
            tools = []
        "#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("b.toml"),
            r#"
            name = "bravo"
            description = "Second"
            system_prompt = "y"
            tools = []
        "#,
        )
        .unwrap();
        std::fs::write(dir.path().join("c.txt"), "ignored").unwrap();

        let profiles = load_profiles_from_dir(dir.path()).unwrap();
        assert_eq!(profiles.len(), 2);
        assert_eq!(profiles[0].name, "alpha");
        assert_eq!(profiles[1].name, "bravo");
    }

    #[test]
    fn load_profiles_skips_invalid_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("good.toml"),
            r#"
            name = "good"
            description = "x"
            system_prompt = "y"
            tools = []
        "#,
        )
        .unwrap();
        std::fs::write(dir.path().join("bad.toml"), "this is invalid toml ====").unwrap();

        let profiles = load_profiles_from_dir(dir.path()).unwrap();
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].name, "good");
    }
}
