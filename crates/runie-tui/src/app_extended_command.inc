impl App {
    async fn route_extended_command(&self, name: &str, args: &str) -> bool {
        if name == "sessions" {
            let root = std::env::current_dir()
                .map(|path| path.to_string_lossy().into_owned())
                .unwrap_or_else(|_| ".".into());
            let result = match self
                .session_storage
                .discover_rows(root, 64, args.trim())
                .await
            {
                Ok(rows) if rows.is_empty() => "No stored sessions found".into(),
                Ok(rows) => rows
                    .into_iter()
                    .map(|row| format!("{} · {} · {}", row.session_id, row.label, row.cwd))
                    .collect::<Vec<_>>()
                    .join("\n"),
                Err(error) => format!("Session discovery failed: {error}"),
            };
            self.ui.send(UiMsg::ShowCommandResult(result)).await;
            return true;
        }
        if name == "resume" && args.trim().is_empty() {
            let root = std::env::current_dir()
                .map(|path| path.to_string_lossy().into_owned())
                .unwrap_or_else(|_| ".".into());
            let options = match self.session_storage.discover(root, 64).await {
                Ok(entries) => entries.into_iter().map(|entry| entry.path).collect(),
                Err(_) => Vec::new(),
            };
            self.ui
                .send(UiMsg::SetPaletteParameterOptions(options))
                .await;
            self.ui
                .send(UiMsg::OpenPaletteParameters(PaletteAction::ResumeSession))
                .await;
            return true;
        }
        if name == "undo" {
            let result = self.session_actor.undo().await;
            if let Err(error) = &result {
                self.bus.publish(AgentEvent::Error {
                    message: error.clone(),
                });
            }
            self.ui
                .send(UiMsg::ShowCommandResult(match result {
                    Ok(()) => "Session undone".into(),
                    Err(error) => format!("Undo unavailable: {error}"),
                }))
                .await;
            return true;
        }
        if name == "jobs" {
            if let Some(id) = runie_core::commands::parse_background_job_command(args) {
                let cancelled = self.background_actor.cancel(id).await;
                self.ui
                    .send(UiMsg::ShowCommandResult(if cancelled {
                        format!("Background job {id} cancelled")
                    } else {
                        format!("Background job {id} is not running")
                    }))
                    .await;
                return true;
            }
            let result = self
                .background_actor
                .snapshot()
                .into_iter()
                .map(|job| job.terminal_line())
                .collect::<Vec<_>>();
            self.ui
                .send(UiMsg::ShowCommandResult(if result.is_empty() {
                    "No background jobs".into()
                } else {
                    result.join("\n")
                }))
                .await;
            return true;
        }
        if name == "questions" {
            let query = runie_core::tools::parse_question_history_query(args);
            let rows = runie_core::tools::question_history_rows(
                &self.question_broker.traces(),
                &query.text,
                query.outcome.as_deref(),
                32,
            );
            let result = rows
                .into_iter()
                .map(|row| {
                    format!(
                        "{} · {} · {}{}",
                        row.id,
                        row.outcome,
                        row.question,
                        row.detail
                            .map(|detail| format!(" · {detail}"))
                            .unwrap_or_default()
                    )
                })
                .collect::<Vec<_>>();
            self.ui
                .send(UiMsg::ShowCommandResult(if result.is_empty() {
                    "No matching user questions".into()
                } else {
                    result.join("\n")
                }))
                .await;
            return true;
        }
        if name == "effort" && args.trim().is_empty() {
            if let Some(model) = self.model_catalog.snapshot().selected {
                if Self::model_has_declared_effort(&model) {
                    self.open_effort_picker(&model).await;
                } else {
                    self.ui
                        .send(UiMsg::ShowCommandResult(format!(
                            "Model {} declares no reasoning effort levels",
                            model.name
                        )))
                        .await;
                }
            } else {
                self.ui
                    .send(UiMsg::ShowCommandResult(
                        "No model selected for reasoning effort".into(),
                    ))
                    .await;
            }
            return true;
        }
        let state = self.command_actor.invoke(name, args).await;
        if matches!(name, "always-approve" | "auto" | "deny") {
            self.approval_mode.set(match state.approval {
                runie_core::command_actor::ApprovalMode::Always => runie_core::tools::ApprovalMode::Yolo,
                runie_core::command_actor::ApprovalMode::Auto => runie_core::tools::ApprovalMode::Auto,
                runie_core::command_actor::ApprovalMode::Deny => runie_core::tools::ApprovalMode::Deny,
                runie_core::command_actor::ApprovalMode::Ask => runie_core::tools::ApprovalMode::Ask,
            });
        }
        match name {
        "help" => {
                self.ui.send(UiMsg::ToggleCommandPalette).await;
            }
            "context" | "recap" => {
                self.ui.send(UiMsg::ToggleSessionInfo).await;
            }
            "effort" if self.model_supports_effort(args.trim()) => {
                let level = match args.trim() {
                    "off" => runie_core::types::ThinkingLevel::Off,
                    "minimal" => runie_core::types::ThinkingLevel::Minimal,
                    "low" => runie_core::types::ThinkingLevel::Low,
                    "medium" => runie_core::types::ThinkingLevel::Medium,
                    "high" => runie_core::types::ThinkingLevel::High,
                    "xhigh" => runie_core::types::ThinkingLevel::XHigh,
                    "max" => runie_core::types::ThinkingLevel::Max,
                    _ => unreachable!(),
                };
                self.loop_actor.set_thinking_level(level).await;
                self.ui
                    .send(UiMsg::ShowCommandResult(format!("Reasoning effort: {}", args.trim())))
                    .await;
            }
            "effort" => {
                self.ui
                    .send(UiMsg::ShowCommandResult(format!(
                        "Unsupported effort {:?} for the current model",
                        args.trim()
                    )))
                    .await;
            }
            _ => {
                let result = match name {
                    "settings" => format!("Settings updated ({} entries)", state.settings.len()),
                    "share" => format!("Session share created ({})", state.shared_sessions),
                    "feedback" => format!("Feedback recorded ({})", state.feedback.len()),
                    "history" => {
                        format!("History search recorded ({})", state.history_queries.len())
                    }
                    "find" | "jump" => format!(
                        "Transcript query recorded ({})",
                        state.transcript_queries.len()
                    ),
                    "rewind" => {
                        format!("Rewind request recorded ({})", state.rewind_requests.len())
                    }
                    "usage" => format!("Usage requested ({})", state.usage_requests),
                    "effort" => format!(
                        "Reasoning effort: {}",
                        state.effort.as_deref().unwrap_or("unchanged")
                    ),
                    "always-approve" | "auto" | "deny" => format!("Approval mode: {:?}", state.approval),
                    "plan" | "view-plan" => state
                        .plan
                        .clone()
                        .unwrap_or_else(|| {
                            let snapshot = self.todo_actor.snapshot();
                            let summary = self.todo_actor.summary();
                            let mut lines = snapshot.terminal_lines();
                            lines.push(format!(
                                "Todo plan: {:?} · completed={} pending={} in_progress={}",
                                summary.status,
                                summary.completed,
                                summary.pending,
                                summary.in_progress
                            ));
                            lines.join("\n")
                        }),
                    "memory" | "remember" => format!(
                        "Memory: {} ({} notes)",
                        if state.memory_enabled { "on" } else { "off" },
                        state.remembered.len()
                    ),
                    "login" | "logout" => {
                        format!("Authenticated providers: {}", state.authenticated.len())
                    }
                    "goal" => state
                        .active_goal
                        .clone()
                        .unwrap_or_else(|| "No active goal".into()),
                    "workflow" | "workflows" => format!("Workflow runs: {}", state.workflows.len()),
                    "loop" => format!("Scheduled loops: {}", state.scheduled_loops.len()),
                    "deep-research" => {
                        format!("Research queries: {}", state.research_queries.len())
                    }
                    "trust" => format!("Trusted projects: {}", state.trusted_projects.len()),
                    "reload" => format!("Resources reloaded {} time(s)", state.reload_count),
                    "skills" => "Skills enabled".into(),
                    "hooks" => "Hooks loaded".into(),
                    "plugins" => "Plugins loaded".into(),
                    "mcps" => "MCP servers loaded".into(),
                    "doctor" => state
                        .diagnostic_report
                        .as_ref()
                        .map(|report| report.terminal_lines().join("\n"))
                        .unwrap_or_else(|| "Diagnostic complete".into()),
                    _ => format!(
                        "/{name} accepted{}",
                        if args.trim().is_empty() {
                            String::new()
                        } else {
                            format!(": {args}")
                        }
                    ),
                };
                self.ui.send(UiMsg::ShowCommandResult(result)).await;
                self.bus.publish(AgentEvent::CustomSessionEntryCreated {
                    custom_type: format!("runie.command.{name}"),
                    data: Some(serde_json::json!({"args": args})),
                });
                self.session_actor.flush().await;
            }
        }
        true
    }
}
