//! Provider layer: `StreamFn` trait and `ProviderActor`.

pub mod actor;
pub mod replay;
pub mod stream_fn;

pub use replay::ReplayProvider;
pub use crate::types::{Model, SimpleStreamOptions};
pub use actor::{ProviderActor, ProviderCommand};
pub use stream_fn::{AssistantMessageEventStream, StreamError, StreamFn};
