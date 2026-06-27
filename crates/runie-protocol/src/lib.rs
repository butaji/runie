//! Shared protocol types for Runie IPC.

pub mod error;
pub mod event;
pub mod message;
pub mod messages;
pub mod notification;
pub mod op;
pub mod provider;
pub mod request;
pub mod response;
pub mod version;

pub use error::Error;
pub use event::{ErrorCode, Event, EventMsg};
pub use message::{ChatMessage, MessageMetadata, Part, Role, ToolCall};
pub use messages::Message;
pub use notification::Notification;
pub use op::{
    ApprovalDecision, ApprovalId, Op, PromptOrigin, SessionConfig, Submission, SubmissionId,
    W3cTraceContext,
};
pub use provider::{ProviderConfig, ProviderConfigBox};
pub use request::Request;
pub use response::Response;
pub use version::{Version, PROTOCOL_VERSION};
