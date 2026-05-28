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
        }
        Msg::CursorBlink => {
            state.animation.streaming_cursor_visible = !state.animation.streaming_cursor_visible;
        }
        _ => {}
    }
}

// ─── Permission Timeout ───────────────────────────────────────────────────────

fn handle_tick_permission_check(state: &mut AppState, _palette: &mut CommandPalette) -> Vec<Cmd> {
    // Only check if we're in permission mode with timeout tracking active
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

            // Process next pending permission if any (FIFO)
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
