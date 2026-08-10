use super::*;
impl App {
    pub fn new(loop_actor: LoopActor, bus: EventBus) -> Self {
        Self::new_with_broker(
            loop_actor,
            bus,
            runie_core::tools::UserQuestionBroker::default(),
        )
    }

    pub fn new_with_broker(
        loop_actor: LoopActor,
        bus: EventBus,
        question_broker: runie_core::tools::UserQuestionBroker,
    ) -> Self {
        let ui = UiActor::new(&bus);
        let (submission_tx, submission_owner) = submission_actor(loop_actor.clone());
        Self {
            prompt: PromptActor::new(&bus),
            status_actor: StatusActor::new(),
            // EventRenderer is the single live bus-delivery boundary. The
            // actor still owns the feed state; it receives acknowledged
            // reducer messages from the renderer, so no second subscription
            // can reduce the same core event concurrently.
            scrollback_actor: ScrollbackActor::new(),
            session_actor: SessionActor::new_with_bus(&bus),
            session_storage: SessionStorageActor::new(),
            loop_actor,
            bus,
            ui,
            model_catalog: runie_core::model_catalog::ModelCatalogActor::new(),
            provider_registry: runie_core::provider_registry::ProviderRegistryActor::new(
                runie_core::provider_registry::ProviderRegistryState::default(),
            ),
            command_actor: runie_core::command_actor::CommandActor::new(),
            question_broker,
            submission_tx,
            _submission_owner: submission_owner,
        }
    }

    pub fn new_with_welcome(loop_actor: LoopActor, bus: EventBus) -> Self {
        let ui = UiActor::new_with_welcome(&bus, true);
        let (submission_tx, submission_owner) = submission_actor(loop_actor.clone());
        Self {
            prompt: PromptActor::new(&bus),
            status_actor: StatusActor::new(),
            // Keep one event-to-feed path in the interactive app: the
            // renderer delivers core events to this actor's mailbox.
            scrollback_actor: ScrollbackActor::new(),
            session_actor: SessionActor::new_with_bus(&bus),
            session_storage: SessionStorageActor::new(),
            loop_actor,
            bus,
            ui,
            model_catalog: runie_core::model_catalog::ModelCatalogActor::new(),
            provider_registry: runie_core::provider_registry::ProviderRegistryActor::new(
                runie_core::provider_registry::ProviderRegistryState::default(),
            ),
            command_actor: runie_core::command_actor::CommandActor::new(),
            question_broker: runie_core::tools::UserQuestionBroker::default(),
            submission_tx,
            _submission_owner: submission_owner,
        }
    }

    pub async fn toggle_shortcuts(&self) {
        self.ui.send(UiMsg::ToggleShortcuts).await;
    }

    /// Resolve provider discovery asynchronously, then admit only the result
    /// through the catalog actor. TUI rendering never performs provider I/O.
    pub async fn refresh_models(&self) {
        let result = self
            .loop_actor
            .list_models()
            .await
            .map_err(|error| error.to_string());
        self.model_catalog.refresh(result).await;
    }

    /// Route a parsed Pi command through the owning actor boundary shared by
    /// live input and YAML replay. Process termination is deliberately left to
    /// the binary; every stateful command is reduced here through an event or
    /// mailbox and acknowledged before returning.
    #[allow(clippy::too_many_lines)]
    #[allow(
        clippy::cognitive_complexity,
        reason = "the shared command boundary keeps each actor-owned Pi route explicit"
    )]
    pub async fn route_mappable_command(&self, command: MappableBuiltinCommand) -> bool {
        if let Some(result) = self.route_simple_command(&command).await {
            return result;
        }
        if let Some(result) = self.route_selection_command(&command).await {
            return result;
        }
        if let Some(result) = self.route_session_command(&command).await {
            return result;
        }
        if is_storage_command(&command) {
            return self.route_storage_command(command).await;
        }
        match command {
            MappableBuiltinCommand::Changelog
            | MappableBuiltinCommand::NewSession
            | MappableBuiltinCommand::Hotkeys
            | MappableBuiltinCommand::SessionInfo => unreachable!(),
            MappableBuiltinCommand::Copy
            | MappableBuiltinCommand::Model { .. }
            | MappableBuiltinCommand::ScopedModels => unreachable!(),
            MappableBuiltinCommand::Name { .. }
            | MappableBuiltinCommand::Compact { .. }
            | MappableBuiltinCommand::Fork { .. }
            | MappableBuiltinCommand::Tree { .. } => unreachable!(),
            MappableBuiltinCommand::Export { .. }
            | MappableBuiltinCommand::Import { .. }
            | MappableBuiltinCommand::Clone { .. }
            | MappableBuiltinCommand::Resume { .. } => unreachable!(),
            MappableBuiltinCommand::Quit => false,
            MappableBuiltinCommand::Extended { name, args } => {
                self.route_extended_command(&name, &args).await
            }
        }
    }

    /// Route the expanded Grok/Runie vocabulary through the shared event
    /// boundary while feature-specific actors are being introduced. The
    /// invocation is journaled as an application command, so it cannot be
    /// mistaken for ordinary prompt text or silently discarded.
    #[allow(clippy::too_many_lines)]
    async fn route_simple_command(&self, command: &MappableBuiltinCommand) -> Option<bool> {
        match command {
            MappableBuiltinCommand::Changelog => {
                self.ui.send(UiMsg::ToggleChangelog).await;
                Some(true)
            }
            MappableBuiltinCommand::NewSession => {
                let _ = self.reset_session().await;
                Some(true)
            }
            MappableBuiltinCommand::Hotkeys => {
                self.toggle_shortcuts().await;
                Some(true)
            }
            MappableBuiltinCommand::SessionInfo => {
                self.ui.send(UiMsg::ToggleSessionInfo).await;
                Some(true)
            }
            _ => None,
        }
    }

    async fn route_selection_command(&self, command: &MappableBuiltinCommand) -> Option<bool> {
        match command {
            MappableBuiltinCommand::Copy => {
                let text = runie_tui_model::last_assistant_text(
                    &self.scrollback_actor.model_snapshot().lines,
                );
                self.ui.send(UiMsg::CopyText(text)).await;
                Some(true)
            }
            MappableBuiltinCommand::Model { reference } => {
                let Some((provider, model)) = reference.split_once('/') else {
                    return Some(false);
                };
                let selected = Model {
                    id: model.to_owned(),
                    name: model.to_owned(),
                    provider: provider.to_owned(),
                    ..Model::default()
                };
                let Some(selected) = self.model_catalog.select(selected).await else {
                    return Some(false);
                };
                self.loop_actor.set_model(selected).await;
                Some(true)
            }
            MappableBuiltinCommand::ScopedModels => {
                self.toggle_model_selector().await;
                self.model_selector_key(UiMsg::ModelSelectorToggleScope)
                    .await;
                Some(true)
            }
            _ => None,
        }
    }

    async fn route_session_command(&self, command: &MappableBuiltinCommand) -> Option<bool> {
        match command {
            MappableBuiltinCommand::Name { name } => {
                self.bus
                    .publish(AgentEvent::SessionNameChanged { name: name.clone() });
                self.session_actor.flush().await;
                Some(true)
            }
            MappableBuiltinCommand::Compact { instructions } => {
                let _ = self.compact_session(None, instructions.clone()).await;
                Some(true)
            }
            MappableBuiltinCommand::Fork { target_id } => {
                Some(self.route_session_result(
                    self.session_actor.fork_at_message(target_id.clone()).await,
                ))
            }
            MappableBuiltinCommand::Tree { target_id } => Some(
                self.route_session_result(self.session_actor.select_tree(target_id.clone()).await),
            ),
            _ => None,
        }
    }

    fn route_session_result(&self, result: Result<(), String>) -> bool {
        if let Err(error) = result {
            self.bus.publish(AgentEvent::Error { message: error });
        }
        true
    }

    async fn route_storage_command(&self, command: MappableBuiltinCommand) -> bool {
        let result = match command {
            MappableBuiltinCommand::Export { path } => {
                self.session_storage
                    .publish_snapshot(path, &self.session_actor.snapshot(), "runie-session", 0, "")
                    .await
            }
            MappableBuiltinCommand::Import { path } | MappableBuiltinCommand::Resume { path } => {
                let (_, _, snapshot) = match self.session_storage.load_snapshot(path).await {
                    Ok(value) => value,
                    Err(error) => {
                        self.bus.publish(AgentEvent::Error { message: error });
                        return true;
                    }
                };
                self.session_actor.restore_snapshot(snapshot).await
            }
            MappableBuiltinCommand::Clone { path } => {
                let snapshot = self.session_actor.snapshot();
                // Clone duplicates the current session position. The
                // snapshot; publishing that immutable snapshot avoids
                // re-entering branch traversal while preserving the exact
                // current session state, including an intentionally empty
                // session.
                self.session_storage
                    .publish_snapshot(path, &snapshot, "runie-clone", 0, "")
                    .await
            }
            _ => unreachable!(),
        };
        if let Err(error) = result {
            self.bus.publish(AgentEvent::Error { message: error });
        }
        true
    }

    /// Route the complete Pi built-in classification through the shared
    /// application boundary. Unsupported capabilities become an explicit
    /// actor-delivered error; ordinary prompt text remains outside this path.
    pub async fn route_builtin_command(&self, disposition: BuiltinCommandDisposition) -> bool {
        match disposition {
            BuiltinCommandDisposition::Mappable(command) => {
                self.route_mappable_command(command).await
            }
            BuiltinCommandDisposition::Unsupported(command) => {
                self.bus.publish(AgentEvent::Error {
                    message: format!("Pi command /{} is not supported by Runie", command.name()),
                });
                true
            }
            BuiltinCommandDisposition::NotBuiltin => false,
        }
    }

    /// Run Pi's manual compaction pipeline through its existing actor seams.
    /// Preparation and journal publication belong to `SessionActor`; summary
    /// generation belongs to `ProviderActor`; the bus transfers the resulting
    /// fact to every interested projection.
    #[allow(
        clippy::too_many_lines,
        reason = "compaction keeps preparation, provider settlement, and journal publication explicit"
    )]
    pub async fn compact_session(
        &self,
        previous_summary: Option<String>,
        custom_instructions: Option<String>,
    ) -> Result<(), String> {
        const GROK_COMPACTION_KEEP_RECENT_TOKENS: u64 = 20_000;
        let snapshot = self.session_actor.snapshot();
        let token_estimates = compaction_token_estimates(&snapshot);
        let Some((compaction_id, preparation, entries)) = self
            .session_actor
            .prepare_and_begin_compaction(
                token_estimates,
                GROK_COMPACTION_KEEP_RECENT_TOKENS,
                "main".into(),
            )
            .await?
        else {
            return Ok(());
        };
        let request = runie_core::session::CompactionSummaryRequest::from_preparation(
            &preparation,
            &entries,
            previous_summary,
        )?
        .with_custom_instructions(custom_instructions);
        let summary = match self.loop_actor.summarize_compaction(request).await {
            Ok(summary) => summary,
            Err(error) => {
                self.record_compaction_failure(&compaction_id, &error).await;
                return Err(error.to_string());
            }
        };
        self.publish_compaction(snapshot, entries, preparation, summary)
            .await;
        self.record_compaction_success(&compaction_id).await?;
        Ok(())
    }

    async fn publish_compaction(
        &self,
        snapshot: runie_core::session::SessionSnapshot,
        entries: Vec<runie_core::session::SessionEntry>,
        preparation: runie_core::session::CompactionPreparation,
        summary: runie_core::session::CompactionSummary,
    ) {
        let retained_tail = compaction_retained_tail(
            &runie_core::session::SessionSnapshot {
                entries,
                ..snapshot
            },
            &preparation,
        );
        self.bus.publish(AgentEvent::CompactionCreated {
            summary: summary.summary,
            retained_tail,
            tokens_before: preparation.tokens_before,
            details: summary.details,
            usage: summary.usage,
        });
        self.session_actor.flush().await;
    }

    async fn record_compaction_failure(&self, id: &str, error: &runie_core::r#loop::LoopError) {
        let _ = self
            .session_actor
            .record_operation(
                runie_core::session::SessionOperationKind::Finished,
                serde_json::json!({
                    "id": id,
                    "outcome": "failed",
                    "error": {"code": "compaction", "message": error.to_string()},
                }),
            )
            .await;
    }

    async fn record_compaction_success(&self, id: &str) -> Result<(), String> {
        self.session_actor
            .record_operation(
                runie_core::session::SessionOperationKind::Finished,
                serde_json::json!({"id": id, "outcome": "completed"}),
            )
            .await
    }
}

include!("app_extended_command.inc");

fn is_storage_command(command: &MappableBuiltinCommand) -> bool {
    matches!(
        command,
        MappableBuiltinCommand::Export { .. }
            | MappableBuiltinCommand::Import { .. }
            | MappableBuiltinCommand::Clone { .. }
            | MappableBuiltinCommand::Resume { .. }
    )
}
