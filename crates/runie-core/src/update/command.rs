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
        // Session IO (save/load/delete/import/export)
        crate::Event::RunLoadCommand { name } => session_dispatch(state, |s| session_run::run_load(s, name.trim())),
        crate::Event::RunSaveCommand { name } => session_dispatch(state, |s| session_run::run_save(s, name.trim())),
        crate::Event::RunDeleteCommand { name } => session_dispatch(state, |s| session_run::run_delete(s, name.trim())),
        crate::Event::RunImportCommand { path } => session_dispatch(state, |s| session_run::run_import(s, path.trim())),
        crate::Event::RunExportCommand { path } => session_dispatch(state, |s| session_run::run_export(s, path.trim())),

        // Session state commands
        crate::Event::RunNameCommand { name } => run_name_command(state, name),
        crate::Event::RunForkCommand { message_index } => run_fork_command(state, message_index),
        crate::Event::RunCompactCommand { keep, focus } => run_compact_command(state, keep, focus),
        crate::Event::RunPromptCommand { name } => run_prompt_command(state, name),

        // Login/logout
        crate::Event::RunLoginCommand { .. } => dispatch_result(
            state,
            CommandResult::OpenDialog(crate::commands::DialogType::CommandPalette),
        ),
        crate::Event::RunLogoutCommand { provider } => run_logout_command(state, provider),
        // Skill
        crate::Event::RunSkillCommand { name } => run_skill_command(state, name),
        // System
        crate::Event::RunThinkingCommand { level } => {
            let result = crate::commands::dsl::handlers::model::run_thinking(state, *level);
            dispatch_result(state, result);
        }
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

fn run_name_command(state: &mut AppState, name: &str) {
    let name = name.trim();
    if name.is_empty() {
        let current = state
            .session()
            .session_display_name
            .as_deref()
            .unwrap_or("(unset)");
        state.add_system_msg(format!("Current display name: {}", current));
    } else {
        let truncated = if name.chars().count() > 64 {
            format!("{}…", name.chars().take(64).collect::<String>())
        } else {
            name.to_owned()
        };
        state.session_mut().session_display_name = Some(truncated.clone());
        state.add_system_msg(format!("Session name set to '{}'", truncated));
    }
}

fn run_fork_command(state: &mut AppState, message_index_raw: &str) {
    let message_index = if message_index_raw.trim().is_empty() {
        session_run::fallback_fork_index(state)
    } else {
        match message_index_raw.trim().parse::<usize>() {
            Ok(n) => n,
            Err(_) => {
                state.add_system_msg(format!(
                    "Invalid message index '{}': expected a non-negative integer.",
                    message_index_raw
                ));
                return;
            }
        }
    };
    let msg_count = state.session().messages.len();
    if message_index >= msg_count {
        state.add_system_msg(format!(
            "Index {} out of range (0–{})",
            message_index,
            msg_count.saturating_sub(1)
        ));
        return;
    }
    let mut tree = state
        .session_mut()
        .session_tree
        .take()
        .unwrap_or_else(|| crate::session::tree::SessionTree::from_messages(&state.session().messages));
    match tree.fork_at(message_index) {
        Some(path) => {
            tree.navigate_to(&path);
            state.session_mut().session_tree = Some(tree);
            state.add_system_msg(format!("Forked at message {}.", message_index));
        }
        None => state.add_system_msg("Could not fork.".into()),
    }
}

fn run_compact_command(state: &mut AppState, keep_raw: &str, focus: &str) {
    let keep = match keep_raw.parse::<usize>() {
        Ok(n) if n > 0 => n,
        _ => {
            state.add_system_msg(format!(
                "Invalid keep value '{}': expected a positive integer.",
                keep_raw
            ));
            return;
        }
    };
    let msg = state.compact(keep);
    let result = if focus.is_empty() {
        msg
    } else {
        format!("{} (focus: {})", msg, focus)
    };
    state.add_system_msg(result);
}

fn run_prompt_command(state: &mut AppState, name: &str) {
    let name = name.trim();
    if name.is_empty() {
        let current = if state.input().current_prompt.is_empty() {
            "default"
        } else {
            &state.input().current_prompt
        };
        let mut lines = vec![format!("Current prompt: {}", current)];
        if !state.prompts().is_empty() {
            lines.push("Available prompts:".into());
            for p in state.prompts() {
                lines.push(format!("  {}", p.summary()));
            }
        }
        state.add_system_msg(lines.join("\n"));
        return;
    }
    if state.prompts().iter().any(|p| p.name == name) {
        state.update(crate::Event::SetPrompt { name: name.to_owned() });
        state.add_system_msg(format!("Prompt switched to '{}'", name));
    } else {
        state.add_system_msg(format!("Prompt '{}' not found.", name));
    }
}

fn run_skill_command(state: &mut AppState, name: &str) {
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
    dispatch_result(state, result);
}

fn run_logout_command(state: &mut AppState, provider: &str) {
    if provider.is_empty() {
        dispatch_result(
            state,
            CommandResult::OpenDialog(crate::commands::DialogType::CommandPalette),
        );
        return;
    }
    state.remove_provider(provider);
    if state.config().current_provider == provider {
        let (provider, model) = state.resolve_default_model();
        state.set_active_model(provider, model, crate::model::ModelSource::ConfigDefault);
    }
    if !state.has_models() {
        crate::login_flow::login_flow_start(state);
    }
    dispatch_result(
        state,
        CommandResult::Message(format!(
            "Disconnected '{}'. Use /providers to manage providers.",
            provider
        )),
    );
}

fn run_palette_command(state: &mut AppState, name: &str, args: &str) {
    let result = if let Some(cmd) = state.registry().get(name) {
        let cmd_name = cmd.name.clone();
        cmd.flow().clone().exec(state, &cmd_name, args)
    } else {
        CommandResult::Message(format!("Unknown command: /{}", name))
    };
    dispatch_result(state, result);
}
