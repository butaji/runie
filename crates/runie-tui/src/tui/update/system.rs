//! System domain update functions.
//! Handles: app lifecycle (quit/stop), tick/animation, permission timeout,
//! startup state initialization, context percentage.

use crate::tui::state::{AppState, TuiMode, Msg, Cmd};
use crate::components::CommandPalette;

/// System-specific commands returned by update functions.
#[derive(Debug, Clone)]
pub enum SystemCmd {
    Interrupt,
}

impl From<SystemCmd> for Cmd {
    fn from(cmd: SystemCmd) -> Self {
        match cmd {
            SystemCmd::Interrupt => Cmd::Interrupt,
        }
    }
}

/// Update system domain: quit/stop, tick, permission timeout, startup state.
pub fn update(state: &mut AppState, palette: &mut CommandPalette, msg: Msg) -> Vec<Cmd> {
    match msg {
        Msg::Quit | Msg::Stop => handle_quit_or_stop(state, &msg),
        Msg::Tick | Msg::CursorBlink => { handle_anim(state, &msg); handle_tick_permission_check(state, palette) }
        Msg::Resize(w, h) => { state.terminal_size = (w, h); vec![] }
        _ => vec![],
    }
}

// ─── Quit/Stop ─────────────────────────────────────────────────────────────────

fn handle_quit_or_stop(state: &mut AppState, msg: &Msg) -> Vec<Cmd> {
    state.agent_running = false;
    // P0-AGENT-TIMEOUT: Clear agent start time on quit/stop/interrupt
    state.agent_start_time = None;
    if matches!(msg, Msg::Quit) {
        state.running = false;
    }
    if state.mode != TuiMode::Onboarding {
        state.mode = TuiMode::Chat;
    }
    if matches!(msg, Msg::Stop) { vec![Cmd::Interrupt] } else { vec![] }
}

// ─── Animation ─────────────────────────────────────────────────────────────────

fn handle_anim(state: &mut AppState, msg: &Msg) {
    match msg {
        Msg::Tick => {
            state.animation.braille_frame = (state.animation.braille_frame + 1) % 10;
            state.animation.rewind_braille_frame = (state.animation.rewind_braille_frame + 1) % 10;
            // Tick matrix rain animation during onboarding
            if let Some(ref mut onboarding) = state.onboarding {
                if onboarding.matrix_rain.is_none() {
                    onboarding.matrix_rain = Some(crate::components::onboarding::MatrixRain::new(80, 24));
                }
                if let Some(ref mut rain) = onboarding.matrix_rain {
                    rain.tick();
                }
            }
        }
        Msg::CursorBlink => {
            state.animation.streaming_cursor_visible = !state.animation.streaming_cursor_visible;
        }
        _ => {}
    }
}

// ─── Permission Timeout ───────────────────────────────────────────────────────

fn handle_tick_permission_check(state: &mut AppState, _palette: &mut CommandPalette) -> Vec<Cmd> {
    if let Some(cmd) = check_agent_timeout(state) {
        return cmd;
    }
    check_permission_timeout(state)
}

fn check_agent_timeout(state: &mut AppState) -> Option<Vec<Cmd>> {
    if !state.agent_running { return None; }
    if let Some(start_time) = state.agent_start_time {
        const AGENT_TIMEOUT_SECS: u64 = 600;
        if start_time.elapsed().as_secs() >= AGENT_TIMEOUT_SECS {
            state.agent_running = false;
            state.agent_start_time = None;
            state.messages.push(crate::components::MessageItem::System {
                text: "Agent timed out after 10 minutes.".to_string(),
            });
            state.permission_modal.pending_queue.clear();
            if state.mode != TuiMode::Onboarding {
                state.mode = TuiMode::Chat;
            }
            return Some(vec![Cmd::Interrupt]);
        }
    }
    None
}

fn check_permission_timeout(state: &mut AppState) -> Vec<Cmd> {
    if state.mode != TuiMode::Permission { return vec![]; }
    if state.permission_modal.timed_out { return vec![]; }

    if let Some(start) = state.permission_modal.timeout_start {
        const TIMEOUT_SECS: u64 = 300;
        if start.elapsed().as_secs() >= TIMEOUT_SECS {
            state.permission_modal.timed_out = true;
            state.messages.push(crate::components::MessageItem::System {
                text: "Permission request timed out after 5 minutes.".to_string(),
            });

            let tool_call_id = state.permission_modal.tool_call_id.clone().unwrap_or_default();
            state.permission_modal.tool = None;
            state.permission_modal.tool_call_id = None;

            if !state.permission_modal.pending_queue.is_empty() {
                let pending = state.permission_modal.pending_queue.remove(0);
                state.permission_modal.tool = Some(pending.tool_name.clone());
                state.permission_modal.tool_call_id = Some(pending.tool_call_id.clone());
                state.permission_modal.args = Some(pending.tool_args.clone());
                state.permission_modal.desc = Some(format!("Agent wants to execute '{}'", pending.tool_name));
                state.permission_modal.timeout_start = Some(std::time::Instant::now());
                state.permission_modal.timed_out = false;
                state.mode = TuiMode::Permission;
            } else {
                state.mode = TuiMode::Chat;
            }

            return vec![Cmd::SendPermission { decision: runie_agent::PermissionDecision::Deny { tool_call_id, tool_name: String::new(), tool_args: String::new() } }];
        }
    }

    vec![]
}
