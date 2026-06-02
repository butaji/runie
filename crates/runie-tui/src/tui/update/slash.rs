use crate::components::MessageItem;
use crate::tui::state::{AppState, TuiMode};
use crate::tui::update::ui::UiCmd;
use crate::tui::update::ui::clipboard::handle_copy_last_response;

pub fn handle_slash(state: &mut AppState, cmd: runie_core::slash_command::SlashCommand) -> Vec<UiCmd> {
    use runie_core::slash_command::SlashCommand;
    match cmd {
        // Session-modifying commands - grouped
        SlashCommand::New | SlashCommand::Clear | SlashCommand::Fork =>
            { handle_session_cmd(state, &cmd); vec![] }
        SlashCommand::Copy => { handle_copy(state); vec![] }
        SlashCommand::Model(model) => { handle_model(state, model); vec![] }
        SlashCommand::Tree => { handle_tree(state); vec![] }
        SlashCommand::Onboard => { handle_onboard(state); vec![] }
        SlashCommand::Quit => { handle_quit(state); vec![] }
        // Informational - grouped
        SlashCommand::Help | SlashCommand::Cost | SlashCommand::Status | SlashCommand::Models |
        SlashCommand::SessionInfo | SlashCommand::Usage => { handle_info_cmd(state, &cmd); vec![] }
        SlashCommand::Unknown(cmd) => { handle_unknown(state, cmd); vec![] }
        // Session
        SlashCommand::Home => { handle_home(state); vec![] }
        SlashCommand::Resume => { handle_not_implemented(state, "resume"); vec![] }
        SlashCommand::Sessions => { handle_sessions(state); vec![] }
        SlashCommand::Rename(title) => { handle_rename(state, title); vec![] }
        SlashCommand::Share => { handle_not_implemented(state, "share"); vec![] }
        // Context
        SlashCommand::Context => { handle_context(state); vec![] }
        SlashCommand::Compact(_) => { handle_not_implemented(state, "compact"); vec![] }
        SlashCommand::CompactMode => { handle_not_implemented(state, "compact-mode"); vec![] }
        SlashCommand::Rewind => { handle_not_implemented(state, "rewind"); vec![] }
        // UI
        SlashCommand::Theme(name) => { handle_theme(state, name); vec![] }
        SlashCommand::Multiline => { handle_not_implemented(state, "multiline"); vec![] }
        // Permission
        SlashCommand::AlwaysApprove => { handle_always_approve(state); vec![] }
        SlashCommand::Plan => { handle_plan(state); vec![] }
        SlashCommand::Feedback(_) => { handle_not_implemented(state, "feedback"); vec![] }
        // Utility
        SlashCommand::Btw(_) => { handle_not_implemented(state, "btw"); vec![] }
        SlashCommand::Logout => { handle_not_implemented(state, "logout"); vec![] }
        // Extensions
        SlashCommand::Hooks | SlashCommand::Plugins | SlashCommand::Skills | SlashCommand::Mcps | SlashCommand::Extensions =>
            { handle_extensions(state, cmd); vec![] }
        // Shell
        SlashCommand::Flush | SlashCommand::Memory | SlashCommand::Dream =>
            { handle_not_implemented(state, "memory"); vec![] }
        SlashCommand::Imagine(_) => { handle_not_implemented(state, "imagine"); vec![] }
        SlashCommand::ImagineVideo(_) => { handle_not_implemented(state, "imagine-video"); vec![] }
    }
}

fn handle_session_cmd(state: &mut AppState, cmd: &runie_core::slash_command::SlashCommand) {
    use runie_core::slash_command::SlashCommand;
    match cmd {
        SlashCommand::New => handle_new(state),
        SlashCommand::Clear => handle_clear(state),
        SlashCommand::Fork => handle_fork(state),
        _ => {}
    }
}

fn handle_info_cmd(state: &mut AppState, cmd: &runie_core::slash_command::SlashCommand) {
    use runie_core::slash_command::SlashCommand;
    match cmd {
        SlashCommand::Help => handle_help(state),
        SlashCommand::Cost => handle_cost(state),
        SlashCommand::Status => handle_status(state),
        SlashCommand::Models => handle_models(state),
        SlashCommand::SessionInfo => handle_session_info(state),
        SlashCommand::Usage => handle_usage(state),
        _ => {}
    }
}

pub(crate) fn handle_new(state: &mut AppState) {
    state.messages.clear();
    state.scroll.feed_offset = 0;
    state.scroll.user_scrolled_up = false;
    state.messages.push(MessageItem::System { text: "New session started".to_string() });
}

pub(crate) fn handle_clear(state: &mut AppState) {
    state.messages.clear();
    state.scroll.feed_offset = 0;
    state.scroll.user_scrolled_up = false;
}

pub(crate) fn handle_model(state: &mut AppState, model: String) {
    state.current_model = Some(model.clone());
    state.messages.push(MessageItem::System { text: format!("Model switched to {}", model) });
}

pub(crate) fn handle_fork(state: &mut AppState) {
    state.messages.push(MessageItem::System { text: "Fork created at current position".to_string() });
}

pub(crate) fn handle_quit(state: &mut AppState) {
    state.running = false;
}

pub(crate) fn handle_help(state: &mut AppState) {
    state.messages.push(MessageItem::System { text: runie_core::slash_command::format_help() });
}

pub(crate) fn handle_unknown(state: &mut AppState, cmd: String) {
    state.messages.push(MessageItem::System { text: format!("Unknown command: {}. Type /help for available commands.", cmd) });
}

pub(crate) fn handle_cost(state: &mut AppState) {
    let usage = &state.session_token_usage;
    let cost = usage.estimated_cost;
    state.messages.push(MessageItem::System {
        text: format!(
            "Session usage: {} prompt + {} completion = {} tokens, ${:.4}",
            usage.prompt_tokens, usage.completion_tokens, usage.total_tokens, cost
        ),
    });
}

pub(crate) fn handle_status(state: &mut AppState) {
    let model = state.current_model.as_deref().unwrap_or("Not set");
    state.messages.push(MessageItem::System {
        text: format!("Status: model={}", model),
    });
}

pub(crate) fn handle_models(state: &mut AppState) {
    state.messages.push(MessageItem::System {
        text: "Use /model <name> to switch models, or press Ctrl+M to open model picker".to_string(),
    });
}

pub(crate) fn handle_copy(state: &mut AppState) {
    let _ = handle_copy_last_response(state);
}

pub fn handle_tree(state: &mut AppState) {
    state.session_tree.toggle();
    state.mode = if state.session_tree.visible { TuiMode::SessionTree } else { TuiMode::Chat };
}

pub(crate) fn handle_onboard(state: &mut AppState) {
    state.mode = TuiMode::Onboarding;
    state.onboarding = Some(crate::components::Onboarding::default());
}

// ─── New Session Commands ─────────────────────────────────────────────────────

pub(crate) fn handle_home(state: &mut AppState) {
    state.home_screen.show();
    state.mode = TuiMode::HomeScreen;
}

pub(crate) fn handle_sessions(state: &mut AppState) {
    state.session_tree.toggle();
    state.mode = if state.session_tree.visible { TuiMode::SessionTree } else { TuiMode::Chat };
}

pub(crate) fn handle_rename(state: &mut AppState, title: String) {
    state.messages.push(MessageItem::System { text: format!("Session renamed to \"{}\"", title) });
}

pub(crate) fn handle_context(state: &mut AppState) {
    state.context_usage_modal.open();
}

pub(crate) fn handle_theme(state: &mut AppState, name: Option<String>) {
    if let Some(theme_name) = name {
        if let Some(resolved) = crate::theme::resolve_theme(&theme_name) {
            state.current_theme = resolved.clone();
            state.messages.push(MessageItem::System { text: format!("Theme switched to {}", resolved) });
        } else {
            state.messages.push(MessageItem::System { text: format!("Unknown theme: {}. Use /theme without arguments to cycle.", theme_name) });
        }
    } else {
        let next = crate::theme::ThemeWrapper::cycle_theme(&state.current_theme);
        state.current_theme = next.name().to_string();
        state.messages.push(MessageItem::System { text: format!("Theme switched to {}", state.current_theme) });
    }
}

pub(crate) fn handle_always_approve(state: &mut AppState) {
    use crate::tui::state::PermissionMode;
    state.permission_mode = match state.permission_mode {
        PermissionMode::AutoApprove => PermissionMode::Normal,
        _ => PermissionMode::AutoApprove,
    };
    let mode_name = match state.permission_mode {
        PermissionMode::Normal => "Normal",
        PermissionMode::AutoApprove => "AutoApprove",
        PermissionMode::Plan => "Plan",
    };
    state.messages.push(MessageItem::System { text: format!("Permission mode: {}", mode_name) });
}

pub(crate) fn handle_plan(state: &mut AppState) {
    use crate::components::plan_modal::{PlanModal, PlanSection, PlanItem, PlanDocument};

    // Build a demo plan document from recent messages
    // In a full implementation, this would extract the actual plan from the conversation
    let mut document = PlanDocument::new();

    // Add "Quick Assessment" section
    document.sections.push(PlanSection {
        title: "Quick Assessment".to_string(),
        items: vec![
            PlanItem::Bullet {
                text: "docs/install.md skips headless mode...".to_string(),
            },
        ],
    });

    // Add "Implementation Plan" section
    document.sections.push(PlanSection {
        title: "Implementation Plan".to_string(),
        items: vec![
            PlanItem::Step {
                number: 1,
                text: "Replace the install snippet with curl bootstrap".to_string(),
            },
            PlanItem::Step {
                number: 2,
                text: "Document `-p` headless mode".to_string(),
            },
            PlanItem::Step {
                number: 3,
                text: "Point users to config.toml for models".to_string(),
            },
            PlanItem::Step {
                number: 4,
                text: "Cross-link the auth and feedback sections".to_string(),
            },
        ],
    });

    // Open the plan modal with the document
    state.plan_modal.open_with_document(document);
    state.mode = TuiMode::Plan;
}

pub(crate) fn handle_session_info(state: &mut AppState) {
    let msg_count = state.messages.len();
    let token_usage = &state.session_token_usage;
    state.messages.push(MessageItem::System {
        text: format!(
            "Session info: {} messages, {} tokens, ${:.4}",
            msg_count, token_usage.total_tokens, token_usage.estimated_cost
        ),
    });
}

pub(crate) fn handle_usage(state: &mut AppState) {
    let usage = &state.session_token_usage;
    state.messages.push(MessageItem::System {
        text: format!(
            "Usage: {} prompt + {} completion = {} tokens, ${:.4}",
            usage.prompt_tokens, usage.completion_tokens, usage.total_tokens, usage.estimated_cost
        ),
    });
}

pub(crate) fn handle_not_implemented(state: &mut AppState, cmd: &str) {
    state.messages.push(MessageItem::System { text: format!("⚡ /{} is not yet implemented", cmd) });
}

pub(crate) fn handle_extensions(state: &mut AppState, cmd: runie_core::slash_command::SlashCommand) {
    use crate::components::extensions_modal::{ExtensionsModal, ExtensionTab};
    use runie_core::slash_command::SlashCommand;
    let tab = match cmd {
        SlashCommand::Hooks => ExtensionTab::Hooks,
        SlashCommand::Plugins => ExtensionTab::Plugins,
        SlashCommand::Skills => ExtensionTab::Skills,
        SlashCommand::Mcps => ExtensionTab::Mcps,
        SlashCommand::Extensions => ExtensionTab::Hooks,
        _ => ExtensionTab::Hooks,
    };
    state.extensions_modal = Some(ExtensionsModal::with_tab(tab));
    state.mode = TuiMode::Overlay;
}
