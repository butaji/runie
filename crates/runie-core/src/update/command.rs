use crate::model::AppState;
use crate::session::Session;

use super::dialog;

pub(super) fn handle_command_event(state: &mut AppState, event: crate::Event) {
    use crate::commands::CommandResult;
    match &event {
        crate::Event::RunLoadCommand { name } => run_load_command(state, name),
        crate::Event::RunSaveCommand { name } => run_save_command(state, name),
        crate::Event::RunDeleteCommand { name } => run_delete_command(state, name),
        crate::Event::RunImportCommand { path } => run_import_command(state, path),
        crate::Event::RunExportCommand { path } => run_export_command(state, path),
        crate::Event::RunSkillCommand { name } => run_skill_command(state, name),
        crate::Event::RunLoginCommand { .. } => {
            dialog::process_command_result(
                state,
                CommandResult::OpenDialog(crate::commands::DialogType::CommandPalette),
            );
        }
        crate::Event::RunLogoutCommand { provider } => run_logout_command(state, provider),
        crate::Event::RunNameCommand { name } => {
            crate::commands::dsl::handlers::session::run_name(state, name);
        }
        crate::Event::RunForkCommand { message_index } => {
            crate::commands::dsl::handlers::session::run_fork(state, message_index);
        }
        crate::Event::RunCompactCommand { keep, focus } => {
            crate::commands::dsl::handlers::session::run_compact(state, keep, focus);
        }
        crate::Event::RunPromptCommand { name } => {
            crate::commands::dsl::handlers::system::run_prompt(state, name);
        }
        crate::Event::RunThinkingCommand { level } => {
            crate::commands::dsl::handlers::model::run_thinking(state, *level);
        }
        crate::Event::RunPaletteCommand { name, args } => {
            run_palette_command(state, name, args);
        }
        // intentionally ignored: other command events fall through
        _ => {}
    }
}

fn run_load_command(state: &mut AppState, name: &str) {
    if send_to_session_store(state, |tx, name| async move { tx.load(name).await }, name) {
        return;
    }
    use crate::commands::CommandResult;
    let result = crate::session::replay::load_session(name, state)
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
    let name_owned = name.to_string();
    let session = crate::session::Session::from_state(state, name_owned.clone());
    if let Some(tx) = state.persistence_tx.clone() {
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            let _ = handle.spawn(async move { tx.save(name_owned, session).await; });
            return;
        }
    }
    use crate::commands::CommandResult;
    let result = crate::session::replay::save_session(name, state)
        .map(|_| CommandResult::Message(format!("Session '{}' saved.", name)))
        .unwrap_or_else(|e| CommandResult::Message(format!("Could not save session: {}", e)));
    dialog::process_command_result(state, result);
}

fn run_delete_command(state: &mut AppState, name: &str) {
    if send_to_session_store(state, |tx, name| async move { tx.delete(name).await }, name) {
        return;
    }
    use crate::commands::CommandResult;
    let result = crate::session::replay::delete_session(name)
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
    if let Some(tx) = state.persistence_tx.clone() {
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            let path = std::path::PathBuf::from(path);
            let _ = handle.spawn(async move { tx.import(path).await; });
            return;
        }
    }
    use crate::commands::CommandResult;
    let path_buf = path.to_string();
    let result = crate::async_io::block_in_place_if_runtime(|| {
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
    if let Some(tx) = state.persistence_tx.clone() {
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            let name = state
                .session
                .session_display_name
                .clone()
                .unwrap_or_else(|| "exported".into());
            let session = Session::from_state(state, name);
            let path = std::path::PathBuf::from(path);
            let _ = handle.spawn(async move { tx.export(path, session).await; });
            return;
        }
    }
    use crate::commands::CommandResult;
    let name = state
        .session
        .session_display_name
        .clone()
        .unwrap_or_else(|| "exported".into());
    let session = Session::from_state(state, name);
    let path_buf = path.to_string();
    let json = serde_json::to_string_pretty(&session).unwrap_or_default();
    let result = crate::async_io::block_in_place_if_runtime(|| std::fs::write(&path_buf, json))
        .map(|_| CommandResult::Message(format!("Session exported to '{}'", path)))
        .unwrap_or_else(|e| CommandResult::Message(format!("Could not export: {}", e)));
    dialog::process_command_result(state, result);
}

fn send_to_session_store<F, Fut>(state: &AppState, f: F, name: &str) -> bool
where
    F: FnOnce(crate::actors::SessionActorHandle, String) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    if let Some(tx) = state.persistence_tx.clone() {
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            let name = name.to_string();
            let _ = handle.spawn(async move { f(tx, name).await; });
            return true;
        }
    }
    false
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
    state.remove_provider(provider);
    if state.config.current_provider == provider {
        let (provider, model) = state.resolve_default_model();
        state.set_active_model(
            provider,
            model,
            crate::state::ModelSource::ConfigDefault,
        );
    }
    if !state.has_models() {
        crate::login_flow::login_flow_start(state);
    }
    dialog::process_command_result(
        state,
        CommandResult::Message(format!(
            "Disconnected '{}'. Use /providers to manage providers.",
            provider
        )),
    );
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
