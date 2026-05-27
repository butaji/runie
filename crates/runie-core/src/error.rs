use serde::{Deserialize, Serialize};

#[derive(thiserror::Error, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RunieError {
    #[error("{0}")]
    Provider(String),
    #[error("{0}")]
    Agent(String),
    #[error("{0}")]
    Tool(String),
    #[error("IO: {0}")]
    Io(String),
    #[error("Config: {0}")]
    Config(String),
    #[error("Permission denied: {0}")]
    Permission(String),
}

impl From<std::io::Error> for RunieError {
    fn from(e: std::io::Error) -> Self {
        RunieError::Io(e.to_string())
    }
}
