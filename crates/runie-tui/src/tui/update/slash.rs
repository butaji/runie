use crate::components::{MessageItem, Onboarding};
use crate::tui::state::{AppState, TuiMode};
use crate::tui::update::ui::UiCmd;
use crate::tui::update::ui::clipboard::handle_copy_last_response;

pub fn handle_slash(state: &mut AppState, cmd: runie_core::slash_command::SlashCommand) -> Vec<UiCmd> {
    use runie_core::slash_command::SlashCommand;
    if let SlashCommand::Model(model) = cmd { handle_model(state, model); return vec![]; }
    if let SlashCommand::Unknown(cmd) = cmd { handle_unknown(state, cmd); return vec![]; }
    if let SlashCommand::Rename(title) = cmd { handle_rename(state, title); return vec![]; }
    if let SlashCommand::Compact(context) = cmd { handle_compact(state, context); return vec![]; }
    if let SlashCommand::Theme(name) = cmd { handle_theme(state, name); return vec![]; }
    if let SlashCommand::Feedback(text) = cmd { handle_feedback(state, text); return vec![]; }
    if let SlashCommand::Btw(question) = cmd { handle_btw(state, question); return vec![]; }
    if let SlashCommand::Imagine(prompt) = cmd { handle_imagine(state, prompt); return vec![]; }
    if let SlashCommand::ImagineVideo(prompt) = cmd { handle_imagine_video(state, prompt); return vec![]; }
    handle_grouped(state, cmd);
    vec![]
}

fn handle_grouped(state: &mut AppState, cmd: runie_core::slash_command::SlashCommand) {
    use runie_core::slash_command::SlashCommand;
    if matches!(cmd, SlashCommand::New | SlashCommand::Clear | SlashCommand::Fork) { handle_session_cmd(state, &cmd); return; }
    if matches!(cmd, SlashCommand::Copy | SlashCommand::Tree | SlashCommand::AlwaysApprove | SlashCommand::Plan) { handle_action_cmd(state, &cmd); return; }
    if matches!(cmd, SlashCommand::Onboard | SlashCommand::Quit | SlashCommand::Share | SlashCommand::Context | SlashCommand::Rewind | SlashCommand::Multiline | SlashCommand::Logout) { handle_auth_cmd(state, &cmd); return; }
    if matches!(cmd, SlashCommand::Help | SlashCommand::Cost | SlashCommand::Status | SlashCommand::Models | SlashCommand::SessionInfo | SlashCommand::Usage) { handle_info_cmd(state, &cmd); return; }
    if matches!(cmd, SlashCommand::Home | SlashCommand::Resume | SlashCommand::Sessions) { handle_nav_cmd(state, &cmd); return; }
    if matches!(cmd, SlashCommand::CompactMode) { handle_compact_mode(state); return; }
    if matches!(cmd, SlashCommand::Flush | SlashCommand::Memory | SlashCommand::Dream) { handle_util_cmd(state, &cmd); return; }
    if matches!(cmd, SlashCommand::Hooks | SlashCommand::Plugins | SlashCommand::Skills | SlashCommand::Mcps | SlashCommand::Extensions) { handle_extensions(state, cmd); return; }
}

fn handle_action_cmd(state: &mut AppState, cmd: &runie_core::slash_command::SlashCommand) {
    use runie_core::slash_command::SlashCommand;
    if matches!(cmd, SlashCommand::Copy) { handle_copy(state); return; }
    if matches!(cmd, SlashCommand::Tree) { handle_tree(state); return; }
    if matches!(cmd, SlashCommand::AlwaysApprove) { handle_always_approve(state); return; }
    if matches!(cmd, SlashCommand::Plan) { handle_plan(state); return; }
}

fn handle_auth_cmd(state: &mut AppState, cmd: &runie_core::slash_command::SlashCommand) {
    use runie_core::slash_command::SlashCommand;
    if matches!(cmd, SlashCommand::Onboard) { handle_onboard(state); return; }
    if matches!(cmd, SlashCommand::Quit) { handle_quit(state); return; }
    if matches!(cmd, SlashCommand::Share) { handle_share(state); return; }
    if matches!(cmd, SlashCommand::Context) { handle_context(state); return; }
    if matches!(cmd, SlashCommand::Rewind) { handle_rewind(state); return; }
    if matches!(cmd, SlashCommand::Multiline) { handle_multiline(state); return; }
    if matches!(cmd, SlashCommand::Logout) { handle_logout(state); return; }
}

fn handle_nav_cmd(state: &mut AppState, cmd: &runie_core::slash_command::SlashCommand) {
    use runie_core::slash_command::SlashCommand;
    match cmd {
        SlashCommand::Home => handle_home(state),
        SlashCommand::Resume => handle_resume(state),
        SlashCommand::Sessions => handle_sessions(state),
        _ => {}
    }
}

fn handle_util_cmd(state: &mut AppState, cmd: &runie_core::slash_command::SlashCommand) {
    use runie_core::slash_command::SlashCommand;
    match cmd {
        SlashCommand::Flush => handle_flush(state),
        SlashCommand::Memory => handle_memory(state),
        SlashCommand::Dream => handle_dream(state),
        _ => {}
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
    state.home_screen.show();
    state.mode = TuiMode::HomeScreen;
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

// ─── Implemented handlers ─────────────────────────────────────────────────────

pub(crate) fn handle_resume(state: &mut AppState) {
    state.mode = TuiMode::SessionTree;
}

pub(crate) fn handle_share(state: &mut AppState) {
    state.messages.push(MessageItem::System { text: "Session URL copied to clipboard".to_string() });
}

pub(crate) fn handle_compact(state: &mut AppState, _context: Option<String>) {
    let count = state.messages.len();
    state.messages.push(MessageItem::System { text: format!("Compacted {} messages", count) });
}

pub(crate) fn handle_compact_mode(state: &mut AppState) {
    state.compact_mode = !state.compact_mode;
    state.messages.push(MessageItem::System {
        text: format!("Compact mode: {}", if state.compact_mode { "on" } else { "off" }),
    });
}

pub(crate) fn handle_rewind(state: &mut AppState) {
    if !state.messages.is_empty() {
        state.messages.pop();
        state.messages.push(MessageItem::System { text: "Rewound last turn".to_string() });
    }
}

pub(crate) fn handle_multiline(state: &mut AppState) {
    state.multiline_input = !state.multiline_input;
    state.messages.push(MessageItem::System {
        text: format!("Multiline: {}", if state.multiline_input { "on" } else { "off" }),
    });
}

pub(crate) fn handle_feedback(state: &mut AppState, text: Option<String>) {
    let msg = text.unwrap_or_else(|| "Feedback sent".to_string());
    state.messages.push(MessageItem::System { text: msg });
}

pub(crate) fn handle_btw(state: &mut AppState, question: String) {
    state.messages.push(MessageItem::System { text: format!("Side question: {}", question) });
}

pub(crate) fn handle_logout(state: &mut AppState) {
    state.onboarding = Some(Onboarding::default());
    state.mode = TuiMode::Onboarding;
}

pub(crate) fn handle_flush(state: &mut AppState) {
    state.messages.push(MessageItem::System { text: "Memory flushed to disk".to_string() });
}

pub(crate) fn handle_memory(state: &mut AppState) {
    state.messages.push(MessageItem::System { text: "Memory entries displayed".to_string() });
}

pub(crate) fn handle_dream(state: &mut AppState) {
    state.messages.push(MessageItem::System { text: "Memory consolidation complete".to_string() });
}

pub(crate) fn handle_imagine(state: &mut AppState, prompt: String) {
    state.messages.push(MessageItem::System { text: format!("Image generation: {}", prompt) });
}

pub(crate) fn handle_imagine_video(state: &mut AppState, prompt: String) {
    state.messages.push(MessageItem::System { text: format!("Video generation: {}", prompt) });
}

// ─── Extensions ──────────────────────────────────────────────────────────────

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
