use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum ProviderError {
    #[error("api error: {0}")]
    ApiError(String),
    #[error("serialization error: {0}")]
    SerializationError(String),
    #[error("rate limited")]
    RateLimited,
    #[error("rate limited, retry after {0} seconds")]
    RateLimitedRetryAfter(u64),
    #[error("invalid response: {0}")]
    InvalidResponse(String),
}

impl ProviderError {
    /// Returns the retry-after duration in seconds if this is a rate limit error
    pub fn retry_after_seconds(&self) -> Option<u64> {
        match self {
            ProviderError::RateLimitedRetryAfter(seconds) => Some(*seconds),
            _ => None,
        }
    }

    /// Returns true if this is a rate limit error (with or without retry-after)
    pub fn is_rate_limited(&self) -> bool {
        matches!(self, ProviderError::RateLimited | ProviderError::RateLimitedRetryAfter(_))
    }
}
