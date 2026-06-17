//! Protocol errors.

use serde_json::Value;
use std::fmt;

/// Typed protocol error.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Error {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

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

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "protocol error {}: {}", self.code, self.message)
    }
}

impl std::error::Error for Error {}

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
}
