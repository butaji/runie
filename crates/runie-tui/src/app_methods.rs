use super::*;
impl App {
    pub async fn load_provider_config(&self) {
        if let Ok(state) =
            runie_core::provider_registry::load_provider_config(provider_config_path()).await
        {
            self.provider_registry.replace(state).await;
        }
    }
    pub async fn apply_provider_event(
        &self,
        event: runie_core::provider_registry::ProviderEvent,
    ) -> Result<(), String> {
        self.provider_registry.apply(event).await;
        let path = provider_config_path();
        runie_core::provider_registry::save_provider_config(
            path,
            &self.provider_registry.snapshot(),
        )
        .await
    }

    pub async fn show_provider_summary(&self) {
        let shared = self.provider_registry.shared_snapshot();
        let state = shared.get();
        let summary = if state.providers.is_empty() {
            "providers: none configured".to_owned()
        } else {
            state
                .providers
                .iter()
                .map(|provider| {
                    format!(
                        "{} [{}]{}",
                        provider.label,
                        if provider.connected {
                            "connected"
                        } else {
                            "disconnected"
                        },
                        provider
                            .selected_model
                            .as_deref()
                            .map(|model| format!(" · {model}"))
                            .unwrap_or_default()
                    )
                })
                .collect::<Vec<_>>()
                .join(" | ")
        };
        self.prompt.set_model_caption(summary).await;
    }

    pub async fn reset_session(&self) -> Result<(), runie_core::r#loop::LoopError> {
        self.loop_actor.reset().await
    }

    /// Deliver one typed theme event to every owning projection and await the
    /// three mailbox acknowledgements before returning. The coordinator is
    /// the single delivery boundary for this application command; each actor
    /// still owns and reduces only its own state.
    pub async fn set_theme(&self, theme: runie_core::types::ThemeKind) {
        let event = AgentEvent::ThemeChanged { theme };
        tokio::join!(
            self.prompt.apply_event(event.clone()),
            self.status_actor.apply_event(&event),
            self.scrollback_actor.apply_event(&event),
        );
    }

    pub async fn toggle_command_palette(&self) {
        self.ui.send(UiMsg::ToggleCommandPalette).await;
    }

    pub async fn model_selector_key(&self, msg: UiMsg) {
        self.ui.send(msg).await;
        let ui = self.ui.snapshot();
        if ui.model_selector_open {
            self.model_catalog
                .search(
                    ui.model_selector_query.clone(),
                    ui.model_selector_scoped_only,
                )
                .await;
            self.ui
                .send(UiMsg::SetModelSelectorRows(model_selector_rows(
                    self.model_catalog.shared_snapshot().get(),
                )))
                .await;
        }
    }

    /// Commit the selected catalog row through both owning actors. The UI
    /// actor only closes its overlay; model selection remains catalog-owned
    /// and the loop actor admits the resulting model through its mailbox.
    pub async fn activate_model_selector(&self) -> Option<Model> {
        let ui = self.ui.snapshot();
        let catalog = self.model_catalog.shared_snapshot();
        let model = catalog
            .get()
            .results
            .get(ui.model_selector_index)
            .cloned()?;
        let selected = self.model_catalog.select(model).await?;
        self.set_model_with_declared_effort(selected.clone()).await;
        self.ui.send(UiMsg::ActivateModelSelector).await;
        if Self::model_has_declared_effort(&selected) {
            self.open_effort_picker(&selected).await;
        }
        Some(selected)
    }

    pub async fn command_palette_key(&self, msg: UiMsg) {
        self.ui.send(msg).await;
    }

    pub async fn activate_command_palette(&self) -> Option<String> {
        self.ui.send(UiMsg::ActivateCommandPalette).await;
        self.ui.snapshot().last_palette_command
    }

    pub fn subscribe_ui_commands(&self) -> broadcast::Receiver<UiCommand> {
        self.ui.subscribe_commands()
    }

    pub async fn hide_welcome(&self) {
        self.ui.send(UiMsg::HideWelcome).await;
    }

    pub async fn toggle_activity_fold(&self) {
        self.scrollback_actor
            .apply(crate::widgets::ScrollbackMsg::ToggleActivityExpanded)
            .await;
    }

    /// Grok's `e` fold intent targets the active scrollback entry. Until the
    /// full cursor/navigation model lands, the actor's last tool block is the
    /// deterministic selected-entry fallback; an empty feed keeps the legacy
    /// activity-group fold behavior.
    pub async fn toggle_selected_tool_fold(&self) {
        let snapshot = self.scrollback_actor.shared_snapshot();
        let tool_call_id = snapshot.get().selected_tool_id.clone().or_else(|| {
            snapshot
                .get()
                .tool_blocks
                .last()
                .map(|block| block.tool_call_id.clone())
        });
        if let Some(tool_call_id) = tool_call_id {
            self.scrollback_actor
                .apply(crate::widgets::ScrollbackMsg::ToggleToolMode(tool_call_id))
                .await;
        } else {
            self.toggle_activity_fold().await;
        }
    }

    pub async fn select_next_tool(&self) {
        self.scrollback_actor
            .apply(crate::widgets::ScrollbackMsg::SelectNextTool)
            .await;
    }

    pub async fn select_previous_tool(&self) {
        self.scrollback_actor
            .apply(crate::widgets::ScrollbackMsg::SelectPreviousTool)
            .await;
    }

    pub async fn select_next_entry(&self) {
        self.scrollback_actor
            .apply(crate::widgets::ScrollbackMsg::SelectNextEntry)
            .await;
    }

    pub async fn select_previous_entry(&self) {
        self.scrollback_actor
            .apply(crate::widgets::ScrollbackMsg::SelectPreviousEntry)
            .await;
    }

    pub async fn extend_selection(&self, delta: i32) {
        let snapshot = self.scrollback_actor.shared_snapshot();
        let Some(current) = snapshot.get().selected_entry else {
            return;
        };
        let next = if delta < 0 {
            current.saturating_sub(delta.unsigned_abs() as usize)
        } else {
            current
                .saturating_add(delta as usize)
                .min(snapshot.get().lines.len().saturating_sub(1))
        };
        let anchor = snapshot.get().selection_anchor.unwrap_or(current);
        self.scrollback_actor
            .apply(crate::widgets::ScrollbackMsg::SelectRange { anchor, head: next })
            .await;
    }

    pub async fn scroll_scrollback_by(&self, lines: i32) {
        self.scrollback_actor
            .apply(crate::widgets::ScrollbackMsg::ScrollBy(lines))
            .await;
    }

    pub async fn mouse_selection_start(&self, position: crate::widgets::CellPosition) {
        self.scrollback_actor
            .apply(crate::widgets::ScrollbackMsg::MouseSelectionStart(position))
            .await;
    }

    pub async fn mouse_selection_extend(&self, position: crate::widgets::CellPosition) {
        self.scrollback_actor
            .apply(crate::widgets::ScrollbackMsg::MouseSelectionExtend(
                position,
            ))
            .await;
    }

    pub async fn mouse_selection_commit(&self) {
        self.scrollback_actor
            .apply(crate::widgets::ScrollbackMsg::MouseSelectionCommit)
            .await;
    }

    pub async fn request_copy_selection(&self) {
        self.scrollback_actor
            .apply(crate::widgets::ScrollbackMsg::RequestCopySelection)
            .await;
    }

    /// Acknowledge the actor-owned selection request and return its immutable
    /// text payload. The terminal clipboard effect remains in the binary.
    pub async fn copy_selection_text(&self) -> Option<String> {
        self.request_copy_selection().await;
        let snapshot = self.scrollback_actor.shared_snapshot();
        snapshot
            .get()
            .copy_selection
            .map(|selection| runie_tui_model::selected_cell_text(&snapshot.get().lines, selection))
    }

    pub async fn clear_copy_request(&self) {
        self.scrollback_actor
            .apply(crate::widgets::ScrollbackMsg::ClearCopyRequest)
            .await;
    }

    /// Apply a feed update through the actor that owns the rendered snapshot.
    /// The mutex is a compatibility fallback for apps whose renderer is not
    /// running yet.
    pub async fn apply_scrollback(&self, message: crate::widgets::ScrollbackMsg) {
        self.scrollback_actor.apply(message).await;
    }

    pub async fn apply_scrollback_batch(&self, messages: Vec<crate::widgets::ScrollbackMsg>) {
        self.scrollback_actor.apply_batch(messages).await;
    }

    /// Handle a prompt outcome. Returns Some(text) on submit.
    pub async fn handle_prompt_outcome(&self, outcome: PromptOutcome) -> Option<String> {
        match outcome {
            PromptOutcome::Submitted(text) => {
                let timestamp = crate::clock::unix_timestamp_seconds();
                let user_msg = AgentMessage::User(runie_core::types::UserMessage {
                    content: vec![runie_core::types::UserContent::Text { text: text.clone() }],
                    timestamp,
                });
                let (accepted, acknowledged) = tokio::sync::oneshot::channel();
                if self
                    .submission_tx
                    .send((vec![user_msg], accepted))
                    .await
                    .is_err()
                {
                    return None;
                }
                let _ = acknowledged.await;
                Some(text)
            }
            PromptOutcome::Edited | PromptOutcome::Ignored => None,
        }
    }

    /// Spawn the renderer task. Owns the spawned task via JoinHandle.
    pub fn spawn_renderer(
        &self,
    ) -> (
        tokio::task::JoinHandle<()>,
        tokio::sync::watch::Sender<bool>,
    ) {
        let workspace = std::env::current_dir()
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_default();
        let renderer = EventRenderer::with_live_actors(
            self.scrollback_actor.clone(),
            self.status_actor.clone(),
            workspace,
        );
        let rx = self.bus.subscribe();
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        // OWNER: App — drives the renderer to completion.
        let handle = tokio::spawn(async move { renderer.run(rx, shutdown_rx).await });
        (handle, shutdown_tx)
    }

    pub fn status_snapshot(&self) -> StatusBar {
        self.status_actor.snapshot()
    }

    pub fn status_model_snapshot(&self) -> StatusSnapshot {
        self.status_actor.model_snapshot()
    }

    pub fn model_snapshot(&self) -> TuiSnapshot {
        TuiSnapshot {
            ui: self.ui.snapshot(),
            feed: self.feed_model_snapshot(),
            prompt: self.prompt.model_snapshot(),
            status: self.status_model_snapshot(),
        }
    }

    pub fn scrollback_snapshot(&self) -> Scrollback {
        self.scrollback_actor.snapshot()
    }

    pub async fn flush_session(&self) {
        self.session_actor.flush().await;
    }

    pub fn session_snapshot(&self) -> SessionSnapshot {
        self.session_actor.snapshot()
    }

    /// Read the renderer-independent feed model. New projections and
    /// scenario assertions should prefer this API over the compatibility
    /// widget snapshot.
    pub fn feed_model_snapshot(&self) -> FeedSnapshot {
        self.scrollback_actor.model_snapshot()
    }

    /// Build the immutable declarative view description from actor snapshots.
    /// Renderers consume this projection; they do not inspect ownership state
    /// through ad-hoc mutable fields.
    pub fn view_tree(&self) -> Element {
        self.view_document().root
    }

    /// Build the complete renderer-neutral document for one frame. The
    /// document retains both composition (`root`) and component ownership
    /// metadata; callers that only need the legacy element tree can use
    /// `view_tree`.
    pub fn view_document(&self) -> ViewDocument {
        Self::view_document_from_model(&self.model_snapshot())
    }

    pub fn view_tree_from_model(model: &TuiSnapshot) -> Element {
        Self::view_document_from_model(model).root
    }

    pub fn view_document_from_model(model: &TuiSnapshot) -> ViewDocument {
        chat_document_with_props(ViewProps {
            chat: ChatViewProps {
                welcome_visible: model.ui.show_welcome,
                shortcuts_visible: dialog_is_visible(&model.ui, "shortcuts"),
                command_palette_visible: dialog_is_visible(&model.ui, "commands"),
                model_selector_visible: dialog_is_visible(&model.ui, "model"),
                // The settled small-screen hint is ambient: after the first
                // completed turn it remains below the feed, matching Grok's
                // one-shot tip promotion. Terminal-size gating belongs to
                // the renderer because this projection is size-independent.
                compact_mode_hint_visible: matches!(model.status.state, Status::Ready)
                    && !model.feed.is_empty(),
            },
            header: HeaderViewProps {
                meter: model.status.header_meter(),
                theme: model.status.theme,
            },
            feed: model.feed.clone(),
            prompt: model.prompt.clone(),
            status: model.status.clone(),
            ui: model.ui.clone(),
        })
    }

    pub fn header_view_props(&self) -> HeaderViewProps {
        let status = self.status_snapshot();
        HeaderViewProps {
            meter: status.header_meter(),
            theme: status.theme(),
        }
    }

    /// Lay out the widgets and render them into the given area using `f`.
    pub fn render<F: FnMut(Rect, &mut Buffer)>(&self, area: Rect, mut f: F) {
        let model = self.model_snapshot();
        let layout = chat_layout_with_prompt_height(area, model.prompt.render_height());
        let sb = Scrollback::from_model_snapshot(model.feed);
        let content_rows = sb.measured_content_rows(layout.scrollback, area.height);
        let anchor_row = sb.measured_anchor_row(layout.scrollback, area.height);
        let _ = self
            .scrollback_actor
            .try_apply(crate::widgets::ScrollbackMsg::LayoutMeasured {
                content_rows,
                viewport_rows: layout.scrollback.height as usize,
                anchor_row,
            });
        let mut buf = Buffer::empty(area);
        sb.render_with_terminal_height(layout.scrollback, area.height, &mut buf);
        f(layout.prompt, &mut buf);
        f(layout.status, &mut buf);
    }
}

fn provider_config_path() -> std::path::PathBuf {
    if let Some(directory) = std::env::var_os("RUNIE_CONFIG_DIR") {
        return std::path::PathBuf::from(directory).join("providers.json");
    }
    let directory = std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| std::path::PathBuf::from(home).join(".config"))
        })
        .unwrap_or_else(|| std::path::PathBuf::from(".config"));
    directory.join("runie/providers.json")
}
