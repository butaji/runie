//! Protocol response envelope.

use serde_json::Value;

use crate::{error::Error, version::Version};

/// A response sent from the server to a client.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Response {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<Error>,
    #[serde(default)]
    pub version: Version,
}

impl Response {
    /// Create a successful response with the current protocol version.
    pub fn ok(id: Option<Value>, result: Value) -> Self {
        Self {
            id,
            result: Some(result),
            error: None,
            version: Version::current(),
        }
    }

    /// Create an error response with the current protocol version.
    pub fn err(id: Option<Value>, error: Error) -> Self {
        Self {
            id,
            result: None,
            error: Some(error),
            version: Version::current(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_ok_roundtrips() {
        let resp = Response::ok(Some(1.into()), serde_json::json!({ "ok": true }));
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: Response = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, Some(1.into()));
        assert_eq!(parsed.result.unwrap()["ok"], true);
    }
}
