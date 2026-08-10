use super::*;
impl TelemetryActor {
    fn handle_start(
        state: &mut TelemetrySnapshot,
        snapshot_tx: &watch::Sender<TelemetrySnapshot>,
        parent_id: Option<u64>,
        name: String,
        attributes: HashMap<String, serde_json::Value>,
        reply: oneshot::Sender<Option<u64>>,
    ) {
        if !validate_telemetry_attributes(&attributes)
            || parent_id.is_some_and(|parent| {
                !state
                    .spans
                    .iter()
                    .any(|span| span.id == parent && !span.ended)
            })
        {
            let _ = reply.send(None);
            return;
        }
        let id = state.next_id;
        state.next_id = state.next_id.wrapping_add(1);
        state.spans.push(SpanSnapshot {
            id,
            parent_id,
            name,
            attributes,
            events: Vec::new(),
            status: SpanStatus::Ok,
            explicit_status: false,
            error: None,
            ended: false,
            end_sequence: None,
        });
        let _ = snapshot_tx.send(state.clone());
        let _ = reply.send(Some(id));
    }

    fn handle_event(
        state: &mut TelemetrySnapshot,
        snapshot_tx: &watch::Sender<TelemetrySnapshot>,
        id: u64,
        name: String,
        attributes: HashMap<String, serde_json::Value>,
        reply: oneshot::Sender<()>,
    ) {
        if validate_telemetry_attributes(&attributes) {
            if let Some(span) = state
                .spans
                .iter_mut()
                .find(|span| span.id == id && !span.ended && span.name != "pi.ai.request")
            {
                span.events
                    .push(TelemetryEventSnapshot { name, attributes });
                let _ = snapshot_tx.send(state.clone());
            }
        }
        let _ = reply.send(());
    }

    fn handle_attributes(
        state: &mut TelemetrySnapshot,
        snapshot_tx: &watch::Sender<TelemetrySnapshot>,
        id: u64,
        attributes: HashMap<String, serde_json::Value>,
        reply: oneshot::Sender<()>,
    ) {
        if validate_telemetry_attributes(&attributes) {
            if let Some(span) = state
                .spans
                .iter_mut()
                .find(|span| span.id == id && !span.ended)
            {
                span.attributes.extend(attributes);
                let _ = snapshot_tx.send(state.clone());
            }
        }
        let _ = reply.send(());
    }

    fn handle_status(
        state: &mut TelemetrySnapshot,
        snapshot_tx: &watch::Sender<TelemetrySnapshot>,
        id: u64,
        status: SpanStatus,
        error: Option<SpanError>,
        reply: oneshot::Sender<()>,
    ) {
        if let Some(span) = state
            .spans
            .iter_mut()
            .find(|span| span.id == id && !span.ended)
        {
            span.status = status;
            span.error = error;
            span.explicit_status = true;
            let _ = snapshot_tx.send(state.clone());
        }
        let _ = reply.send(());
    }

    async fn handle_end(
        state: &mut TelemetrySnapshot,
        snapshot_tx: &watch::Sender<TelemetrySnapshot>,
        exporter: &Option<Arc<dyn TelemetryExporter>>,
        next_end_sequence: &mut u64,
        id: u64,
        reply: oneshot::Sender<()>,
    ) {
        if let Some(span) = state
            .spans
            .iter_mut()
            .find(|span| span.id == id && !span.ended)
        {
            span.ended = true;
            span.end_sequence = Some(*next_end_sequence);
            *next_end_sequence = next_end_sequence.wrapping_add(1);
            let _ = snapshot_tx.send(state.clone());
            if let Some(exporter) = exporter.as_ref() {
                let _ = exporter.export(state.clone()).await;
            }
        }
        let _ = reply.send(());
    }

    async fn run_telemetry_worker(
        mut rx: mpsc::Receiver<TelemetryCommand>,
        snapshot_tx: watch::Sender<TelemetrySnapshot>,
        exporter: Option<Arc<dyn TelemetryExporter>>,
    ) {
        let mut state = TelemetrySnapshot {
            next_id: 1,
            ..TelemetrySnapshot::default()
        };
        let mut next_end_sequence = 0_u64;
        while let Some(command) = rx.recv().await {
            Self::dispatch_command(
                &mut state,
                &snapshot_tx,
                &exporter,
                &mut next_end_sequence,
                command,
            )
            .await;
        }
    }

    async fn dispatch_command(
        state: &mut TelemetrySnapshot,
        snapshot_tx: &watch::Sender<TelemetrySnapshot>,
        exporter: &Option<Arc<dyn TelemetryExporter>>,
        next_end_sequence: &mut u64,
        command: TelemetryCommand,
    ) {
        match command {
            TelemetryCommand::Start {
                parent_id,
                name,
                attributes,
                reply,
            } => Self::handle_start(state, snapshot_tx, parent_id, name, attributes, reply),
            TelemetryCommand::Event {
                id,
                name,
                attributes,
                reply,
            } => Self::handle_event(state, snapshot_tx, id, name, attributes, reply),
            TelemetryCommand::SetAttributes {
                id,
                attributes,
                reply,
            } => Self::handle_attributes(state, snapshot_tx, id, attributes, reply),
            TelemetryCommand::Status {
                id,
                status,
                error,
                reply,
            } => Self::handle_status(state, snapshot_tx, id, status, error, reply),
            TelemetryCommand::End { id, reply } => {
                Self::handle_end(state, snapshot_tx, exporter, next_end_sequence, id, reply).await
            }
        }
    }

    #[allow(
        clippy::too_many_lines,
        reason = "the actor reducer keeps the complete span lifecycle in one explicit boundary"
    )]
    pub fn new() -> Self {
        Self::new_with_exporter(None)
    }

    #[allow(
        clippy::too_many_lines,
        reason = "the telemetry actor keeps its complete reducer and exporter settlement boundary together"
    )]
    pub fn new_with_exporter(exporter: Option<Arc<dyn TelemetryExporter>>) -> Self {
        let (snapshot_tx, snapshot) = watch::channel(TelemetrySnapshot::default());
        let (tx, owner) =
            spawn_actor_worker!(32, move |rx: mpsc::Receiver<TelemetryCommand>| async move {
                TelemetryActor::run_telemetry_worker(rx, snapshot_tx.clone(), exporter.clone())
                    .await;
            });
        Self {
            tx,
            snapshot,
            _owner: owner,
        }
    }

    pub fn snapshot(&self) -> TelemetrySnapshot {
        self.snapshot.borrow().clone()
    }

    /// Apply one declarative replay action through the actor mailbox.
    pub async fn apply(&self, action: TelemetryAction) -> Option<u64> {
        match action {
            TelemetryAction::Start {
                parent_id,
                name,
                attributes,
            } => self
                .start_span(parent_id, name, attributes)
                .await
                .map(|span| span.id),
            TelemetryAction::Event {
                id,
                name,
                attributes,
            } => self.apply_event(id, name, attributes).await,
            TelemetryAction::SetAttributes { id, attributes } => {
                self.apply_attributes(id, attributes).await
            }
            TelemetryAction::Status { id, status, error } => {
                self.apply_status(id, status, error).await
            }
            TelemetryAction::End { id } => self.apply_end(id).await,
        }
    }

    async fn apply_event(
        &self,
        id: u64,
        name: String,
        attributes: HashMap<String, serde_json::Value>,
    ) -> Option<u64> {
        self.span(id).event(name, attributes).await;
        None
    }

    async fn apply_attributes(
        &self,
        id: u64,
        attributes: HashMap<String, serde_json::Value>,
    ) -> Option<u64> {
        self.span(id).set_attributes(attributes).await;
        None
    }

    async fn apply_status(
        &self,
        id: u64,
        status: SpanStatus,
        error: Option<SpanError>,
    ) -> Option<u64> {
        self.span(id).status_with_error(status, error).await;
        None
    }

    async fn apply_end(&self, id: u64) -> Option<u64> {
        self.span(id).end().await;
        None
    }

    fn span(&self, id: u64) -> TelemetrySpan {
        TelemetrySpan {
            actor: self.clone(),
            id,
            noop: false,
        }
    }

    pub async fn replay(&self, actions: impl IntoIterator<Item = TelemetryAction>) {
        for action in actions {
            let _ = self.apply(action).await;
        }
    }

    pub async fn start_span(
        &self,
        parent_id: Option<u64>,
        name: impl Into<String>,
        attributes: HashMap<String, serde_json::Value>,
    ) -> Option<TelemetrySpan> {
        let (reply, result) = oneshot::channel();
        self.tx
            .send(TelemetryCommand::Start {
                parent_id,
                name: name.into(),
                attributes,
                reply,
            })
            .await
            .ok()?;
        Some(TelemetrySpan {
            actor: self.clone(),
            id: result.await.ok()??,
            noop: false,
        })
    }

    /// Execute a callback inside an actor-owned span, matching Pi's
    /// callback-scoped `startSpan` contract. Completion always settles the
    /// span through mailbox commands before the result is returned.
    pub async fn with_span<F, Fut, T, E>(
        &self,
        parent_id: Option<u64>,
        name: impl Into<String>,
        attributes: HashMap<String, serde_json::Value>,
        callback: F,
    ) -> Option<Result<T, E>>
    where
        F: FnOnce(TelemetrySpan) -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: std::fmt::Display,
    {
        let span = self
            .start_span(parent_id, name, attributes)
            .await
            .unwrap_or_else(|| TelemetrySpan {
                actor: self.clone(),
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

    /// Execute a synchronous callback inside an actor-owned span. Pi accepts
    /// both callback return shapes; keep the synchronous path explicit so the
    /// callback itself never needs to manufacture an async wrapper.
    pub async fn with_span_sync<F, T, E>(
        &self,
        parent_id: Option<u64>,
        name: impl Into<String>,
        attributes: HashMap<String, serde_json::Value>,
        callback: F,
    ) -> Option<Result<T, E>>
    where
        F: FnOnce(TelemetrySpan) -> Result<T, E>,
        E: std::fmt::Display,
    {
        let span = self
            .start_span(parent_id, name, attributes)
            .await
            .unwrap_or_else(|| TelemetrySpan {
                actor: self.clone(),
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

impl Default for TelemetryActor {
    fn default() -> Self {
        Self::new()
    }
}
