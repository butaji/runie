//! Mode command — switch the agent orchestration pattern (`/mode`).
//!
//! See PATTERNS.md: `single` (direct execution), `swarm` (coordinated
//! multi-agent work), and `eval-optimizer` (critical review loops).

use crate::commands::dsl::handlers::NamedHandler;
use crate::commands::{CommandResult, DialogType};
use crate::model::AppState;
use crate::ui_strings::mode as m;

/// Orchestration patterns accepted by `/mode <pattern>`.
const PATTERNS: [&str; 3] = ["single", "swarm", "eval-optimizer"];
/// Execution variants accepted by `/mode swarm <variant>`.
const SWARM_VARIANTS: [&str; 3] = ["parallel", "delegation", "dag"];

/// Register the mode handler with the handler registry (for YAML-based commands).
pub fn register_handlers(registry: &mut crate::commands::dsl::handlers::registry::HandlerRegistry) {
    registry.register("mode", NamedHandler::Handler(handle_mode));
}

pub fn handle_mode(state: &mut AppState, args: &str) -> CommandResult {
    let rest = args.trim();
    if rest.is_empty() {
        return CommandResult::OpenDialog(DialogType::ModeSelector);
    }
    if rest == "list" {
        return CommandResult::Message(m::list(&state.config().mode));
    }
    let parts: Vec<&str> = rest.split_whitespace().collect();
    let active = parts[0];
    if !PATTERNS.contains(&active) {
        return CommandResult::Warning(m::unknown_pattern(active));
    }
    match parts.get(1).copied() {
        None => switch(active, None),
        Some("workers") => parse_workers(active, &parts[2..]),
        Some(variant) if active == "swarm" => set_swarm_variant(state, variant),
        Some(extra) => CommandResult::Warning(m::unknown_arg(active, extra)),
    }
}

/// Emit the pattern-switch event; the model config update handler applies it.
fn switch(active: &str, workers: Option<usize>) -> CommandResult {
    CommandResult::Event(crate::Event::SetMode {
        active: active.to_owned(),
        workers,
    })
}

/// Parse `<pattern> workers <n>`; `n` must be a usize >= 1.
fn parse_workers(active: &str, rest: &[&str]) -> CommandResult {
    match rest
        .first()
        .and_then(|w| w.parse::<usize>().ok())
        .filter(|&w| w >= 1)
    {
        Some(w) => switch(active, Some(w)),
        None => CommandResult::Warning(m::invalid_workers(&rest.join(" "))),
    }
}

/// `/mode swarm <variant> [task...]` — store the variant in session state and
/// switch to swarm. Phase 1 accepts but does not dispatch trailing task text.
fn set_swarm_variant(state: &mut AppState, variant: &str) -> CommandResult {
    if !SWARM_VARIANTS.contains(&variant) {
        return CommandResult::Warning(m::unknown_variant(variant));
    }
    state.config_mut().swarm_variant = Some(variant.to_owned());
    switch("swarm", None)
}
