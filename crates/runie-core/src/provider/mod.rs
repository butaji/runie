//! Provider layer: `StreamFn` trait and `ProviderActor`.

pub mod actor;
pub mod codex;
pub mod http;
pub mod replay;
pub mod stream_fn;

pub use crate::types::{Model, ProviderTransport, SimpleStreamOptions};
pub use actor::{ProviderActor, ProviderCommand};
pub use http::{
    mapped_reasoning, provider_retry_delay_ms, provider_retry_delay_ms_with_jitter_at,
    with_model_effort, with_model_provider_effort, with_provider_effort, EffortWireField,
    HttpActor, HttpRequest, HttpResponse, ProviderRequestProfile, ReplayHttpActor,
};
pub use replay::response_usage;
pub use replay::ReplayProvider;
pub use stream_fn::{
    classify_failure, AssistantMessageEventStream, ProviderFailure, ProviderFailureKind,
    StreamError, StreamFn, WebSocketAdapter,
};
