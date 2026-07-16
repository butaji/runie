//! Top-level protocol message enum.

use std::sync::Arc;

use serde_json::Value;

use crate::hidden_params::{AsHiddenParams, HiddenParams};
use super::error::Error;
use super::notification::Notification;
use super::request::Request;
use super::response::Response;
use super::version::Version;

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

/// A `Message` with attached hidden parameters.
///
/// Provides `AsHiddenParams` implementation while keeping `Message` serializable
/// as a plain protocol message. For storing hidden params on `Message`, use this
/// wrapper instead of adding fields to the enum directly.
#[derive(Debug, Clone)]
pub struct MessageWithHiddenParams {
    /// The underlying message.
    pub message: Message,
    /// Hidden parameters attached to this message.
    hidden: Arc<HiddenParams>,
}

impl MessageWithHiddenParams {
    /// Create a new wrapper with hidden parameters.
    pub fn new(message: Message, params: HiddenParams) -> Self {
        Self {
            message,
            hidden: Arc::new(params),
        }
    }

    /// Create a new wrapper with hidden parameters (arc form).
    pub fn with_params(message: Message, params: Arc<HiddenParams>) -> Self {
        Self { message, hidden: params }
    }

    /// Wrap a message with no hidden parameters.
    pub fn from_message(message: Message) -> Self {
        Self {
            message,
            hidden: Arc::new(HiddenParams::default()),
        }
    }

    /// Set hidden parameters.
    pub fn set_hidden_params(&mut self, params: Arc<HiddenParams>) {
        self.hidden = params;
    }
}

impl AsHiddenParams for Message {
    fn hidden_params(&self) -> Option<&Arc<HiddenParams>> {
        None
    }
}

impl AsHiddenParams for MessageWithHiddenParams {
    fn hidden_params(&self) -> Option<&Arc<HiddenParams>> {
        if self.hidden.response_cost.is_none()
            && self.hidden.api_base.is_none()
            && self.hidden.original_model.is_none()
            && self.hidden.additional_headers.is_empty()
        {
            None
        } else {
            Some(&self.hidden)
        }
    }

    fn hidden_params_mut(&mut self) -> Option<&mut Arc<HiddenParams>> {
        Some(&mut self.hidden)
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
