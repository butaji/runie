//! Top-level protocol message enum.

use serde_json::Value;

use crate::{error::Error, notification::Notification, request::Request, response::Response, version::Version};

/// Any protocol message on the wire.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Message {
    Request(Request),
    Response(Response),
    Notification(Notification),
}

impl Message {
    /// Build a request message.
    pub fn request(method: impl Into<String>, params: Value) -> Self {
        Self::Request(Request::new(method, params))
    }

    /// Build a response message.
    pub fn response(id: Option<Value>, result: Value) -> Self {
        Self::Response(Response::ok(id, result))
    }

    /// Build an error message.
    pub fn error(id: Option<Value>, error: Error) -> Self {
        Self::Response(Response::err(id, error))
    }

    /// Build a notification message.
    pub fn notification(method: impl Into<String>, params: Value) -> Self {
        Self::Notification(Notification::new(method, params))
    }

    /// The protocol version carried by this message.
    pub fn version(&self) -> &Version {
        match self {
            Self::Request(r) => &r.version,
            Self::Response(r) => &r.version,
            Self::Notification(n) => &n.version,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_request_roundtrips() {
        let msg = Message::request("initialize", serde_json::json!({}));
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: Message = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, Message::Request(_)));
        assert_eq!(*parsed.version(), Version::current());
    }

    #[test]
    fn message_response_roundtrips() {
        let msg = Message::response(Some(1.into()), serde_json::json!({"ok": true}));
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: Message = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, Message::Response(_)));
    }

    #[test]
    fn message_notification_roundtrips() {
        let msg = Message::notification("progress", serde_json::json!({"done": true}));
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: Message = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, Message::Notification(_)));
    }
}
