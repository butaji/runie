//! Protocol errors.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

/// Typed protocol error.
/// Note: This struct uses manual Display/Error implementations because
/// `thiserror` cannot derive Display for fields containing `serde_json::Value`
/// which doesn't implement Display.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Error {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "protocol error {}: {}", self.code, self.message)
    }
}

impl std::error::Error for Error {}

impl Error {
    /// Build a parse error.
    pub fn parse(message: impl Into<String>) -> Self {
        Self {
            code: -32700,
            message: message.into(),
            data: None,
        }
    }

    /// Build a method-not-found error.
    pub fn method_not_found(method: impl Into<String>) -> Self {
        Self {
            code: -32601,
            message: method.into(),
            data: None,
        }
    }

    /// Build an internal error.
    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            code: -32603,
            message: message.into(),
            data: None,
        }
    }
}

impl From<anyhow::Error> for Error {
    fn from(e: anyhow::Error) -> Self {
        Error::internal(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_serializes_code_and_message() {
        let err = Error::internal("boom");
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["code"], -32603);
        assert_eq!(json["message"], "boom");
    }

    #[test]
    fn error_display_includes_code_and_message() {
        let err = Error::internal("something went wrong");
        let display = format!("{}", err);
        assert!(display.contains("-32603"));
        assert!(display.contains("something went wrong"));
    }

    #[test]
    fn error_is_error_trait() {
        let err: Box<dyn std::error::Error> = Error::internal("test").into();
        assert!(err.to_string().contains("test"));
    }
}
