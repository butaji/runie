use super::{UiActor, UiMsg, UiState};

#[test]
fn palette_registry_maps_every_visible_entry_to_a_typed_action() {
    assert!(super::PaletteAction::labels()
        .iter()
        .all(|label| super::palette_action_for(label).is_some()));
    assert!(super::palette_action_for("unknown command").is_none());
}

#[tokio::test]
async fn ui_actor_turns_parameter_form_submission_into_a_builtin_command() {
    let bus = runie_core::events::EventBus::new();
    let actor = UiActor::new(&bus);
    let mut commands = actor.subscribe_commands();
    actor
        .send(UiMsg::OpenPaletteParameters(
            super::PaletteAction::ExportSession,
        ))
        .await;
    for ch in "x.jsonl".chars() {
        actor.send(UiMsg::PaletteParameterChar(ch)).await;
    }
    actor.send(UiMsg::PaletteParameterSubmit).await;
    assert_eq!(
        commands.recv().await.unwrap(),
        super::UiCommand::ExecuteMappable(runie_core::commands::MappableBuiltinCommand::Export {
            path: "x.jsonl".into()
        })
    );
}

#[tokio::test]
async fn ui_actor_publishes_select_model_as_an_immediate_picker_action() {
    let bus = runie_core::events::EventBus::new();
    let actor = UiActor::new(&bus);
    let mut commands = actor.subscribe_commands();
    actor.send(UiMsg::ToggleCommandPalette).await;
    for ch in "model".chars() {
        actor.send(UiMsg::CommandPaletteChar(ch)).await;
    }
    actor.send(UiMsg::ActivateCommandPalette).await;
    assert_eq!(
        commands.recv().await.unwrap(),
        super::UiCommand::ActivatePaletteEntry(super::PaletteAction::SelectModel)
    );
}

#[test]
fn parameterized_palette_session_flows_emit_typed_commands() {
    for (action, input, expected) in [
        (super::PaletteAction::SetSessionName, "demo", "/name demo"),
        (
            super::PaletteAction::CompactContext,
            "keep tools",
            "/compact keep tools",
        ),
        (
            super::PaletteAction::ForkSession,
            "entry-1",
            "/fork entry-1",
        ),
        (
            super::PaletteAction::SelectTreeEntry,
            "entry-2",
            "/tree entry-2",
        ),
    ] {
        assert_parameter_flow(action, input, expected);
    }
}

#[test]
fn session_history_query_palette_action_emits_extended_command() {
    assert_parameter_flow(
        super::PaletteAction::SessionHistoryQuery,
        "active_tools",
        "/sessions history query active_tools",
    );
}

#[test]
fn cancel_running_jobs_palette_action_emits_extended_command() {
    assert_parameter_flow(
        super::PaletteAction::CancelRunningJobs,
        "",
        "/jobs cancel running",
    );
}

#[test]
fn cancel_queued_jobs_palette_action_emits_extended_command() {
    assert_parameter_flow(
        super::PaletteAction::CancelQueuedJobs,
        "",
        "/jobs scheduler cancel queued",
    );
}

#[test]
fn close_mcp_palette_action_emits_extended_command() {
    assert_parameter_flow(super::PaletteAction::CloseMcps, "", "/mcps close");
}

#[test]
fn reconnect_mcp_palette_action_emits_extended_command() {
    assert_parameter_flow(super::PaletteAction::ReconnectMcps, "", "/mcps reconnect");
}

#[test]
fn usage_chart_palette_action_emits_extended_command() {
    assert_parameter_flow(super::PaletteAction::UsageChart, "", "/usage chart");
}

#[test]
fn context_policy_palette_action_emits_extended_command() {
    assert_parameter_flow(super::PaletteAction::ContextPolicy, "", "/context policy");
}

#[test]
fn pending_questions_palette_action_emits_extended_command() {
    assert_parameter_flow(
        super::PaletteAction::PendingQuestions,
        "",
        "/questions pending",
    );
}

#[test]
fn ask_before_tools_palette_action_emits_extended_command() {
    assert_parameter_flow(super::PaletteAction::AskBeforeTools, "", "/ask");
}

#[test]
fn running_jobs_palette_action_emits_extended_command() {
    assert_parameter_flow(super::PaletteAction::RunningJobs, "", "/jobs running");
}

#[test]
fn queued_jobs_palette_action_emits_extended_command() {
    assert_parameter_flow(
        super::PaletteAction::QueuedJobs,
        "",
        "/jobs scheduler queued",
    );
}

#[test]
fn pop_mcp_notification_palette_action_emits_extended_command() {
    assert_parameter_flow(
        super::PaletteAction::McpPopNotifications,
        "",
        "/mcps notifications pop",
    );
}

#[test]
fn mcp_notification_palette_actions_emit_extended_commands() {
    assert_parameter_flow(
        super::PaletteAction::McpNotifications,
        "",
        "/mcps notifications",
    );
    assert_parameter_flow(
        super::PaletteAction::ClearMcpNotifications,
        "",
        "/mcps notifications clear",
    );
}

#[test]
fn parameterized_palette_storage_flows_emit_typed_commands() {
    for (action, input, expected) in [
        (
            super::PaletteAction::ExportSession,
            "session.jsonl",
            "/export session.jsonl",
        ),
        (
            super::PaletteAction::ImportSession,
            "session.jsonl",
            "/import session.jsonl",
        ),
        (
            super::PaletteAction::CloneSession,
            "session.jsonl",
            "/clone session.jsonl",
        ),
        (
            super::PaletteAction::ResumeSession,
            "session.jsonl",
            "/resume session.jsonl",
        ),
    ] {
        assert_parameter_flow(action, input, expected);
    }
}

#[test]
fn extended_palette_parameters_emit_typed_invocations() {
    for (action, input, name) in [
        (super::PaletteAction::Settings, "theme=dark", "settings"),
        (super::PaletteAction::Login, "openai", "login"),
        (super::PaletteAction::PlanMode, "ship it", "plan"),
        (super::PaletteAction::Remember, "keep this", "remember"),
    ] {
        let mut state = UiState::new().update(UiMsg::OpenPaletteParameters(action.clone()));
        for ch in input.chars() {
            state = state.update(UiMsg::PaletteParameterChar(ch));
        }
        assert_eq!(
            super::app_projection::ui_command_for(&state, &UiMsg::PaletteParameterSubmit),
            Some(super::UiCommand::ExecuteMappable(
                runie_core::commands::MappableBuiltinCommand::Extended {
                    name: name.into(),
                    args: input.into(),
                }
            ))
        );
    }
}

#[test]
fn multiword_git_palette_routes_preserve_command_prefix_data() {
    for (action, args) in [
        (super::PaletteAction::GitPush, "origin main"),
        (super::PaletteAction::GitRevert, "deadbee"),
    ] {
        let mut state = UiState::new().update(UiMsg::OpenPaletteParameters(action.clone()));
        for ch in args.chars() {
            state = state.update(UiMsg::PaletteParameterChar(ch));
        }
        assert_eq!(
            super::app_projection::ui_command_for(&state, &UiMsg::PaletteParameterSubmit),
            Some(super::UiCommand::ExecuteMappable(
                runie_core::commands::MappableBuiltinCommand::Extended {
                    name: "git".into(),
                    args: format!(
                        "{} {args}",
                        if action == super::PaletteAction::GitPush {
                            "push"
                        } else {
                            "revert"
                        }
                    ),
                }
            ))
        );
    }
}

#[test]
fn job_output_facts_palette_parameter_emits_typed_command() {
    let mut state = UiState::new().update(UiMsg::OpenPaletteParameters(
        super::PaletteAction::JobOutputFacts,
    ));
    for ch in "job-7".chars() {
        state = state.update(UiMsg::PaletteParameterChar(ch));
    }
    assert_eq!(
        super::app_projection::ui_command_for(&state, &UiMsg::PaletteParameterSubmit),
        Some(super::UiCommand::ExecuteMappable(
            runie_core::commands::MappableBuiltinCommand::Extended {
                name: "jobs".into(),
                args: "output job-7 facts".into(),
            }
        ))
    );
}

#[test]
fn job_output_window_palette_parameters_emit_typed_commands() {
    for (action, direction) in [
        (super::PaletteAction::JobOutputHead, "head"),
        (super::PaletteAction::JobOutputTail, "tail"),
    ] {
        let mut state = UiState::new().update(UiMsg::OpenPaletteParameters(action));
        for ch in "job-7 3".chars() {
            state = state.update(UiMsg::PaletteParameterChar(ch));
        }
        assert_eq!(
            super::app_projection::ui_command_for(&state, &UiMsg::PaletteParameterSubmit),
            Some(super::UiCommand::ExecuteMappable(
                runie_core::commands::MappableBuiltinCommand::Extended {
                    name: "jobs".into(),
                    args: format!("output job-7 {direction} 3"),
                }
            ))
        );
    }
}

fn assert_parameter_flow(action: super::PaletteAction, input: &str, expected: &str) {
    let mut state = UiState::new().update(UiMsg::OpenPaletteParameters(action));
    for ch in input.chars() {
        state = state.update(UiMsg::PaletteParameterChar(ch));
    }
    let command = super::app_projection::ui_command_for(&state, &UiMsg::PaletteParameterSubmit)
        .expect("parameter flow must emit a command");
    let parsed = runie_core::commands::parse_mappable_builtin_command(expected)
        .expect("fixture command must be mappable");
    assert_eq!(command, super::UiCommand::ExecuteMappable(parsed));
}

#[test]
fn every_palette_command_projects_to_an_executable_flow() {
    for label in super::PaletteAction::labels() {
        let mut state = UiState::new().update(UiMsg::ToggleCommandPalette);
        for ch in label.chars() {
            state = state.update(UiMsg::CommandPaletteChar(ch));
        }
        let command =
            super::app_projection::palette_command_for(&state, &UiMsg::ActivateCommandPalette)
                .unwrap_or_else(|| panic!("no projected flow for palette command {label}"));
        let action = super::PaletteAction::from_label(label).unwrap();
        if action.requires_parameters() {
            assert_eq!(command, super::UiCommand::OpenPaletteParameters(action));
        } else {
            assert_eq!(command, super::UiCommand::ActivatePaletteEntry(action));
        }
    }
}
