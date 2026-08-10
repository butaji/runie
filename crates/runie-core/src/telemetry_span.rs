use super::*;
#[derive(Clone)]
pub struct TelemetrySpan {
    pub(super) actor: TelemetryActor,
    pub id: u64,
    pub(super) noop: bool,
}

impl TelemetrySpan {
    pub(super) fn explicit_status(&self) -> bool {
        self.actor
            .snapshot()
            .spans
            .iter()
            .find(|span| span.id == self.id)
            .is_some_and(|span| span.explicit_status)
    }

    pub async fn event(
        &self,
        name: impl Into<String>,
        attributes: HashMap<String, serde_json::Value>,
    ) {
        if self.noop {
            return;
        }
        let (reply, acknowledged) = oneshot::channel();
        let _ = self
            .actor
            .tx
            .send(TelemetryCommand::Event {
                id: self.id,
                name: name.into(),
                attributes,
                reply,
            })
            .await;
        let _ = acknowledged.await;
    }

    pub async fn set_attributes(&self, attributes: HashMap<String, serde_json::Value>) {
        if self.noop {
            return;
        }
        let (reply, acknowledged) = oneshot::channel();
        let _ = self
            .actor
            .tx
            .send(TelemetryCommand::SetAttributes {
                id: self.id,
                attributes,
                reply,
            })
            .await;
        let _ = acknowledged.await;
    }

    pub async fn status(&self, status: SpanStatus) {
        self.status_with_error(status, None).await;
    }

    pub async fn status_with_error(&self, status: SpanStatus, error: Option<SpanError>) {
        if self.noop {
            return;
        }
        let (reply, acknowledged) = oneshot::channel();
        let _ = self
            .actor
            .tx
            .send(TelemetryCommand::Status {
                id: self.id,
                status,
                error,
                reply,
            })
            .await;
        let _ = acknowledged.await;
    }

    pub async fn end(&self) {
        if self.noop {
            return;
        }
        let (reply, acknowledged) = oneshot::channel();
        let _ = self
            .actor
            .tx
            .send(TelemetryCommand::End { id: self.id, reply })
            .await;
        let _ = acknowledged.await;
    }

    pub async fn child(
        &self,
        name: impl Into<String>,
        attributes: HashMap<String, serde_json::Value>,
    ) -> Option<Self> {
        self.actor.start_span(Some(self.id), name, attributes).await
    }

    /// Run a nested callback-scoped span through the owning actor.
    pub async fn with_child<F, Fut, T, E>(
        &self,
        name: impl Into<String>,
        attributes: HashMap<String, serde_json::Value>,
        callback: F,
    ) -> Option<Result<T, E>>
    where
        F: FnOnce(TelemetrySpan) -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: std::fmt::Display,
    {
        let child = self.actor.start_span(Some(self.id), name, attributes).await;
        // Pi's settled-span context remains callable: the callback executes,
        // but all operations on the detached child are inert. Preserve that
        // behavior without creating a second recorded span.
        let span = child.unwrap_or_else(|| TelemetrySpan {
            actor: self.actor.clone(),
            id: 0,
            noop: true,
        });
        let result = callback(span.clone()).await;
        if !span.noop {
            match &result {
                Ok(_) => span.status(SpanStatus::Ok).await,
                Err(error) if !span.explicit_status() => {
                    span.status_with_error(
                        SpanStatus::Error,
                        Some(SpanError {
                            name: "Error".to_owned(),
                            message: error.to_string(),
                        }),
                    )
                    .await;
                }
                Err(_) => {}
            }
            span.end().await;
        }
        Some(result)
    }

    /// Synchronous counterpart to [`Self::with_child`], matching Pi's nested
    /// callback contract while retaining the same actor-owned settlement.
    pub async fn with_child_sync<F, T, E>(
        &self,
        name: impl Into<String>,
        attributes: HashMap<String, serde_json::Value>,
        callback: F,
    ) -> Option<Result<T, E>>
    where
        F: FnOnce(TelemetrySpan) -> Result<T, E>,
        E: std::fmt::Display,
    {
        let child = self.actor.start_span(Some(self.id), name, attributes).await;
        let span = child.unwrap_or_else(|| TelemetrySpan {
            actor: self.actor.clone(),
            id: 0,
            noop: true,
        });
        let result = callback(span.clone());
        if !span.noop {
            match &result {
                Ok(_) => span.status(SpanStatus::Ok).await,
                Err(error) if !span.explicit_status() => {
                    span.status_with_error(
                        SpanStatus::Error,
                        Some(SpanError {
                            name: "Error".to_owned(),
                            message: error.to_string(),
                        }),
                    )
                    .await;
                }
                Err(_) => {}
            }
            span.end().await;
        }
        Some(result)
    }
}
