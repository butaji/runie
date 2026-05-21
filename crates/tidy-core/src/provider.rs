use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum ProviderError {
    #[error("api error: {0}")]
    ApiError(String),
    #[error("serialization error: {0}")]
    SerializationError(String),
    #[error("rate limited")]
    RateLimited,
    #[error("invalid response: {0}")]
    InvalidResponse(String),
}
