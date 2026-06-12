pub mod agent;
pub mod chat;
pub mod misc;
pub mod onboarding;
pub mod palette;
pub mod slash;
pub mod system;
pub mod tree;
pub mod ui;

#[cfg(test)]
mod palette_test;
#[cfg(test)]
mod palette_tests;
#[cfg(test)]
mod slash_tests;

use crate::components::CommandPalette;
use crate::tui::state::{AppState, Msg, Cmd};

/// Main update dispatcher - routes messages to domain-specific update functions.
/// Each domain handles its own state mutations and returns commands.
pub fn update(state: &mut AppState, palette: &mut CommandPalette, msg: Msg) -> Vec<Cmd> {
    let mut cmds = Vec::new();

    // Chat domain: messages, textarea, scroll
    let chat_cmds = chat::update(state, msg.clone());
    cmds.extend(chat_cmds.into_iter().map(Cmd::from));

    // Agent domain: agent events, permissions
    let agent_cmds = agent::update(state, msg.clone());
    cmds.extend(agent_cmds.into_iter().map(Cmd::from));

    // UI domain: mode, overlays, command palette, model picker
    let ui_cmds = ui::update(state, palette, msg.clone());
    cmds.extend(ui_cmds.into_iter().map(Cmd::from));

    // System domain: quit/stop, tick, permission timeout
    let system_cmds = system::update(state, palette, msg.clone());
    cmds.extend(system_cmds);

    // Onboarding domain: onboarding flow
    let onboarding_cmds = onboarding::handle_onboarding_msg(state, msg);
    cmds.extend(onboarding_cmds);

    cmds
}
