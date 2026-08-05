//! Minimal transport boundary used by concrete providers and replay tests.

use std::{fs, path::Path};

use super::stream_fn::StreamError;

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub body: String,
}

#[async_trait::async_trait]
pub trait HttpActor: Send + Sync + 'static {
    async fn post(&self, body: String) -> Result<HttpResponse, StreamError>;
}

/// Serves one recorded HTTP response body without opening a socket.
pub struct ReplayHttpActor {
    response: HttpResponse,
}

impl ReplayHttpActor {
    pub fn from_sse(path: impl AsRef<Path>) -> Result<Self, StreamError> {
        let body = fs::read_to_string(path).map_err(|e| StreamError::Network(e.to_string()))?;
        Ok(Self {
            response: HttpResponse { status: 200, body },
        })
    }
}

#[async_trait::async_trait]
impl HttpActor for ReplayHttpActor {
    async fn post(&self, _body: String) -> Result<HttpResponse, StreamError> {
        if self.response.status >= 400 {
            return Err(StreamError::Api(format!("HTTP {}", self.response.status)));
        }
        Ok(self.response.clone())
    }
}
