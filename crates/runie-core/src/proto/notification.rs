//! Protocol notification envelope.

use serde_json::Value;

use super::version::Version;

/// A one-way notification from server to client.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Notification {
    pub method: String,
    #[serde(default)]
    pub params: Value,
    #[serde(default)]
    pub version: Version,
}

impl Notification {
    /// Create a new notification with the current protocol version.
    pub fn new(method: impl Into<String>, params: Value) -> Self {
        Self {
            method: method.into(),
            params,
            version: Version::current(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notification_roundtrips() {
        let n = Notification::new("progress", serde_json::json!({"done": true}));
        let json = serde_json::to_string(&n).unwrap();
        let parsed: Notification = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.method, "progress");
        assert_eq!(parsed.params["done"], true);
    }
}
