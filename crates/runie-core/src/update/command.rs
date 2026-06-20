use crate::event::CommandEvent;
use crate::model::AppState;
use crate::session::Session;

use super::dialog;

pub(super) fn handle_command_event(state: &mut AppState, event: CommandEvent) {
    use crate::commands::CommandResult;
    match &event {
        CommandEvent::RunLoadCommand { name } => run_load_command(state, name),
        CommandEvent::RunSaveCommand { name } => run_save_command(state, name),
        CommandEvent::RunDeleteCommand { name } => run_delete_command(state, name),
        CommandEvent::RunImportCommand { path } => run_import_command(state, path),
        CommandEvent::RunExportCommand { path } => run_export_command(state, path),
        CommandEvent::RunSkillCommand { name } => run_skill_command(state, name),
        CommandEvent::RunLoginCommand { .. } => {
            dialog::process_command_result(
                state,
                CommandResult::OpenDialog(crate::commands::DialogType::CommandPalette),
            );
        }
        CommandEvent::RunLogoutCommand { provider } => run_logout_command(state, provider),
        CommandEvent::RunNameCommand { name } => {
            crate::commands::dsl::handlers::session::run_name(state, name);
        }
        CommandEvent::RunForkCommand { message_index } => {
            crate::commands::dsl::handlers::session::run_fork(state, message_index);
        }
        CommandEvent::RunCompactCommand { keep, focus } => {
            crate::commands::dsl::handlers::session::run_compact(state, keep, focus);
        }
        CommandEvent::RunPromptCommand { name } => {
            crate::commands::dsl::handlers::system::run_prompt(state, name);
        }
        CommandEvent::RunThinkingCommand { level } => {
            crate::commands::dsl::handlers::model::run_thinking(state, *level);
        }
        CommandEvent::RunPaletteCommand { name, args } => {
            run_palette_command(state, name, args);
        }
        _ => {}
    }
}

fn run_load_command(state: &mut AppState, name: &str) {
    use crate::commands::CommandResult;
    let result = crate::async_io::block_in_place_if_runtime(|| crate::session_replay::load_session(name, state))
        .map(|_| CommandResult::Message(format!("Session '{}' loaded.", name)))
        .unwrap_or_else(|_| {
            CommandResult::Message(format!(
                "Session '{}' not found. Use /sessions to list saved sessions.",
                name
            ))
        });
    dialog::process_command_result(state, result);
}

fn run_save_command(state: &mut AppState, name: &str) {
    use crate::commands::CommandResult;
    let result = crate::async_io::block_in_place_if_runtime(|| crate::session_replay::save_session(name, state))
        .map(|_| CommandResult::Message(format!("Session '{}' saved.", name)))
        .unwrap_or_else(|e| CommandResult::Message(format!("Could not save session: {}", e)));
    dialog::process_command_result(state, result);
}

fn run_delete_command(state: &mut AppState, name: &str) {
    use crate::commands::CommandResult;
    let result = crate::async_io::block_in_place_if_runtime(|| crate::session_replay::delete_session(name))
        .map(|_| CommandResult::Message(format!("Session '{}' deleted.", name)))
        .unwrap_or_else(|_| {
            CommandResult::Message(format!(
                "Session '{}' not found. Use /sessions to list saved sessions.",
                name
            ))
        });
    dialog::process_command_result(state, result);
}

fn run_import_command(state: &mut AppState, path: &str) {
    use crate::commands::CommandResult;
    let path_buf = path.to_string();
    let result = crate::async_io::block_in_place_if_runtime(move || {
        std::fs::read_to_string(&path_buf)
            .ok()
            .and_then(|json| serde_json::from_str::<Session>(&json).ok())
    })
    .map(|session| {
        let msg = format!("Session imported from '{}'", path);
        state.restore_session(&session);
        CommandResult::Message(msg)
    })
    .unwrap_or_else(|| {
        CommandResult::Message(format!("Could not import session from '{}'", path))
    });
    dialog::process_command_result(state, result);
}

fn run_export_command(state: &mut AppState, path: &str) {
    use crate::commands::CommandResult;
    let name = state
        .session
        .session_display_name
        .clone()
        .unwrap_or_else(|| "exported".into());
    let session = Session::from_state(state, name);
    let path_buf = path.to_string();
    let json = serde_json::to_string_pretty(&session).unwrap_or_default();
    let result = crate::async_io::block_in_place_if_runtime(move || std::fs::write(&path_buf, json))
        .map(|_| CommandResult::Message(format!("Session exported to '{}'", path)))
        .unwrap_or_else(|e| CommandResult::Message(format!("Could not export: {}", e)));
    dialog::process_command_result(state, result);
}

fn run_skill_command(state: &mut AppState, name: &str) {
    use crate::commands::CommandResult;
    let result = state
        .skills
        .iter()
        .find(|s| s.name == name)
        .map(|skill| {
            let mut lines = vec![format!("Skill: {}", skill.name)];
            if !skill.description.is_empty() {
                lines.push(format!("Description: {}", skill.description));
            }
            if !skill.context.is_empty() {
                lines.push(format!("Context: {}", skill.context));
            }
            CommandResult::Message(lines.join("\n"))
        })
        .unwrap_or_else(|| {
            CommandResult::Message(format!(
                "Skill '{}' not found. Use /skills to list loaded skills.",
                name
            ))
        });
    dialog::process_command_result(state, result);
}

fn run_logout_command(state: &mut AppState, provider: &str) {
    use crate::commands::CommandResult;
    if provider.is_empty() {
        dialog::process_command_result(
            state,
            CommandResult::OpenDialog(crate::commands::DialogType::CommandPalette),
        );
        return;
    }
    match crate::login_config::remove_provider_config(provider) {
        Ok(()) => {
            if state.config.current_provider == provider {
                let (provider, model) =
                    crate::login_config::with_read_lock(|config| config.resolve_default_model());
                state.set_active_model(
                    provider,
                    model,
                    crate::state::ModelSource::ConfigDefault,
                );
            }
            if !state.has_models() {
                crate::update::login_flow::login_flow_start(state);
            }
            dialog::process_command_result(
                state,
                CommandResult::Message(format!(
                    "Disconnected '{}'. Use /providers to manage providers.",
                    provider
                )),
            );
        }
        Err(e) => dialog::process_command_result(
            state,
            CommandResult::Message(format!("Could not remove provider config: {}", e)),
        ),
    }
}

fn run_palette_command(state: &mut AppState, name: &str, args: &str) {
    use crate::commands::CommandResult;
    let result = if let Some(cmd) = state.registry.get(name) {
        let cmd_name = cmd.name.clone();
        cmd.flow.clone().exec(state, &cmd_name, args)
    } else {
        CommandResult::Message(format!("Unknown command: /{}", name))
    };
    dialog::process_command_result(state, result);
}
