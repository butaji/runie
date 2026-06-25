//! Command event dispatcher.
//!
//! Routes `RunXCommand` events to canonical handlers.  This is the single
//! dispatch point for slash commands triggered via events (as opposed to
//! `/name args` in the input bar which goes through `AppState::handle_slash`).

use crate::commands::dsl::handlers::session::run as session_run;
use crate::model::AppState;

use super::dialog;
use crate::commands::CommandResult;

pub(super) fn handle_command_event(state: &mut AppState, event: crate::Event) {
    match &event {
        // Session IO (save/load/delete/import/export) and state (name/fork)
        // use a helper that calls the handler and dispatches the result.
        crate::Event::RunLoadCommand { name } => session_dispatch(state, |s| session_run::run_load(s, name.trim())),
        crate::Event::RunSaveCommand { name } => session_dispatch(state, |s| session_run::run_save(s, name.trim())),
        crate::Event::RunDeleteCommand { name } => session_dispatch(state, |s| session_run::run_delete(s, name.trim())),
        crate::Event::RunImportCommand { path } => session_dispatch(state, |s| session_run::run_import(s, path.trim())),
        crate::Event::RunExportCommand { path } => session_dispatch(state, |s| session_run::run_export(s, path.trim())),
        crate::Event::RunNameCommand { name } => session_dispatch(state, |s| session_run::run_name(s, name.trim())),
        crate::Event::RunForkCommand { message_index } => session_dispatch(state, |s| session_run::run_fork(s, message_index.trim())),
        crate::Event::RunCompactCommand { keep, focus } => run_compact(state, keep, focus),
        // Login/logout
        crate::Event::RunLoginCommand { .. } => dispatch_result(state, CommandResult::OpenDialog(crate::commands::DialogType::CommandPalette)),
        crate::Event::RunLogoutCommand { provider } => run_logout_command(state, provider),
        // Skill
        crate::Event::RunSkillCommand { name } => run_skill_command(state, name),
        // System
        crate::Event::RunPromptCommand { name } => crate::commands::dsl::handlers::system::run_prompt(state, name),
        crate::Event::RunThinkingCommand { level } => crate::commands::dsl::handlers::model::run_thinking(state, *level),
        // Registry dispatch (from command palette)
        crate::Event::RunPaletteCommand { name, args } => run_palette_command(state, name, args),
        // intentionally ignored: other command events fall through
        _ => {}
    }
}

/// Call a session handler and dispatch its result.
fn session_dispatch<F>(state: &mut AppState, f: F)
where
    F: FnOnce(&mut AppState) -> CommandResult,
{
    let result = f(state);
    dispatch_result(state, result);
}

fn dispatch_result(state: &mut AppState, result: CommandResult) {
    dialog::process_command_result(state, result);
}

fn run_compact(state: &mut AppState, keep: &str, focus: &str) {
    let args = if focus.is_empty() { keep.to_owned() } else { format!("{keep} {focus}") };
    let result = session_run::run_compact(state, &args);
    dispatch_result(state, result);
}

fn run_skill_command(state: &mut AppState, name: &str) {
    let result = state.skills.iter()
        .find(|s| s.name == name)
        .map(|skill| {
            let mut lines = vec![format!("Skill: {}", skill.name)];
            if !skill.description.is_empty() { lines.push(format!("Description: {}", skill.description)); }
            if !skill.context.is_empty() { lines.push(format!("Context: {}", skill.context)); }
            CommandResult::Message(lines.join("\n"))
        })
        .unwrap_or_else(|| CommandResult::Message(format!("Skill '{}' not found. Use /skills to list loaded skills.", name)));
    dispatch_result(state, result);
}

fn run_logout_command(state: &mut AppState, provider: &str) {
    if provider.is_empty() {
        dispatch_result(state, CommandResult::OpenDialog(crate::commands::DialogType::CommandPalette));
        return;
    }
    state.remove_provider(provider);
    if state.config().current_provider == provider {
        let (provider, model) = state.resolve_default_model();
        state.set_active_model(provider, model, crate::model::ModelSource::ConfigDefault);
    }
    if !state.has_models() { crate::login_flow::login_flow_start(state); }
    dispatch_result(state, CommandResult::Message(format!("Disconnected '{}'. Use /providers to manage providers.", provider)));
}

fn run_palette_command(state: &mut AppState, name: &str, args: &str) {
    let result = if let Some(cmd) = state.registry().get(name) {
        let cmd_name = cmd.name.clone();
        cmd.flow.clone().exec(state, &cmd_name, args)
    } else {
        CommandResult::Message(format!("Unknown command: /{}", name))
    };
    dispatch_result(state, result);
}
