impl App {
    async fn route_extended_command(&self, name: &str, args: &str) -> bool {
        if name == "sessions" {
            let root = if args.trim().is_empty() {
                std::env::current_dir()
                    .map(|path| path.to_string_lossy().into_owned())
                    .unwrap_or_else(|_| ".".into())
            } else {
                args.trim().to_owned()
            };
            let result = match self.session_storage.discover_rows(root, 64, "").await {
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
                        .unwrap_or_else(|| "No active plan".into()),
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
                    "jobs" => serde_json::to_string(&self.background_actor.snapshot())
                        .unwrap_or_else(|_| "Background jobs unavailable".into()),
                    "doctor" => state
                        .last_diagnostic
                        .clone()
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
