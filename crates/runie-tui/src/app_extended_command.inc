impl App {
    async fn route_extended_command(&self, name: &str, args: &str) -> bool {
        if matches!(name, "clear" | "reset") && args.trim().is_empty() {
            self.session_actor.reset().await;
            self.ui
                .send(UiMsg::ShowCommandResult("The context has been cleared.".into()))
                .await;
            return true;
        }
        if name == "mcps" {
            let transport = runie_core::commands::parse_mcp_transport_query(args);
            let statuses = self
                .loop_actor
                .mcp_status_rows()
                .into_iter()
                .filter(|row| transport.is_none_or(|transport| row.transport == transport))
                .collect::<Vec<_>>();
            let result = if statuses.is_empty() {
                transport
                    .map(|transport| format!("No MCP {transport} servers"))
                    .unwrap_or_else(|| "No MCP stdio servers".into())
            } else {
                statuses
                    .into_iter()
                    .map(|row| format!("{}[{}] status={}", row.transport, row.index, row.status))
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            self.ui.send(UiMsg::ShowCommandResult(result)).await;
            return true;
        }
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
            if runie_core::commands::parse_background_job_clear_finished(args) {
                let cleared = self.background_actor.clear_finished().await;
                self.ui
                    .send(UiMsg::ShowCommandResult(format!(
                        "Cleared {cleared} finished background job(s)"
                    )))
                    .await;
                return true;
            }
            if runie_core::commands::parse_background_job_cancel_all(args) {
                let cancelled = self.background_actor.cancel_all().await;
                self.ui
                    .send(UiMsg::ShowCommandResult(format!(
                        "Cancelled {cancelled} running background job(s)"
                    )))
                    .await;
                return true;
            }
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
            if let Some(status) = runie_core::commands::parse_background_job_status_query(args) {
                let result = runie_core::background::background_job_summaries(
                    &self.background_actor.snapshot(),
                )
                .into_iter()
                .filter(|job| format!("{:?}", job.status).eq_ignore_ascii_case(status))
                .map(|job| format!(
                    "{} · {:?} · {} exit={:?} · output={} lines/{} bytes{}",
                    job.id,
                    job.status,
                    job.command,
                    job.exit_code,
                    job.output_lines,
                    job.output_bytes,
                    if job.truncated { " truncated" } else { "" },
                ))
                .collect::<Vec<_>>();
                self.ui
                    .send(UiMsg::ShowCommandResult(if result.is_empty() {
                        format!("No {status} background jobs")
                    } else {
                        result.join("\n")
                    }))
                    .await;
                return true;
            }
            if let Some(id) = runie_core::commands::parse_background_job_query(args) {
                let result = match runie_core::background::background_job_summaries(
                    &self.background_actor.snapshot(),
                )
                .into_iter()
                .find(|job| job.id == id)
                {
                    Some(job) => {
                        let output = self.background_actor.read_output(id).await.unwrap_or_default();
                        format!("{}\noutput:\n{}", background_job_line(&job), output)
                    }
                    None => format!("Background job {id} not found"),
                };
                self.ui.send(UiMsg::ShowCommandResult(result)).await;
                return true;
            }
            let mut result = runie_core::background::background_job_summaries(
                &self.background_actor.snapshot(),
            )
                .into_iter()
                .map(|job| format!(
                    "{} · {:?} · {} exit={:?} · output={} lines/{} bytes{}",
                    job.id,
                    job.status,
                    job.command,
                    job.exit_code,
                    job.output_lines,
                    job.output_bytes,
                    if job.truncated { " truncated" } else { "" },
                ))
                .collect::<Vec<_>>();
            result.extend(
                self.loop_actor
                    .scheduler_metrics()
                    .await
                    .terminal_lines()
                    .into_iter()
                    .map(|line| format!("scheduler {line}")),
            );
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
            if args.trim().eq_ignore_ascii_case("clear") {
                self.question_broker.clear_traces();
                self.ui
                    .send(UiMsg::ShowCommandResult("User-question history cleared".into()))
                    .await;
                return true;
            }
            let query = runie_core::tools::parse_question_history_query(args);
            let rows = runie_core::tools::question_history_rows_page(
                &self.question_broker.traces(),
                &query.text,
                query.outcome.as_deref(),
                query.offset,
                query.limit,
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
            "context" => {
                if let Some(instructions) = args.trim().strip_prefix("compact") {
                    if instructions.is_empty()
                        || instructions.starts_with(' ')
                        || instructions.starts_with('\t')
                    {
                        let instructions = instructions.trim();
                        let result = self
                            .compact_session(
                                None,
                                (!instructions.is_empty()).then(|| instructions.to_owned()),
                            )
                            .await
                            .map(|()| "Context compacted".to_owned())
                            .unwrap_or_else(|error| format!("Compaction failed: {error}"));
                        self.ui.send(UiMsg::ShowCommandResult(result)).await;
                        return true;
                    }
                }
                let snapshot = self.session_actor.snapshot();
                let messages = snapshot.branch_context_messages();
                let estimate = runie_core::session::estimate_context_tokens(&messages);
                let model = self.model_catalog.snapshot().selected;
                let mut lines = vec![
                    format!("context_tokens: {}", estimate.tokens),
                    format!(
                        "context_window: {}",
                        model.as_ref().map(|model| model.context_window).unwrap_or(0)
                    ),
                ];
                let context_window = model.as_ref().map(|model| model.context_window).unwrap_or(0);
                let settings = runie_core::session::CompactionSettings {
                    enabled: true,
                    reserve_tokens: 20_000,
                    keep_recent_tokens: 20,
                };
                lines.extend(
                    runie_core::session::compaction_decision(
                        estimate.tokens,
                        context_window,
                        settings,
                    )
                    .terminal_lines(),
                );
                if let Some(projection) = snapshot.compaction_context_projection() {
                    lines.extend(projection.terminal_lines());
                } else {
                    lines.push("compaction: none".into());
                }
                let result = lines.join("\n");
                self.ui.send(UiMsg::ShowCommandResult(result)).await;
            }
            "recap" => {
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
                    "usage" => {
                        let usage = runie_core::telemetry::usage_summary(&self.telemetry.snapshot());
                        format!(
                            "Usage requested ({})\nrequests: {}\ninput_tokens: {}\noutput_tokens: {}\ntotal_tokens: {}\ncost: {:.6}",
                            state.usage_requests,
                            usage.requests,
                            usage.input_tokens,
                            usage.output_tokens,
                            usage.total_tokens,
                            usage.cost
                        )
                    }
                    "effort" => format!(
                        "Reasoning effort: {}",
                        state.effort.as_deref().unwrap_or("unchanged")
                    ),
                    "always-approve" | "auto" | "deny" => format!("Approval mode: {:?}", state.approval),
                    "plan" => format!(
                        "Plan mode {}",
                        if state.plan_mode { "ON" } else { "OFF" }
                    ),
                    "view-plan" => state.plan.clone().unwrap_or_else(|| {
                            let snapshot = self.todo_actor.snapshot();
                            let summary = self.todo_actor.summary();
                            let mut lines = snapshot.terminal_lines();
                            lines.push(summary.terminal_line());
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
                    "plugins" => self
                        .plugin_host
                        .as_ref()
                        .map(|host| host.snapshot().terminal_lines().join("\n"))
                        .filter(|lines| !lines.is_empty())
                        .unwrap_or_else(|| "Plugins loaded".into()),
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

fn background_job_line(job: &runie_core::background::BackgroundJobSummary) -> String {
    format!(
        "{} · {:?} · {} exit={:?} · output={} lines/{} bytes{}",
        job.id,
        job.status,
        job.command,
        job.exit_code,
        job.output_lines,
        job.output_bytes,
        if job.truncated { " truncated" } else { "" },
    )
}
