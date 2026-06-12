//! Extension system errors

use thiserror::Error;

/// Extension system errors
#[derive(Debug, Error)]
pub enum ExtError {
    #[error("Extension already registered: {0}")]
    AlreadyRegistered(String),

    #[error("Extension not found: {0}")]
    NotFound(String),

    #[error("Failed to load extension {0}: {1}")]
    LoadFailed(String, String),

    #[error("Failed to unload extension {0}: {1}")]
    UnloadFailed(String, String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Communication error: {0}")]
    CommunicationError(String),

    #[error("Hook error: {0}")]
    HookError(String),

    #[error("Skill error: {0}")]
    SkillError(String),

    #[error("MCP error: {0}")]
    McpError(String),

    #[error("Marketplace error: {0}")]
    MarketplaceError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),
}

impl From<std::io::Error> for ExtError {
    fn from(e: std::io::Error) -> Self {
        ExtError::IoError(e.to_string())
    }
}

impl From<serde_json::Error> for ExtError {
    fn from(e: serde_json::Error) -> Self {
        ExtError::ParseError(e.to_string())
    }
}

impl From<toml::de::Error> for ExtError {
    fn from(e: toml::de::Error) -> Self {
        ExtError::ParseError(e.to_string())
    }
}
