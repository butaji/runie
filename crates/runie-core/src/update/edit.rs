//! Form-submit and edit-event handling.

use crate::commands::CommandResult;
use crate::model::AppState;
use crate::Event;

pub fn update(state: &mut AppState, event: Event) {
    if edit_misc_event(state, &event).is_some() {
        return;
    }
    edit_run_event(state, event);
}

fn edit_misc_event(state: &mut AppState, event: &Event) -> Option<()> {
    match event {
        Event::PendingEdit {
            path,
            original,
            proposed,
            diff,
        } => {
            state.session.pending_edits.push(crate::edit_preview::EditPreview::new(
                std::path::PathBuf::from(path.clone()),
                original.clone(),
                proposed.clone(),
                diff.clone(),
            ));
            state.mark_dirty();
        }
        Event::ApproveEdit => state.approve_edits(),
        Event::RejectEdit => state.reject_edits(),
        Event::ReloadAll => state.reload_all(),
        Event::ShowDiagnostics => state.show_diagnostics(),
        Event::TogglePathCompletion => state.toggle_path_completion(),
        Event::PathCompletionUp => state.path_completion_up(),
        Event::PathCompletionDown => state.path_completion_down(),
        Event::PathCompletionSelect => state.path_completion_select(),
        Event::PathCompletionClose => state.path_completion_close(),
        _ => return None,
    }
    Some(())
}

fn edit_run_event(state: &mut AppState, event: Event) {
    match event {
        Event::RunSaveCommand { name } => run_save_command(state, &name),
        Event::RunLoadCommand { name } => run_load_command(state, &name),
        Event::RunDeleteCommand { name } => run_delete_command(state, &name),
        Event::RunExportCommand { path } => run_export_command(state, &path),
        Event::RunImportCommand { path } => run_import_command(state, &path),
        Event::RunSkillCommand { name } => run_skill_command(state, &name),
        Event::RunLoginCommand { provider, token } => run_login_command(state, &provider, &token),
        Event::RunLogoutCommand { provider } => run_logout_command(state, &provider),
        Event::RunNameCommand { name } => run_name_command(state, &name),
        Event::RunForkCommand { message_index } => run_fork_command(state, &message_index),
        Event::RunCompactCommand { keep, focus } => run_compact_command(state, &keep, &focus),
        Event::RunPromptCommand { name } => run_prompt_command(state, &name),
        Event::RunThinkingCommand { level } => run_thinking_command(state, level),
        Event::RunPaletteCommand { name, args } => run_palette_command(state, &name, &args),
        _ => {}
    }
}

fn run_palette_command(state: &mut AppState, name: &str, args: &str) {
    let result = if let Some(cmd) = state.registry.get(name) {
        let cmd_name = cmd.name.clone();
        cmd.flow.clone().exec(state, &cmd_name, args)
    } else {
        CommandResult::Message(format!("Unknown command: /{}", name))
    };
    super::dialog::process_command_result(state, result);
}

fn run_save_command(state: &mut AppState, name: &str) {
    use crate::session::Session;
    let now = crate::update::now();
    let session = Session {
        name: name.to_string(),
        display_name: state.session.session_display_name.clone(),
        created_at: state.session.session_created_at,
        updated_at: now,
        messages: state.session.messages.clone(),
        provider: state.config.current_provider.clone(),
        model: state.config.current_model.clone(),
        theme_name: state.config.theme_name.clone(),
        thinking_level: state.config.thinking_level,
        read_only: state.config.read_only,
        session_tree: state.session.session_tree.clone(),
    };
    match crate::session::save(name, &session) {
        Ok(()) => {
            state.session.session_updated_at = now;
            state.add_system_msg(format!("Session '{}' saved.", name));
        }
        Err(e) => state.add_system_msg(format!("Could not save '{}': {}", name, e)),
    }
}

fn run_load_command(state: &mut AppState, name: &str) {
    match crate::session::load(name) {
        Ok(session) => {
            state.session.messages = session.messages;
            state.config.current_provider = session.provider;
            state.config.current_model = session.model;
            state.config.theme_name = session.theme_name;
            state.config.thinking_level = session.thinking_level;
            state.config.read_only = session.read_only;
            state.session.session_display_name = session.display_name.or(Some(session.name));
            state.session.session_created_at = session.created_at;
            state.session.session_updated_at = session.updated_at;
            state.session.session_tree = session.session_tree;
            state.messages_changed();
            state.add_system_msg(format!("Session '{}' loaded.", name));
        }
        Err(_) => state.add_system_msg(format!(
            "Session '{}' not found. Use /sessions to list saved sessions.",
            name
        )),
    }
}

fn run_delete_command(state: &mut AppState, name: &str) {
    match crate::session::delete(name) {
        Ok(()) => state.add_system_msg(format!("Session '{}' deleted.", name)),
        Err(_) => state.add_system_msg(format!(
            "Session '{}' not found. Use /sessions to list saved sessions.",
            name
        )),
    }
}

fn run_export_command(state: &mut AppState, path: &str) {
    use crate::session::Session;
    let session = Session {
        name: state
            .session
            .session_display_name
            .clone()
            .unwrap_or_else(|| "exported".into()),
        display_name: state.session.session_display_name.clone(),
        created_at: state.session.session_created_at,
        updated_at: crate::update::now(),
        messages: state.session.messages.clone(),
        provider: state.config.current_provider.clone(),
        model: state.config.current_model.clone(),
        theme_name: state.config.theme_name.clone(),
        thinking_level: state.config.thinking_level,
        read_only: state.config.read_only,
        session_tree: state.session.session_tree.clone(),
    };
    match std::fs::write(
        path,
        serde_json::to_string_pretty(&session).unwrap_or_default(),
    ) {
        Ok(()) => state.add_system_msg(format!("Session exported to '{}'", path)),
        Err(e) => state.add_system_msg(format!("Could not export: {}", e)),
    }
}

fn run_import_command(state: &mut AppState, path: &str) {
    match std::fs::read_to_string(path) {
        Ok(json) => match serde_json::from_str::<crate::session::Session>(&json) {
            Ok(session) => {
                state.session.messages = session.messages;
                state.config.current_provider = session.provider;
                state.config.current_model = session.model;
                state.config.theme_name = session.theme_name;
                state.config.thinking_level = session.thinking_level;
                state.config.read_only = session.read_only;
                state.session.session_display_name = session.display_name.or(Some(session.name));
                state.session.session_created_at = session.created_at;
                state.session.session_updated_at = session.updated_at;
                state.session.session_tree = session.session_tree;
                state.messages_changed();
                state.add_system_msg(format!("Session imported from '{}'", path));
            }
            Err(e) => state.add_system_msg(format!("Invalid session file: {}", e)),
        },
        Err(e) => state.add_system_msg(format!("Could not read file: {}", e)),
    }
}

fn run_skill_command(state: &mut AppState, name: &str) {
    match state.skills.iter().find(|s| s.name == name) {
        Some(skill) => {
            let mut lines = vec![format!("Skill: {}", skill.name)];
            if !skill.description.is_empty() {
                lines.push(format!("Description: {}", skill.description));
            }
            if !skill.context.is_empty() {
                lines.push(format!("Context: {}", skill.context));
            }
            state.add_system_msg(lines.join("\n"));
        }
        None => state.add_system_msg(format!(
            "Skill '{}' not found. Use /skills to list loaded skills.",
            name
        )),
    }
}

fn run_login_command(state: &mut AppState, _provider: &str, _token: &str) {
    // Redirect to the providers dialog for a guided flow
    super::login_flow::providers_event(state, crate::Event::ProvidersDialog);
}

fn run_logout_command(state: &mut AppState, provider: &str) {
    if provider.is_empty() {
        // Redirect to the providers dialog
        super::login_flow::providers_event(state, crate::Event::ProvidersDialog);
        return;
    }
    match crate::login_config::remove_provider_config(provider) {
        Ok(()) => {
            // If this was the active provider, switch to another one or clear.
            if state.config.current_provider == provider {
                let configured = crate::login_config::list_configured_providers();
                if let Some((name, _, models)) = configured.first() {
                    state.config.current_provider = name.clone();
                    state.config.current_model = models.first().cloned().unwrap_or_default();
                } else {
                    state.config.current_provider.clear();
                    state.config.current_model.clear();
                }
            }
            state.add_system_msg(format!(
                "Disconnected '{}'. Use /providers to manage providers.",
                provider
            ));
        }
        Err(e) => state.add_system_msg(format!("Could not remove provider config: {}", e)),
    }
}

fn run_name_command(state: &mut AppState, name: &str) {
    crate::commands::handlers::session::run_name(state, name)
}

fn run_fork_command(state: &mut AppState, index_raw: &str) {
    crate::commands::handlers::session::run_fork(state, index_raw)
}

fn run_compact_command(state: &mut AppState, keep_raw: &str, focus: &str) {
    crate::commands::handlers::session::run_compact(state, keep_raw, focus)
}

fn run_prompt_command(state: &mut AppState, name: &str) {
    crate::commands::handlers::system::run_prompt(state, name)
}

fn run_thinking_command(state: &mut AppState, level: crate::model::ThinkingLevel) {
    crate::commands::handlers::model::run_thinking(state, level)
}
