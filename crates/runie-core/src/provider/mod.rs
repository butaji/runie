//! Provider layer: `StreamFn` trait and `ProviderActor`.

pub mod actor;
pub mod http;
pub mod replay;
pub mod stream_fn;

pub use crate::types::{Model, SimpleStreamOptions};
pub use actor::{ProviderActor, ProviderCommand};
pub use http::{provider_retry_delay_ms, HttpActor, HttpRequest, HttpResponse, ReplayHttpActor};
pub use replay::ReplayProvider;
pub use stream_fn::{AssistantMessageEventStream, StreamError, StreamFn};
