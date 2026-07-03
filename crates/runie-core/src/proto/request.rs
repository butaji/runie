//! Protocol request envelope.

use serde_json::Value;

use super::version::Version;

/// A request sent from a client to the server.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Request {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Value,
    #[serde(default)]
    pub version: Version,
}

impl Request {
    /// Create a new request with the current protocol version.
    pub fn new(method: impl Into<String>, params: Value) -> Self {
        Self {
            id: None,
            method: method.into(),
            params,
            version: Version::current(),
        }
    }

    /// Set the request id.
    pub fn with_id(mut self, id: Value) -> Self {
        self.id = Some(id);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_roundtrips_with_version() {
        let req = Request::new("initialize", serde_json::json!({})).with_id(1.into());
        let json = serde_json::to_string(&req).unwrap();
        let parsed: Request = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.method, "initialize");
        assert_eq!(parsed.id, Some(1.into()));
        assert_eq!(parsed.version, Version::current());
    }
}
