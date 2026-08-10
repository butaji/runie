impl App {
    async fn route_extended_command(&self, name: &str, args: &str) -> bool {
        let state = self.command_actor.invoke(name, args).await;
        match name {
            "help" => {
                self.ui.send(UiMsg::ToggleCommandPalette).await;
            }
            "context" | "recap" => {
                self.ui.send(UiMsg::ToggleSessionInfo).await;
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
                    "always-approve" | "auto" => format!("Approval mode: {:?}", state.approval),
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
