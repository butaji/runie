//! Sandbox profile definitions and resolution.
//!
//! Provides the `Profile` enum and `SandboxConfig` with profile-specific
//! defaults. Also handles loading custom profiles from `sandbox.toml`.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Valid sandbox profile names.
pub const PROFILE_NAMES: &[&str] = &["off", "workspace", "strict", "devbox", "custom"];

/// Sandbox profile type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Profile {
    /// No sandboxing - full access.
    #[default]
    Off,
    /// Workspace profile - read all, write to workspace only.
    Workspace,
    /// Strict profile - explicit allowlist, network blocked.
    Strict,
    /// Devbox profile - wide write access.
    Devbox,
    /// Custom profile loaded from config.
    Custom,
}

impl Profile {
    /// Parse profile from string.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "off" | "none" => Some(Profile::Off),
            "workspace" => Some(Profile::Workspace),
            "strict" => Some(Profile::Strict),
            "devbox" => Some(Profile::Devbox),
            "custom" => Some(Profile::Custom),
            _ => None,
        }
    }

    /// Get profile name for display.
    pub fn name(&self) -> &'static str {
        match self {
            Profile::Off => "off",
            Profile::Workspace => "workspace",
            Profile::Strict => "strict",
            Profile::Devbox => "devbox",
            Profile::Custom => "custom",
        }
    }

    /// Check if this profile requires network access.
    pub fn allows_network(&self) -> bool {
        !matches!(self, Profile::Strict)
    }

    /// Check if this profile restricts file writes.
    pub fn restricts_writes(&self) -> bool {
        matches!(self, Profile::Strict | Profile::Workspace)
    }
}

impl std::fmt::Display for Profile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl Serialize for Profile {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.name())
    }
}

impl<'de> Deserialize<'de> for Profile {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Profile::parse(&s).ok_or_else(|| {
            serde::de::Error::custom(format!("invalid profile: {}", s))
        })
    }
}

/// Sandbox configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SandboxConfig {
    /// Active profile.
    #[serde(skip)]
    pub profile: Profile,
    /// Workspace root for sandboxing.
    #[serde(rename = "workspaceRoot")]
    pub workspace_root: Option<PathBuf>,
    /// Explicit read-write paths.
    #[serde(rename = "readWritePaths")]
    pub read_write_paths: Vec<PathBuf>,
    /// Explicit read-only paths.
    #[serde(rename = "readOnlyPaths")]
    pub read_only_paths: Vec<PathBuf>,
    /// Deny patterns (glob).
    #[serde(rename = "denyPatterns")]
    pub deny_patterns: Vec<String>,
    /// Whether to restrict network.
    #[serde(rename = "restrictNetwork")]
    pub restrict_network: bool,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            profile: Profile::Off,
            workspace_root: None,
            read_write_paths: Vec::new(),
            read_only_paths: Vec::new(),
            deny_patterns: Vec::new(),
            restrict_network: false,
        }
    }
}

impl SandboxConfig {
    /// Create config for a workspace profile.
    pub fn workspace(workspace_root: PathBuf) -> Self {
        Self {
            profile: Profile::Workspace,
            workspace_root: Some(workspace_root),
            read_write_paths: vec![],
            read_only_paths: vec![
                PathBuf::from("/usr"),
                PathBuf::from("/System"),
                PathBuf::from("/bin"),
                PathBuf::from("/lib"),
            ],
            deny_patterns: vec![],
            restrict_network: false,
        }
    }

    /// Create config for a strict profile.
    pub fn strict(workspace_root: PathBuf) -> Self {
        Self {
            profile: Profile::Strict,
            workspace_root: Some(workspace_root.clone()),
            read_write_paths: vec![workspace_root],
            read_only_paths: vec![
                PathBuf::from("/usr"),
                PathBuf::from("/System"),
                PathBuf::from("/bin"),
                PathBuf::from("/lib"),
                PathBuf::from("/etc"),
            ],
            deny_patterns: vec![],
            restrict_network: true,
        }
    }

    /// Create config for devbox profile.
    pub fn devbox(workspace_root: PathBuf) -> Self {
        Self {
            profile: Profile::Devbox,
            workspace_root: Some(workspace_root.clone()),
            read_write_paths: vec![
                workspace_root,
                PathBuf::from("/tmp"),
                dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp")),
            ],
            read_only_paths: vec![],
            deny_patterns: vec!["/data".into()],
            restrict_network: false,
        }
    }

    /// Create config for a custom profile.
    pub fn custom(
        workspace_root: Option<PathBuf>,
        read_write_paths: Vec<PathBuf>,
        read_only_paths: Vec<PathBuf>,
        deny_patterns: Vec<String>,
        restrict_network: bool,
    ) -> Self {
        Self {
            profile: Profile::Custom,
            workspace_root,
            read_write_paths,
            read_only_paths,
            deny_patterns,
            restrict_network,
        }
    }

    /// Update the active profile.
    pub fn with_profile(mut self, profile: Profile) -> Self {
        self.profile = profile;
        self
    }

    /// Get the workspace root or default to current directory.
    pub fn workspace_root_or_default(&self) -> PathBuf {
        self.workspace_root.clone().unwrap_or_else(|| PathBuf::from("."))
    }
}

/// Profile configuration loaded from sandbox.toml.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileConfig {
    /// Profile name.
    pub name: String,
    /// Read-write paths.
    #[serde(default)]
    pub read_write_paths: Vec<String>,
    /// Read-only paths.
    #[serde(default)]
    pub read_only_paths: Vec<String>,
    /// Deny patterns.
    #[serde(default)]
    pub deny_patterns: Vec<String>,
    /// Restrict network.
    #[serde(default)]
    pub restrict_network: bool,
}

/// Resolve a profile from config string or file.
///
/// If `config_str` is provided, parse it as the profile name.
/// Otherwise, try to load from `sandbox_path` if it exists.
pub fn resolve_profile(
    config_str: Option<&str>,
    sandbox_path: Option<&Path>,
) -> Result<(Profile, SandboxConfig), String> {
    // First try to parse from config string
    if let Some(s) = config_str {
        if let Some(profile) = Profile::parse(s) {
            let workspace = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let config = match profile {
                Profile::Off => SandboxConfig::default(),
                Profile::Workspace => SandboxConfig::workspace(workspace),
                Profile::Strict => SandboxConfig::strict(workspace),
                Profile::Devbox => SandboxConfig::devbox(workspace),
                Profile::Custom => {
                    return Err("Custom profile requires sandbox.toml".to_owned());
                }
            };
            return Ok((profile, config));
        }
    }

    // Try to load from sandbox.toml
    if let Some(path) = sandbox_path {
        if path.exists() {
            return load_profile_from_file(path);
        }
    }

    // Default to off
    Ok((Profile::Off, SandboxConfig::default()))
}

/// Load a custom profile from a sandbox.toml file.
pub fn load_profile_from_file(path: &Path) -> Result<(Profile, SandboxConfig), String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read sandbox config: {}", e))?;

    let profile_config: ProfileConfig = toml::from_str(&content)
        .map_err(|e| format!("Failed to parse sandbox config: {}", e))?;

    let profile = Profile::parse(&profile_config.name)
        .unwrap_or(Profile::Custom);

    let config = SandboxConfig::custom(
        None,
        profile_config
            .read_write_paths
            .into_iter()
            .map(PathBuf::from)
            .collect(),
        profile_config
            .read_only_paths
            .into_iter()
            .map(PathBuf::from)
            .collect(),
        profile_config.deny_patterns,
        profile_config.restrict_network,
    );

    Ok((profile, config))
}

/// Get the default sandbox config path (`.runie/sandbox.toml`).
#[allow(dead_code)]
pub fn default_sandbox_path() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("runie").join("sandbox.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_parse() {
        assert_eq!(Profile::parse("off"), Some(Profile::Off));
        assert_eq!(Profile::parse("none"), Some(Profile::Off));
        assert_eq!(Profile::parse("OFF"), Some(Profile::Off));
        assert_eq!(Profile::parse("workspace"), Some(Profile::Workspace));
        assert_eq!(Profile::parse("strict"), Some(Profile::Strict));
        assert_eq!(Profile::parse("devbox"), Some(Profile::Devbox));
        assert_eq!(Profile::parse("custom"), Some(Profile::Custom));
        assert_eq!(Profile::parse("unknown"), None);
    }

    #[test]
    fn profile_name() {
        assert_eq!(Profile::Off.name(), "off");
        assert_eq!(Profile::Workspace.name(), "workspace");
        assert_eq!(Profile::Strict.name(), "strict");
        assert_eq!(Profile::Devbox.name(), "devbox");
        assert_eq!(Profile::Custom.name(), "custom");
    }

    #[test]
    fn profile_allows_network() {
        assert!(Profile::Off.allows_network());
        assert!(Profile::Workspace.allows_network());
        assert!(!Profile::Strict.allows_network());
        assert!(Profile::Devbox.allows_network());
        assert!(Profile::Custom.allows_network());
    }

    #[test]
    fn sandbox_config_workspace() {
        let config = SandboxConfig::workspace("/tmp/test".into());
        assert_eq!(config.profile, Profile::Workspace);
        assert!(config.workspace_root.is_some());
        assert!(!config.restrict_network);
    }

    #[test]
    fn sandbox_config_strict() {
        let config = SandboxConfig::strict("/tmp/test".into());
        assert_eq!(config.profile, Profile::Strict);
        assert!(config.restrict_network);
    }

    #[test]
    fn sandbox_config_devbox() {
        let config = SandboxConfig::devbox("/tmp/test".into());
        assert_eq!(config.profile, Profile::Devbox);
        assert!(config.deny_patterns.contains(&"/data".to_string()));
    }

    #[test]
    fn sandbox_config_custom() {
        let config = SandboxConfig::custom(
            Some("/workspace".into()),
            vec!["/tmp".into()],
            vec!["/usr".into()],
            vec!["*.secret".into()],
            true,
        );
        assert_eq!(config.profile, Profile::Custom);
        assert!(config.restrict_network);
        assert_eq!(config.deny_patterns.len(), 1);
    }

    #[test]
    fn resolve_profile_from_string() {
        let (profile, config) = resolve_profile(Some("workspace"), None).unwrap();
        assert_eq!(profile, Profile::Workspace);
        assert_eq!(config.profile, Profile::Workspace);
    }

    #[test]
    fn resolve_profile_default_off() {
        let (profile, config) = resolve_profile(None, None).unwrap();
        assert_eq!(profile, Profile::Off);
        assert_eq!(config.profile, Profile::Off);
    }

    #[test]
    fn profile_display() {
        assert_eq!(format!("{}", Profile::Off), "off");
        assert_eq!(format!("{}", Profile::Strict), "strict");
    }
}
