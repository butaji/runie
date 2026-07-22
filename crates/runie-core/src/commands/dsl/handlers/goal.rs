//! `/goal` command handler.
//!
//! Grammar:
//!   /goal [--replace] -- <objective>   — create or update goal
//!   /goal status                       — show current goal status
//!   /goal pause                        — pause active goal
//!   /goal resume                       — resume paused goal
//!   /goal cancel                       — cancel active goal

use crate::commands::CommandResult;
use crate::dialog::{ItemAction, Panel, PanelStack};
use crate::model::{AppState, GoalState, GoalStatus};

/// Parse goal arguments and dispatch to the appropriate handler.
pub fn handle_goal(state: &mut AppState, args: &str) -> CommandResult {
    let args = args.trim();
    if args.is_empty() || args == "status" {
        return handle_goal_status(state);
    }
    if args == "pause" {
        return handle_goal_pause(state);
    }
    if args == "resume" {
        return handle_goal_resume(state);
    }
    if args == "cancel" {
        return handle_goal_cancel(state);
    }
    // Parse create/update: [--replace] [--] <objective>
    let mut input = args;
    let replace = input.starts_with("--replace ");
    if replace {
        input = input
            .trim_start_matches("--replace ")
            .trim_start_matches("--")
            .trim();
    } else if input.starts_with("-- ") {
        input = input.trim_start_matches("-- ").trim();
    } else if input == "--" {
        // `/goal --` (no trailing space) → empty objective
        input = "";
    }
    handle_goal_create(state, input, replace)
}

/// Show the current goal status in a panel stack.
fn handle_goal_status(state: &AppState) -> CommandResult {
    let Some(goal) = state.goal_state() else {
        return CommandResult::Message("No active goal. Use /goal -- <objective> to create one.".into());
    };

    let elapsed = goal.created_at.elapsed().as_secs();
    let elapsed_str = format_duration(elapsed);

    let status_icon = match goal.status {
        GoalStatus::Active => "🟢",
        GoalStatus::Paused => "⏸",
        GoalStatus::Completed => "✅",
        GoalStatus::Cancelled => "❌",
    };

    let content = format!(
        "Goal: {}\nStatus: {} {}\nElapsed: {}\nCompletion criterion: {}",
        goal.objective,
        status_icon,
        goal.status.label(),
        elapsed_str,
        goal.completion_criterion
            .as_deref()
            .unwrap_or("(not specified)")
    );

    let panel = Panel::new("goal_status", " Goal Status ")
        .header(&content)
        .item("⏸ Pause", ItemAction::Emit(crate::Event::GoalPause))
        .item("▶ Resume", ItemAction::Emit(crate::Event::GoalResume))
        .item("❌ Cancel", ItemAction::Emit(crate::Event::GoalCancel))
        .item("✕ Close", ItemAction::Close);

    CommandResult::OpenPanelStack(Box::new(PanelStack::new(panel)))
}

/// Pause the active goal.
fn handle_goal_pause(state: &mut AppState) -> CommandResult {
    match state.goal_state_mut() {
        Some(goal) if goal.status == GoalStatus::Active => {
            goal.status = GoalStatus::Paused;
            CommandResult::Message("Goal paused. Use /goal resume to continue.".into())
        }
        Some(_) => CommandResult::Warning("Goal is not active.".into()),
        None => CommandResult::Warning("No active goal to pause.".into()),
    }
}

/// Resume a paused goal.
fn handle_goal_resume(state: &mut AppState) -> CommandResult {
    match state.goal_state_mut() {
        Some(goal) if goal.status == GoalStatus::Paused => {
            goal.status = GoalStatus::Active;
            CommandResult::Message("Goal resumed.".into())
        }
        Some(_) => CommandResult::Warning("Goal is not paused.".into()),
        None => CommandResult::Warning("No goal to resume.".into()),
    }
}

/// Cancel the active goal.
fn handle_goal_cancel(state: &mut AppState) -> CommandResult {
    match state.goal_state() {
        Some(goal) => {
            let objective = goal.objective.clone();
            *state.goal_state_mut() = None;
            state.add_system_msg(format!("Goal cancelled: {}", objective));
            CommandResult::None
        }
        None => CommandResult::Warning("No active goal to cancel.".into()),
    }
}

/// Create or update a goal.
fn handle_goal_create(state: &mut AppState, objective: &str, replace: bool) -> CommandResult {
    if objective.trim().is_empty() {
        return CommandResult::Warning("Goal objective cannot be empty. Use /goal -- <objective>.".into());
    }

    let goal = if replace {
        // Replace existing goal
        if let Some(existing) = state.goal_state() {
            GoalState::new(objective.to_string(), existing.completion_criterion.clone())
        } else {
            GoalState::new(objective.to_string(), None)
        }
    } else {
        // Create new goal (replacing any existing)
        GoalState::new(objective.to_string(), None)
    };

    let was_active = state.goal_state().is_some();
    *state.goal_state_mut() = Some(goal);

    if was_active {
        state.add_system_msg("Goal updated.".to_string());
    } else {
        state.add_system_msg("Goal set. Working toward your objective.".to_string());
    }

    // Emit event so actor layer knows about goal creation
    CommandResult::Event(crate::Event::GoalCreate { objective: objective.to_string() })
}

fn format_duration(secs: u64) -> String {
    let hours = secs / 3600;
    let mins = (secs % 3600) / 60;
    let secs = secs % 60;
    if hours > 0 {
        format!("{}h {}m {}s", hours, mins, secs)
    } else if mins > 0 {
        format!("{}m {}s", mins, secs)
    } else {
        format!("{}s", secs)
    }
}
