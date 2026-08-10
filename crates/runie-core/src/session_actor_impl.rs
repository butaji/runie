impl SessionActor {
    #[allow(
        clippy::too_many_lines,
        reason = "the actor constructor keeps its complete mailbox reduction loop visible"
    )]
    pub fn new() -> Self {
        let (snapshot_tx, snapshot) = watch::channel(SessionSnapshot::default());
        let (tx, owner) = spawn_actor_worker!(32, |rx: mpsc::Receiver<Command>| {
            session_worker!(snapshot_tx.clone(), rx)
        });
        Self {
            tx,
            snapshot,
            _owner: owner,
            _bus_owner: None,
        }
    }

    #[allow(
        clippy::too_many_lines,
        reason = "the session bus bridge keeps event-to-record ownership explicit"
    )]
    pub fn new_with_bus(bus: &EventBus) -> Self {
        let mut actor = Self::new();
        let events = bus.subscribe();
        let rejection_bus = bus.clone();
        let tx = actor.tx.clone();
        actor._bus_owner = Some(spawn_owned_worker!(async move {
            Self::run_session_bus_bridge(events, tx, rejection_bus).await;
        }));
        actor
    }

    async fn run_session_bus_bridge(
        mut events: SessionEventReceiver,
        tx: SessionMailbox,
        rejection_bus: EventBus,
    ) {
        let mut tool_termination = HashMap::<String, bool>::new();
        while let Ok(event) = events.recv().await {
            let event = match Self::handle_tool_bus_event(&tx, &mut tool_termination, event).await {
                Ok(Some(event)) => event,
                Ok(None) => continue,
                Err(()) => break,
            };
            if !Self::reduce_session_bus_event(&tx, &rejection_bus, event).await {
                break;
            }
        }
    }

    async fn reduce_session_bus_event(
        tx: &SessionMailbox,
        rejection_bus: &EventBus,
        event: AgentEvent,
    ) -> bool {
        match event {
            AgentEvent::Reset => Self::reset_session_bus(tx).await,
            AgentEvent::SessionEntryAppended { lane, message } => {
                Self::append_session_entry(tx, lane, message).await
            }
            event => match session_config_record!(&event) {
                Some(record) => Self::forward_session_config(tx, rejection_bus, record).await,
                None => true,
            },
        }
    }

    async fn handle_tool_bus_event(
        tx: &SessionMailbox,
        terminations: &mut HashMap<String, bool>,
        event: AgentEvent,
    ) -> Result<Option<AgentEvent>, ()> {
        match event {
            AgentEvent::MessageEnd { message } => {
                if Self::append_bus_message(tx, terminations, message).await {
                    Ok(None)
                } else {
                    Err(())
                }
            }
            AgentEvent::ToolExecutionEnd {
                tool_call_id,
                result,
                ..
            } => {
                Self::remember_tool_termination(terminations, tool_call_id, result);
                Ok(None)
            }
            AgentEvent::ToolExecutionStart {
                tool_call_id,
                tool_name,
                args,
            } => {
                if Self::start_bus_tool(tx, tool_call_id, tool_name, args).await {
                    Ok(None)
                } else {
                    Err(())
                }
            }
            event => Ok(Some(event)),
        }
    }

    async fn append_session_entry(
        tx: &mpsc::Sender<Command>,
        lane: String,
        message: AgentMessage,
    ) -> bool {
        let (reply_tx, reply_rx) = oneshot::channel();
        tx.send(Command::Append(lane, Box::new(message), false, reply_tx))
            .await
            .is_ok()
            && reply_rx.await.is_ok()
    }

    async fn reset_session_bus(tx: &mpsc::Sender<Command>) -> bool {
        mailbox_ack!(tx, Command::Reset)
    }

    async fn append_bus_message(
        tx: &mpsc::Sender<Command>,
        terminations: &mut HashMap<String, bool>,
        message: AgentMessage,
    ) -> bool {
        let terminate = match &message {
            AgentMessage::ToolResult(result) => {
                terminations.remove(&result.tool_call_id).unwrap_or(false)
            }
            _ => false,
        };
        mailbox_ack!(tx, |reply| Command::Append(
            "main".into(),
            Box::new(message),
            terminate,
            reply
        ))
    }

    fn remember_tool_termination(
        terminations: &mut HashMap<String, bool>,
        tool_call_id: String,
        result: serde_json::Value,
    ) {
        let terminate = result
            .get("terminate")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        terminations.insert(tool_call_id, terminate);
    }

    async fn start_bus_tool(
        tx: &mpsc::Sender<Command>,
        tool_call_id: String,
        tool_name: String,
        args: serde_json::Value,
    ) -> bool {
        mailbox_ack!(tx, |reply| Command::ToolStarted {
            tool_call_id,
            tool_name,
            args,
            reply
        })
    }

    async fn forward_session_config(
        tx: &mpsc::Sender<Command>,
        rejection_bus: &EventBus,
        record: SessionConfigRecord,
    ) -> bool {
        let (reply_tx, reply_rx) = oneshot::channel();
        if tx.send(Command::Config(record, reply_tx)).await.is_err() {
            return false;
        }
        match reply_rx.await {
            Ok(Ok(())) => true,
            Ok(Err(error)) => {
                rejection_bus.publish(AgentEvent::Error {
                    message: format!("Session event rejected: {error}"),
                });
                true
            }
            Err(_) => false,
        }
    }

    pub async fn append(&self, message: AgentMessage) {
        let _ = mailbox_ack!(self.tx, |reply| {
            Command::Append("main".into(), Box::new(message), false, reply)
        });
    }

    /// Append through a named Pi session lane. Invalid lanes are rejected at
    /// the actor boundary without publishing a partial entry.
    pub async fn append_to_lane(&self, lane: String, message: AgentMessage) -> Result<(), String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(Command::Append(lane, Box::new(message), false, reply_tx))
            .await
            .map_err(|_| "session actor is closed".to_owned())?;
        reply_rx
            .await
            .map_err(|_| "session actor response was dropped".to_owned())?
    }

    /// Append a Pi custom journal entry through the session owner. Custom
    /// entries are opaque extension data: the actor journals and persists the
    /// payload but never interprets it as an agent message.
    pub async fn append_custom_entry(
        &self,
        custom_type: String,
        data: Option<serde_json::Value>,
    ) -> Result<(), String> {
        if custom_type.trim().is_empty() {
            return Err("custom session entry type cannot be empty".to_owned());
        }
        self.record_config(SessionConfigRecord::CustomSessionEntryCreated { custom_type, data })
            .await
    }

    /// Apply a session configuration fact through the owning mailbox.
    pub async fn record_config(&self, record: SessionConfigRecord) -> Result<(), String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(Command::Config(record, reply_tx))
            .await
            .map_err(|_| "session actor is closed".to_owned())?;
        reply_rx
            .await
            .map_err(|_| "session actor response was dropped".to_owned())?
    }

    /// Record a typed operation family through the session actor while keeping
    /// Pi's lossless JSON payload at the persistence boundary.
    pub async fn record_operation(
        &self,
        kind: SessionOperationKind,
        data: serde_json::Value,
    ) -> Result<(), String> {
        let record = SessionLaneRecord::decode(kind.wire_name(), &data)
            .map_err(|error| format!("invalid session operation: {error}"))?;
        self.record_typed_operation(record).await
    }

    /// Record a validated operation-lane union through the session actor.
    /// Callers that already possess the typed fact do not need to reconstruct
    /// the generic `(record_type, data)` compatibility edge.
    pub async fn record_typed_operation(&self, record: SessionLaneRecord) -> Result<(), String> {
        self.record_config(SessionConfigRecord::TypedOperation(record))
            .await
    }

    /// Allocate and admit a compaction operation identity in one actor turn.
    pub async fn begin_compaction(&self, lane: String) -> Result<String, String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(Command::BeginCompaction {
                lane,
                reply: reply_tx,
            })
            .await
            .map_err(|_| "session actor is closed".to_owned())?;
        reply_rx
            .await
            .map_err(|_| "session actor response was dropped".to_owned())?
    }

    /// Admit a Pi navigation operation only when its target and optional
    /// summary entry belong to this journal. Replay may retain unresolved
    /// historical intents, but live navigation cannot publish one that would
    /// point outside the actor-owned session tree.
    pub async fn admit_navigation(
        &self,
        operation_id: String,
        lane: String,
        target_id: String,
        summarize: bool,
        summary_entry_id: Option<String>,
    ) -> Result<(), String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(Command::AdmitNavigation {
                operation_id,
                lane,
                target_id,
                summarize,
                summary_entry_id,
                reply: reply_tx,
            })
            .await
            .map_err(|_| "session actor is closed".to_owned())?;
        reply_rx
            .await
            .map_err(|_| "session actor response was dropped".to_owned())?
    }

    pub async fn record_lane(
        &self,
        lane: String,
        leaf_id: Option<String>,
        create: bool,
    ) -> Result<(), String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(Command::Lane {
                lane,
                leaf_id,
                create,
                reply: reply_tx,
            })
            .await
            .map_err(|_| "session actor is closed".to_owned())?;
        reply_rx
            .await
            .map_err(|_| "session actor response was dropped".to_owned())?
    }

    /// Fork the selected message branch through the session owner's mailbox.
    /// The caller supplies only a target identity; branch validation and state
    /// replacement remain inside the actor.
    pub async fn fork_at_message(&self, target_id: String) -> Result<(), String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(Command::Fork {
                target_id,
                reply: reply_tx,
            })
            .await
            .map_err(|_| "session actor is closed".to_owned())?;
        reply_rx
            .await
            .map_err(|_| "session actor response was dropped".to_owned())?
    }

    /// Select an existing journal node without deleting alternate branches.
    /// Tree navigation is an actor-owned leaf change, distinct from forking.
    pub async fn select_tree(&self, target_id: String) -> Result<(), String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(Command::SelectTree {
                target_id,
                reply: reply_tx,
            })
            .await
            .map_err(|_| "session actor is closed".to_owned())?;
        reply_rx
            .await
            .map_err(|_| "session actor response was dropped".to_owned())?
    }

    /// Apply session-owned configuration facts from a replay event sequence.
    /// The reducer remains the actor boundary; callers do not mutate the
    /// snapshot or manufacture message entries.
    #[allow(
        clippy::too_many_lines,
        clippy::cognitive_complexity,
        reason = "session event dispatch keeps each journal variant explicit"
    )]
    pub async fn apply_event(&self, event: &AgentEvent) -> Result<(), String> {
        if let AgentEvent::CustomSessionEntryCreated { custom_type, data } = event {
            self.append_custom_entry(custom_type.clone(), data.clone())
                .await
        } else if let Some(record) = session_config_record!(event) {
            self.record_config(record).await
        } else if let AgentEvent::SessionLaneChanged {
            lane,
            leaf_id,
            create,
        } = event
        {
            self.record_lane(lane.clone(), leaf_id.clone(), *create)
                .await
        } else if let AgentEvent::SessionEntryAppended { lane, message } = event {
            self.append_to_lane(lane.clone(), message.clone()).await
        } else if matches!(event, AgentEvent::Reset) {
            self.reset().await;
            Ok(())
        } else {
            Ok(())
        }
    }

    pub async fn reset(&self) {
        let _ = mailbox_ack!(self.tx, Command::Reset);
    }

    /// Restore a validated Pi JSONL message lane through the actor mailbox.
    /// Parsing is pure; replacing the owned journal and publishing its
    /// snapshot are performed only by the actor worker.
    pub async fn restore_jsonl(&self, input: &str) -> Result<(String, String), String> {
        let repaired = SessionSnapshot::repair_jsonl_torn_tail(input)?;
        let (session_id, cwd, snapshot) = SessionSnapshot::from_jsonl(&repaired)?;
        if !mailbox_ack!(self.tx, |reply| Command::Import(snapshot, reply)) {
            return Err("session actor restore was not acknowledged".to_owned());
        }
        Ok((session_id, cwd))
    }

    /// Restore a validated snapshot through the same mailbox used by JSONL
    /// import. Storage actors own parsing; this method owns publication into
    /// the session actor without exposing mutable state to callers.
    pub async fn restore_snapshot(&self, snapshot: SessionSnapshot) -> Result<(), String> {
        if !mailbox_ack!(self.tx, |reply| Command::Import(snapshot, reply)) {
            return Err("session actor restore was not acknowledged".into());
        }
        Ok(())
    }

    pub fn snapshot(&self) -> SessionSnapshot {
        self.snapshot.borrow().clone()
    }

    pub async fn flush(&self) {
        let _ = mailbox_ack!(self.tx, Command::Flush);
    }

    /// Ask the session owner to prepare compaction from its current state.
    /// Callers provide deterministic token estimates; no snapshot mutation or
    /// summarization occurs in this command.
    pub async fn prepare_compaction(
        &self,
        token_estimates: Vec<u64>,
        keep_recent_tokens: u64,
    ) -> Result<Option<CompactionPreparation>, String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        if self
            .tx
            .send(Command::PrepareCompaction {
                token_estimates,
                keep_recent_tokens,
                reply: reply_tx,
            })
            .await
            .is_err()
        {
            return Err("session actor compaction request was not acknowledged".into());
        }
        reply_rx
            .await
            .map_err(|_| "session actor compaction response was dropped".to_owned())?
    }

    /// Prepare a compaction and admit its operation start against one actor
    /// snapshot. The returned entries are the exact source journal used by
    /// the preparation, so provider summarization cannot use a different
    /// message set.
    pub async fn prepare_and_begin_compaction(
        &self,
        token_estimates: Vec<u64>,
        keep_recent_tokens: u64,
        lane: String,
    ) -> Result<Option<PreparedCompaction>, String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(Command::PrepareAndBeginCompaction {
                token_estimates,
                keep_recent_tokens,
                lane,
                reply: reply_tx,
            })
            .await
            .map_err(|_| "session actor compaction request was not acknowledged".to_owned())?;
        reply_rx
            .await
            .map_err(|_| "session actor compaction response was dropped".to_owned())?
    }

    /// Publish a provider-generated compaction through the session owner.
    /// The actor revalidates preparation indices against its current journal,
    /// so a stale caller cannot append a summary for a different session
    /// state.
    pub async fn publish_compaction(
        &self,
        preparation: CompactionPreparation,
        summary: CompactionSummary,
    ) -> Result<(), String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(Command::PublishCompaction {
                preparation,
                summary,
                reply: reply_tx,
            })
            .await
            .map_err(|_| "session actor compaction publication was not acknowledged".to_owned())?;
        reply_rx
            .await
            .map_err(|_| "session actor compaction publication response was dropped".to_owned())?
    }
}

