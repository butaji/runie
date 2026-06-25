//! DSL usage examples — reference implementations showing the declarative pattern.
//!
//! These examples demonstrate how command handlers and control events can be
//! expressed using the declarative DSL. Each example shows the "before" (direct
//! mutation) and "after" (DSL flow) patterns.
//!
//! ## ToggleVimMode
//!
//! ### Before (direct mutation)
//! ```ignore
//! Event::ToggleVimMode => {
//!     state.config.vim_mode = !state.config.vim_mode;
//!     state.view.cached_settings_valid = false;
//!     state.view.dirty = true;
//! }
//! ```
//!
//! ### After (DSL flow)
//! ```ignore
//! use crate::dsl::{on, Intent, Fact};
//!
//! let toggle_vim_flow = on(Event::ToggleVimMode)
//!     .intent(Intent::ToggleVimMode)
//!     .fact(Fact::ViewInvalidated);
//! ```
//!
//! ## Theme switch
//!
//! ### Before (direct mutation)
//! ```ignore
//! fn handle_theme(state: &mut AppState, args: &str) -> CommandResult {
//!     let name = args.trim();
//!     if name.is_empty() {
//!         open_theme_selector(state);
//!         return CommandResult::None;
//!     }
//!     state.config.theme_name = name.to_string();
//!     CommandResult::Message(format!("Theme switched to '{}'", name))
//! }
//! ```
//!
//! ### After (DSL flow)
//! ```ignore
//! use crate::dsl::{on, Intent};
//!
//! fn handle_theme_flow(name: &str) -> Flow<()> {
//!     let name = name.trim();
//!     if name.is_empty() {
//!         return on(()).none(); // caller handles dialog
//!     }
//!     on(()).intent(Intent::SetTheme { name: name.to_string() })
//! }
//! ```
//!
//! ## Session saved notification
//!
//! ### Before (direct mutation)
//! ```ignore
//! Event::SessionSaved { name } => {
//!     state.notify(format!("Session '{}' saved.", name), TransientLevel::Info);
//! }
//! ```
//!
//! ### After (DSL flow)
//! ```ignore
//! use crate::dsl::{on, Fact};
//!
//! let session_saved_flow = on(Event::SessionSaved { name: String::new() })
//!     .notify("Session saved.");
//! ```

#[cfg(test)]
mod tests {
    use crate::dsl::runtime::TestRuntime;
    use crate::dsl::{on, Fact, Intent};
    use crate::event::TransientLevel;

    // ── ToggleVimMode DSL flow example ───────────────────────────────────────

    /// Theme command flow emits SetTheme intent
    #[test]
    fn theme_command_flow_emits_set_theme_intent() {
        let mut rt = TestRuntime::new();
        let name = "dark";
        let flow = on(()).intent(Intent::SetTheme {
            name: name.to_string(),
        });
        flow.run(&mut rt, ());

        assert!(rt.intents().any(|i| matches!(
            i,
            Intent::SetTheme { name } if name == "dark"
        )));
    }

    /// ToggleVimMode flow emits intent and fact
    #[test]
    fn toggle_vim_mode_flow_emits_intent_and_fact() {
        let mut rt = TestRuntime::new();
        let flow = on(())
            .intent(Intent::ToggleVimMode)
            .fact(Fact::ViewInvalidated);
        flow.run(&mut rt, ());

        assert!(rt.intents().any(|i| matches!(i, Intent::ToggleVimMode)));
        assert!(rt.facts().any(|f| matches!(f, Fact::ViewInvalidated)));
    }

    /// SessionSaved flow emits notification
    #[test]
    fn session_saved_flow_emits_notification() {
        let mut rt = TestRuntime::new();
        let flow = on(()).notify_level("Session saved.", TransientLevel::Info);
        flow.run(&mut rt, ());

        assert!(rt
            .notifications()
            .iter()
            .any(|(n, l)| { n == "Session saved." && *l == TransientLevel::Info }));
    }

    /// DSL flow composes with .then()
    #[test]
    fn submit_input_flow_composes_with_then() {
        let mut rt = TestRuntime::new();
        // Simulate: submit → add user message → run turn if queued
        let flow = on(())
            .intent(Intent::Submit)
            .then(on(()).intent(Intent::Abort)); // simplified example
        flow.run(&mut rt, ());

        assert!(rt.intents().any(|i| matches!(i, Intent::Submit)));
        assert!(rt.intents().any(|i| matches!(i, Intent::Abort)));
    }
}
