use super::*;
impl LoopActor {
    pub async fn scheduler_metrics(&self) -> crate::tools::SchedulerMetrics {
        self.inner.deps.tool_executor.scheduler_metrics().await
    }

    pub fn mcp_stdio_statuses(&self) -> Vec<crate::tools::McpStdioStatus> {
        self.inner.deps.tool_executor.mcp_stdio_statuses()
    }

    pub fn mcp_http_statuses(&self) -> Vec<crate::tools::McpHttpStatus> {
        self.inner.deps.tool_executor.mcp_http_statuses()
    }

    pub fn mcp_status_rows(&self) -> Vec<crate::tools::McpStatusRow> {
        self.inner.deps.tool_executor.mcp_status_rows()
    }

    /// Project the next context-recovery operation without mutating any
    /// actor. Callers can route `Prepare` through the session owner and keep
    /// summarization in the provider owner.
    pub fn context_recovery_plan(
        messages: &[crate::types::AgentMessage],
        context_window: u64,
        settings: crate::session::CompactionSettings,
    ) -> crate::session::CompactionRecoveryPlan {
        let estimate = crate::session::estimate_context_tokens(messages);
        crate::session::plan_compaction_recovery(estimate.tokens, context_window, settings)
    }

    pub fn new(mut deps: LoopDeps) -> Self {
        let (abort_tx, abort_rx) = tokio::sync::watch::channel(false);
        deps.abort = Some(abort_rx);
        let initial_snapshot = LoopControlSnapshot {
            running: false,
            abort_requested: false,
            steering_mode: deps.steering_mode,
            follow_up_mode: deps.follow_up_mode,
        };
        let (control_tx, control_rx) = watch::channel(initial_snapshot.clone());
        let (shared_control_tx, shared_control_rx) =
            watch::channel(crate::SharedSnapshot::new(initial_snapshot.clone()));
        let (control_commands, control_owner) = spawn_control_worker(
            initial_snapshot,
            abort_tx.clone(),
            control_tx.clone(),
            shared_control_tx,
        );
        Self {
            inner: Arc::new(Inner {
                deps,
                next_run_id: AtomicU64::new(1),
                running: Arc::new(Semaphore::new(1)),
                control_commands,
                control_rx,
                shared_control_rx,
                _control_owner: control_owner,
            }),
        }
    }

    pub async fn prompt(
        &self,
        prompts: Vec<AgentMessage>,
        context: AgentContext,
    ) -> Result<Vec<AgentMessage>, LoopError> {
        self.prompt_with_events(prompts, context)
            .await
            .map(|(messages, _)| messages)
    }

    pub async fn prompt_with_events(
        &self,
        prompts: Vec<AgentMessage>,
        context: AgentContext,
    ) -> Result<(Vec<AgentMessage>, Vec<crate::types::AssistantMessageEvent>), LoopError> {
        // Busy guard: ownership of the single permit spans the complete run
        // (pi agent.ts:340), so no mutable boolean is shared across callers.
        let _run_permit = self.acquire_run().await?;
        self.reduce_control(LoopControlEvent::RunStarted).await;
        let result = self.run_inner_with_events(prompts, context, false).await;
        self.reduce_control(LoopControlEvent::RunFinished).await;
        result
    }

    async fn run_inner(
        &self,
        prompts: Vec<AgentMessage>,
        context: AgentContext,
        skip_initial_steering_poll: bool,
    ) -> Result<Vec<AgentMessage>, LoopError> {
        self.run_inner_with_events(prompts, context, skip_initial_steering_poll)
            .await
            .map(|(messages, _)| messages)
    }

    async fn run_inner_with_events(
        &self,
        prompts: Vec<AgentMessage>,
        context: AgentContext,
        skip_initial_steering_poll: bool,
    ) -> Result<(Vec<AgentMessage>, Vec<crate::types::AssistantMessageEvent>), LoopError> {
        self.sync_context_to_state(&context).await;
        let run_id = format!(
            "run-{}",
            self.inner.next_run_id.fetch_add(1, Ordering::Relaxed)
        );
        let mut deps = self.inner.deps.as_run_loop_deps(run_id);
        let control = self.inner.control_rx.borrow().clone();
        deps.steering_mode = control.steering_mode;
        deps.follow_up_mode = control.follow_up_mode;
        let outcome = run_loop(prompts, context, deps, skip_initial_steering_poll).await;
        Ok((outcome.new_messages, outcome.provider_events))
    }

    async fn sync_context_to_state(&self, context: &AgentContext) {
        if !context.system_prompt.is_empty() {
            self.inner
                .deps
                .state
                .set_system_prompt(context.system_prompt.clone())
                .await;
        }
        if !context.messages.is_empty() {
            self.inner
                .deps
                .state
                .replace_messages(context.messages.clone())
                .await;
        }
        let tools = context
            .tools
            .clone()
            .unwrap_or_else(|| self.inner.deps.tool_executor.tools());
        self.inner.deps.state.set_tools(tools).await;
    }

    pub async fn continue_run(
        &self,
        context: AgentContext,
    ) -> Result<Vec<AgentMessage>, LoopError> {
        let _run_permit = self.acquire_run().await?;
        self.reduce_control(LoopControlEvent::RunStarted).await;
        let result = self.continue_inner(context).await;
        self.reduce_control(LoopControlEvent::RunFinished).await;
        result
    }

    async fn continue_inner(&self, context: AgentContext) -> Result<Vec<AgentMessage>, LoopError> {
        // pi runAgentLoopContinue validation (agent-loop.ts:127,131).
        if context.messages.is_empty() {
            return Err(LoopError::EmptyContext);
        }
        if matches!(context.messages.last(), Some(AgentMessage::Assistant(_))) {
            let steering = self.drain_steering_for_continue().await;
            if !steering.is_empty() {
                return self.run_inner(steering, context, true).await;
            }
            let follow_up = self.drain_follow_up_for_continue().await;
            if !follow_up.is_empty() {
                return self.run_inner(follow_up, context, false).await;
            }
            return Err(LoopError::LastIsAssistant);
        }
        self.run_inner(vec![], context, false).await
    }

    async fn acquire_run(&self) -> Result<OwnedSemaphorePermit, LoopError> {
        self.inner
            .running
            .clone()
            .try_acquire_owned()
            .map_err(|_| LoopError::Busy)
    }

    async fn drain_steering_for_continue(&self) -> Vec<AgentMessage> {
        match self.steering_mode().await {
            QueueMode::OneAtATime => self
                .inner
                .deps
                .steering
                .drain_one()
                .await
                .into_iter()
                .collect(),
            QueueMode::All => self.inner.deps.steering.drain_all().await,
        }
    }

    async fn drain_follow_up_for_continue(&self) -> Vec<AgentMessage> {
        match self.follow_up_mode().await {
            QueueMode::OneAtATime => self
                .inner
                .deps
                .follow_up
                .drain_one()
                .await
                .into_iter()
                .collect(),
            QueueMode::All => self.inner.deps.follow_up.drain_all().await,
        }
    }

    pub async fn steer(&self, msg: AgentMessage) {
        let payload = serde_json::to_value(&msg).unwrap_or(serde_json::Value::Null);
        if let Some(entry_id) = self.inner.deps.steering.push(msg).await {
            let target = provisioned_queue_target(&entry_id, payload);
            publish_queue_record(
                &self.inner.deps.bus,
                crate::types::OperationRecordKind::QueueEnqueued,
                serde_json::json!({
                    "id": entry_id,
                    "queue": "steer",
                    "target": target,
                }),
            );
        }
    }

    pub async fn follow_up(&self, msg: AgentMessage) {
        let payload = serde_json::to_value(&msg).unwrap_or(serde_json::Value::Null);
        if let Some(entry_id) = self.inner.deps.follow_up.push(msg).await {
            let target = provisioned_queue_target(&entry_id, payload);
            publish_queue_record(
                &self.inner.deps.bus,
                crate::types::OperationRecordKind::QueueEnqueued,
                serde_json::json!({
                    "id": entry_id,
                    "queue": "followUp",
                    "target": target,
                }),
            );
        }
    }

    /// Remove all queued steering messages.
    pub async fn clear_steering_queue(&self) {
        for entry_id in self.inner.deps.steering.clear().await {
            publish_queue_record(
                &self.inner.deps.bus,
                crate::types::OperationRecordKind::QueueCancelled,
                serde_json::json!({"id": entry_id, "entryId": entry_id}),
            );
        }
    }

    /// Remove all queued follow-up messages.
    pub async fn clear_follow_up_queue(&self) {
        for entry_id in self.inner.deps.follow_up.clear().await {
            publish_queue_record(
                &self.inner.deps.bus,
                crate::types::OperationRecordKind::QueueCancelled,
                serde_json::json!({"id": entry_id, "entryId": entry_id}),
            );
        }
    }

    /// Remove all queued steering and follow-up messages.
    pub async fn clear_all_queues(&self) {
        self.clear_steering_queue().await;
        self.clear_follow_up_queue().await;
    }

    /// Whether either queue contains a pending message.
    pub async fn has_queued_messages(&self) -> bool {
        !self.inner.deps.steering.is_empty().await || !self.inner.deps.follow_up.is_empty().await
    }

    /// Clear transcript, projections, and queued messages through their owners.
    pub async fn reset(&self) -> Result<(), LoopError> {
        // Pi rejects reset while an active run exists. Hold the admission
        // permit through the reset event so a new prompt cannot race it.
        let _run_permit = self.acquire_run().await?;
        self.inner
            .deps
            .state
            .publish_event(&self.inner.deps.bus, crate::types::AgentEvent::Reset)
            .await;
        self.clear_all_queues().await;
        Ok(())
    }

    /// Apply an externally replayed event through the core state owner.
    ///
    /// This is intentionally an event boundary rather than a state mutator;
    /// YAML replay and adapters can feed control events into the same reducer
    /// without reaching into another actor's fields.
    pub async fn apply_event(&self, event: &AgentEvent) {
        self.inner.deps.state.apply_event(event).await;
    }

    /// Set model configuration through the state actor's mailbox.
    pub async fn set_model(&self, model: crate::types::Model) {
        self.inner
            .deps
            .state
            .publish_event(
                &self.inner.deps.bus,
                crate::types::AgentEvent::ModelChanged { model },
            )
            .await;
    }

    pub async fn set_thinking_level(&self, level: crate::types::ThinkingLevel) {
        self.inner.deps.state.set_thinking_level(level).await;
    }

    /// Ask the provider actor for a Pi-compatible compaction summary. The
    /// provider remains the owner of transport/generation; this coordinator
    /// only forwards the immutable prepared request through its mailbox.
    pub async fn summarize_compaction(
        &self,
        request: crate::session::CompactionSummaryRequest,
    ) -> Result<crate::session::CompactionSummary, LoopError> {
        self.inner
            .deps
            .provider
            .summarize_compaction(request)
            .await
            .map_err(|error| LoopError::Provider(error.to_string()))
    }

    /// Fetch a provider-owned deferred response through the loop boundary.
    ///
    /// Deferred polling is a provider capability, so the loop only forwards
    /// the immutable model, handle, and request options. The returned event
    /// stream remains owned by the caller that admitted the operation; no
    /// loop or renderer state is mutated by this forwarding method.
    pub async fn fetch_deferred(
        &self,
        model: crate::types::Model,
        handle: crate::types::DeferredHandle,
        options: Option<crate::types::SimpleStreamOptions>,
    ) -> Result<tokio::sync::broadcast::Receiver<crate::types::AssistantMessageEvent>, LoopError>
    {
        self.inner
            .deps
            .provider
            .fetch_deferred(model, handle, options)
            .await
            .map_err(|error| LoopError::Provider(error.to_string()))
    }

    /// Cancel a provider-owned deferred response through the loop boundary.
    pub async fn cancel_deferred(
        &self,
        model: crate::types::Model,
        handle: crate::types::DeferredHandle,
        options: Option<crate::types::SimpleStreamOptions>,
    ) -> Result<(), LoopError> {
        self.inner
            .deps
            .provider
            .cancel_deferred(model, handle, options)
            .await
            .map_err(|error| LoopError::Provider(error.to_string()))
    }

    /// Discover models through the provider actor; the loop never performs
    /// provider I/O directly or mutates the catalog actor.
    pub async fn list_models(&self) -> Result<Vec<crate::types::Model>, LoopError> {
        self.inner
            .deps
            .provider
            .list_models()
            .await
            .map_err(|error| LoopError::Provider(error.to_string()))
    }

    /// Replace the owned conversation context through the state actor
    /// mailbox. Session restore and replay adapters use this instead of
    /// reaching into `AgentStateActor` behind the loop boundary.
    pub async fn replace_messages(&self, messages: Vec<AgentMessage>) {
        self.inner.deps.state.replace_messages(messages).await;
    }

    /// Controls how steering messages are drained on subsequent turns.
    pub async fn set_steering_mode(&self, mode: QueueMode) {
        self.reduce_control(LoopControlEvent::SteeringModeChanged(mode))
            .await;
    }

    pub async fn steering_mode(&self) -> QueueMode {
        self.inner.control_rx.borrow().steering_mode
    }

    /// Controls how follow-up messages are drained on subsequent turns.
    pub async fn set_follow_up_mode(&self, mode: QueueMode) {
        self.reduce_control(LoopControlEvent::FollowUpModeChanged(mode))
            .await;
    }

    pub async fn follow_up_mode(&self) -> QueueMode {
        self.inner.control_rx.borrow().follow_up_mode
    }

    pub async fn abort(&self) {
        self.reduce_control(LoopControlEvent::AbortRequested).await;
    }

    async fn reduce_control(&self, event: LoopControlEvent) {
        let _ = mailbox_ack!(self.inner.control_commands, |reply| {
            LoopControlCommand::Reduce(event, reply)
        });
    }

    pub fn control_snapshot(&self) -> LoopControlSnapshot {
        self.inner.control_rx.borrow().clone()
    }

    pub fn shared_control_snapshot(&self) -> crate::SharedSnapshot<LoopControlSnapshot> {
        self.inner.shared_control_rx.borrow().clone()
    }

    pub fn shared_control_subscribe(
        &self,
    ) -> watch::Receiver<crate::SharedSnapshot<LoopControlSnapshot>> {
        self.inner.shared_control_rx.clone()
    }

    pub async fn wait_for_idle(&self) {
        // The admission permit is the single source of truth for whether a
        // run is active. Waiting for a fresh permit avoids a second mutable
        // JoinHandle/Mutex state owner and remains sleep-free.
        if let Ok(permit) = self.inner.running.clone().acquire_owned().await {
            drop(permit);
        }
    }

    pub async fn subscribe(&self, sub: Box<dyn crate::events::Subscriber>) -> crate::events::SubId {
        self.inner.deps.subscribers.register(sub).await
    }

    /// Register a Pi-core-only subscriber. Application/TUI events are
    /// filtered by the registry adapter before delivery.
    pub async fn subscribe_pi(
        &self,
        sub: Box<dyn crate::events::PiSubscriber>,
    ) -> crate::events::SubId {
        self.inner.deps.subscribers.register_pi(sub).await
    }

    pub fn bus(&self) -> EventBus {
        self.inner.deps.bus.clone()
    }

    /// Read-only state projection for UI consumers.
    pub fn state_snapshot(&self) -> crate::state::AgentStateSnapshot {
        self.inner.deps.state.snapshot()
    }
}
