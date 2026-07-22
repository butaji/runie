//! Status command — opens a read-only panel summarizing the current session.
//!
//! The panel is informational only: every line is a non-selectable header and
//! the panel is closable, so Esc/Enter returns to the chat via the generic
//! panel-close path. It reads live `AppState` at open time; no live refresh.

use crate::commands::dsl::handlers::registry::HandlerRegistry;
use crate::commands::dsl::handlers::NamedHandler;
use crate::commands::CommandResult;
use crate::dialog::{Panel, PanelStack};
use crate::model::AppState;

/// Register the status handler with the handler registry.
pub fn register_handlers(registry: &mut HandlerRegistry) {
    registry.register("status", NamedHandler::Handler(handle_status));
}

pub fn handle_status(state: &mut AppState, _: &str) -> CommandResult {
    CommandResult::OpenPanelStack(Box::new(PanelStack::new(build_status_panel(state))))
}

/// Format a token count for compact display (e.g. 128_000 -> "128k").
fn format_tokens_short(n: usize) -> String {
    if n >= 1_000_000 {
        format!("{}M", n / 1_000_000)
    } else if n >= 1_000 {
        format!("{}k", n / 1_000)
    } else {
        n.to_string()
    }
}

fn build_status_panel(state: &AppState) -> Panel {
    let provider = state.current_provider();
    let model = state.current_model();
    let thinking = state.thinking_level().as_str();
    let access = if state.read_only() {
        "read-only"
    } else {
        "read-write"
    };
    let queued = state.agent_state().message_queue.len();
    let ctx = if provider.is_empty() || model.is_empty() {
        "n/a".to_string()
    } else {
        let window = crate::model_catalog::context_window_for(provider, model);
        let used = state.agent_state().tokens_in.min(window);
        let pct = (used as f64 / window as f64 * 100.0).round() as u64;
        format!(
            "~{pct}% ({} / {} tokens)",
            format_tokens_short(used),
            format_tokens_short(window)
        )
    };

    let lead_model = state
        .lead_model()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "(not set)".to_string());
    let worker_model = state
        .worker_model()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "(not set)".to_string());

    Panel::new("status", " Status ")
        .header(format!("Provider:   {provider}"))
        .header(format!("Model:     {model}"))
        .header(format!("Thinking:  {thinking}"))
        .header(format!("Access:    {access}"))
        .header(format!("Queued:    {queued}"))
        .header(format!("Context:   {ctx}"))
        .header(format!("Lead:      {lead_model}"))
        .header(format!("Worker:    {worker_model}"))
}
