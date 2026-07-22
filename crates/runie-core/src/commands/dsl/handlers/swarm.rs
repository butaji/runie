//!
//! `/swarm` command handler for swarm mode operations.
//!
//! Grammar:
//!   /swarm cleanup    — clean up orphaned and cancelled workers
//!   /swarm status     — show current swarm status
//!   /swarm reset      — reset the circuit breaker and resume dispatch

use crate::commands::CommandResult;
use crate::event::TransientLevel;
use crate::model::AppState;

/// Handle swarm subcommands.
pub fn handle_swarm(state: &mut AppState, args: &str) -> CommandResult {
    let args = args.trim();
    match args {
        "cleanup" => handle_swarm_cleanup(state),
        "status" => handle_swarm_status(state),
        "reset" => handle_swarm_reset(state),
        "" => handle_swarm_status(state),
        _ => CommandResult::Warning(format!("Unknown swarm command: {}. Use /swarm cleanup, /swarm status, or /swarm reset.", args)),
    }
}

/// Clean up orphaned and cancelled workers.
fn handle_swarm_cleanup(state: &mut AppState) -> CommandResult {
    let counts = state.swarm_cleanup();

    let parts = [
        if counts.orphaned > 0 {
            format!("{} orphaned workers removed", counts.orphaned)
        } else {
            "No orphaned workers".to_string()
        },
        if counts.cancelled > 0 {
            format!("{} cancelled workers removed", counts.cancelled)
        } else {
            "No cancelled workers".to_string()
        },
    ];

    let message = if parts.iter().all(|p| p.starts_with("No ")) {
        "Swarm cleanup complete. No workers to clean.".to_string()
    } else {
        format!("Swarm cleanup complete: {}", parts.join(", "))
    };

    CommandResult::Message(message)
}

/// Show swarm status.
fn handle_swarm_status(state: &AppState) -> CommandResult {
    let counts = state.swarm_status_counts();
    let cb_status = if state.circuit_breaker_tripped {
        format!("\nCircuit Breaker: TRIPPED (threshold: {})", state.circuit_breaker_threshold)
    } else {
        "\nCircuit Breaker: OK".to_string()
    };

    let content = format!(
        "Swarm Workers:\n  Running: {}\n  Completed: {}\n  Failed: {}\n  Cancelled: {}\n  Orphaned: {}{}",
        counts.running,
        counts.completed,
        counts.failed,
        counts.cancelled,
        counts.orphaned,
        cb_status,
    );

    CommandResult::Message(content)
}

/// Reset the circuit breaker and resume dispatch.
fn handle_swarm_reset(state: &mut AppState) -> CommandResult {
    if !state.circuit_breaker_tripped {
        return CommandResult::Message("Circuit breaker is not tripped.".to_string());
    }

    state.circuit_breaker_tripped = false;
    state.circuit_breaker_threshold = 0;
    state.notify("Circuit breaker reset. Dispatch resumed.".to_string(), TransientLevel::Success);

    CommandResult::Message("Circuit breaker reset. Dispatch resumed.".to_string())
}
