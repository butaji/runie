impl App {
    #[allow(clippy::cognitive_complexity, clippy::too_many_lines, reason = "The extended command registry is the single typed routing boundary")]
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
            let status = runie_core::commands::parse_mcp_status_query(args);
            let statuses = self
                .loop_actor
                .mcp_status_rows()
                .into_iter()
                .filter(|row| transport.is_none_or(|transport| row.transport == transport))
                .filter(|row| status.is_none_or(|status| row.status == status))
                .collect::<Vec<_>>();
            let result = if statuses.is_empty() {
                status
                    .map(|status| format!("No MCP {status} servers"))
                    .or_else(|| {
                        transport
                    .map(|transport| format!("No MCP {transport} servers"))
                    })
                    .unwrap_or_else(|| "No MCP stdio servers".into())
            } else {
                statuses
                    .into_iter()
                    .map(|row| row.terminal_line())
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            self.ui.send(UiMsg::ShowCommandResult(result)).await;
            return true;
        }
        if name == "git"
            && matches!(
                args.trim().to_ascii_lowercase().as_str(),
                "status" | "diff" | "review" | "worktrees" | "conflicts"
            )
        {
            let result = match if args.trim().eq_ignore_ascii_case("diff") {
                runie_core::tools::GitDiffTool
                    .execute("git-diff", serde_json::json!({}), None, None)
                    .await
            } else if args.trim().eq_ignore_ascii_case("review") {
                runie_core::tools::GitReviewTool
                    .execute("git-review", serde_json::json!({}), None, None)
                    .await
            } else if args.trim().eq_ignore_ascii_case("worktrees") {
                runie_core::tools::GitWorktreeTool
                    .execute("git-worktrees", serde_json::json!({}), None, None)
                    .await
            } else {
                runie_core::tools::GitStatusTool
                    .execute("git-status", serde_json::json!({}), None, None)
                    .await
            }
            {
                Ok(result) => result
                    .content
                    .into_iter()
                    .find_map(|content| match content {
                        runie_core::types::ToolResultContent::Text { text } => Some(text),
                        runie_core::types::ToolResultContent::Image { .. } => None,
                    })
                    .map(|status| {
                        if args.trim().eq_ignore_ascii_case("conflicts") {
                            runie_core::tools::classify_conflicts(&status)
                                .terminal_lines()
                                .join("\n")
                        } else {
                            status
                        }
                    })
                    .unwrap_or_else(|| "Git status returned no text".into()),
                Err(error) => format!("Git status failed: {error}"),
            };
            self.ui.send(UiMsg::ShowCommandResult(result)).await;
            return true;
        }
        if name == "sessions" {
            if let Some(target_id) = runie_core::commands::parse_session_history_selection(args) {
                let result = self
                    .session_actor
                    .select_tree(target_id.to_owned())
                    .await
                    .map(|()| format!("Selected session history entry {target_id}"))
                    .unwrap_or_else(|error| format!("Session history selection failed: {error}"));
                self.ui.send(UiMsg::ShowCommandResult(result)).await;
                return true;
            }
            if args.trim().eq_ignore_ascii_case("history") {
                let result = self
                    .session_actor
                    .snapshot()
                    .history_rows()
                    .into_iter()
                    .map(|row| row.terminal_line())
                    .collect::<Vec<_>>();
                self.ui
                    .send(UiMsg::ShowCommandResult(if result.is_empty() {
                        "No session history".into()
                    } else {
                        result.join("\n")
                    }))
                    .await;
                return true;
            }
            let root = std::env::current_dir()
                .map(|path| path.to_string_lossy().into_owned())
                .unwrap_or_else(|_| ".".into());
            if let Some(query) = runie_core::commands::parse_session_picker_query(args) {
                let options = self
                    .session_storage
                    .discover_rows(root, 64, &query)
                    .await
                    .unwrap_or_default()
                    .into_iter()
                    .map(|row| row.path)
                    .collect();
                self.ui
                    .send(UiMsg::SetPaletteParameterOptions(options))
                    .await;
                self.ui
                    .send(UiMsg::OpenPaletteParameters(PaletteAction::ResumeSession))
                    .await;
                return true;
            }
            let result = match self
                .session_storage
                .discover_rows(root, 64, args.trim())
                .await
            {
                Ok(rows) if rows.is_empty() => "No stored sessions found".into(),
                Ok(rows) => rows
                    .into_iter()
                    .map(|row| row.terminal_line())
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
            let Some(count) = runie_core::commands::parse_undo_count(args) else {
                self.ui
                    .send(UiMsg::ShowCommandResult(
                        "Undo expects a positive integer count".into(),
                    ))
                    .await;
                return true;
            };
            let result = self.undo_session(count).await;
            if let Err(error) = &result {
                self.bus.publish(AgentEvent::Error {
                    message: error.clone(),
                });
            }
            self.ui
                .send(UiMsg::ShowCommandResult(match result {
                    Ok(done) => format!("Session undone ({done})"),
                    Err(error) => format!("Undo unavailable: {error}"),
                }))
                .await;
            return true;
        }
        if name == "jobs" {
            if let Some(id) = runie_core::commands::parse_background_job_output_query(args) {
                let result = match self.background_actor.read_output(id).await {
                    Some(output) => output.terminal_lines().join("\n"),
                    None => format!("Background job {id} output not found"),
                };
                self.ui.send(UiMsg::ShowCommandResult(result)).await;
                return true;
            }
            if runie_core::commands::parse_background_job_clear_finished(args) {
                let cleared = self.background_actor.clear_finished().await;
                self.ui
                    .send(UiMsg::ShowCommandResult(format!(
                        "Cleared {cleared} finished background job(s)"
                    )))
                    .await;
                return true;
            }
            if let Some(scope) = runie_core::commands::parse_background_job_cancel_scope(args) {
                let cancelled = self.background_actor.cancel_all().await;
                let scope = match scope {
                    runie_core::commands::BackgroundCancelScope::All
                    | runie_core::commands::BackgroundCancelScope::Running => "running",
                };
                self.ui
                    .send(UiMsg::ShowCommandResult(format!(
                        "Cancelled {cancelled} {scope} background job(s)",
                    )))
                    .await;
                return true;
            }
            if runie_core::commands::parse_background_scheduler_active_query(args) {
                let lines = self
                    .loop_actor
                    .scheduler_metrics()
                    .await
                    .active_terminal_lines()
                    .into_iter()
                    .map(|line| format!("scheduler {line}"))
                    .collect::<Vec<_>>();
                self.ui
                    .send(UiMsg::ShowCommandResult(lines.join("\n")))
                    .await;
                return true;
            }
            if runie_core::commands::parse_background_scheduler_query(args) {
                let lines = self
                    .loop_actor
                    .scheduler_metrics()
                    .await
                    .terminal_lines()
                    .into_iter()
                    .map(|line| format!("scheduler {line}"))
                    .collect::<Vec<_>>();
                self.ui
                    .send(UiMsg::ShowCommandResult(lines.join("\n")))
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
                .map(|job| job.terminal_line())
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
                        let output = self.background_actor.read_output(id).await;
                        let output_lines = output
                            .map(|output| output.terminal_lines().join("\n"))
                            .unwrap_or_else(|| "background output unavailable".into());
                        format!("{}\n{}", job.terminal_line(), output_lines)
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
                .map(|job| job.terminal_line())
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
            if args.trim().eq_ignore_ascii_case("pending") {
                let result = self
                    .question_broker
                    .pending()
                    .into_iter()
                    .map(|question| question.terminal_line())
                    .collect::<Vec<_>>();
                self.ui
                    .send(UiMsg::ShowCommandResult(if result.is_empty() {
                        "No pending user questions".into()
                    } else {
                        result.join("\n")
                    }))
                    .await;
                return true;
            }
            if args.trim().eq_ignore_ascii_case("clear") {
                self.question_broker.clear_traces();
                self.ui
                    .send(UiMsg::ShowCommandResult("User-question history cleared".into()))
                    .await;
                return true;
            }
            let query = runie_core::tools::parse_question_history_query(args);
            let page = runie_core::tools::question_history_page(
                &self.question_broker.traces(),
                &query.text,
                query.outcome.as_deref(),
                query.offset,
                query.limit,
            );
            let has_more = page.has_more;
            let has_rows = !page.rows.is_empty();
            let mut result = page
                .rows
                .into_iter()
                .map(|row| row.terminal_line())
                .collect::<Vec<_>>();
            if has_rows {
                result.push(format!(
                    "history offset={} limit={} more={}",
                    query.offset, query.limit, has_more
                ));
            }
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
            self.approval_mode.set(state.approval.clone().into());
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
                let context_window = model.as_ref().map(|model| model.context_window).unwrap_or(0);
                let report = runie_core::session::context_report(
                    estimate.tokens,
                    context_window,
                    runie_core::session::CompactionSettings::default(),
                    snapshot.compaction_context_projection(),
                );
                let result = report.terminal_lines().join("\n");
                self.ui.send(UiMsg::ShowCommandResult(result)).await;
            }
            "recap" => {
                self.ui.send(UiMsg::ToggleSessionInfo).await;
            }
            "effort" if self.model_supports_effort(args.trim()) => {
                let level = runie_core::types::ThinkingLevel::from_name(args.trim())
                    .expect("validated effort");
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
                        match runie_core::commands::parse_usage_limit(args) {
                            Some(limit) => {
                                let usage = runie_core::telemetry::usage_summary_limited(
                                    &self.telemetry.snapshot(),
                                    limit,
                                );
                                format!("Usage requested ({})\n{}", state.usage_requests, usage.terminal_lines().join("\n"))
                            }
                            None => "Usage expects optional `last N`".into(),
                        }
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
                        .map(|host| {
                            host.metadata()
                                .into_iter()
                                .map(|metadata| metadata.terminal_line())
                                .chain(host.snapshot().terminal_lines())
                                .collect::<Vec<_>>()
                                .join("\n")
                        })
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

    async fn undo_session(&self, count: usize) -> Result<usize, String> {
        let mut completed = 0;
        for _ in 0..count {
            self.session_actor.undo().await?;
            completed += 1;
        }
        Ok(completed)
    }
}
