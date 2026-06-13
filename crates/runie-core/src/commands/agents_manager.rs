//! Event handler for the agent profile manager UI.

use crate::agent_profiles::{self, AgentProfile};
use crate::commands::handlers::agents;
use crate::commands::DialogState;
use crate::event::Event;
use crate::model::AppState;

/// Handle agents-manager related events.
pub fn agents_manager_event(state: &mut AppState, event: Event) {
    match event {
        Event::OpenAgentsManager => {
            open_root(state);
        }
        Event::AgentsManagerSave { name } => {
            // Reload the profile from disk (the user has presumably
            // edited it through the form). For now we just show a
            // transient message and refresh the panel.
            match agent_profiles::load_profile_from_file(
                &agent_profiles::profiles_dir().join(format!("{}.toml", name)),
            ) {
                Ok(p) => {
                    state.set_transient(
                        format!("Profile '{}' saved", p.name),
                        crate::event::TransientLevel::Info,
                    );
                    // Refresh the panel by re-opening root
                    open_root(state);
                }
                Err(e) => {
                    state.set_transient(
                        format!("Save failed: {}", e),
                        crate::event::TransientLevel::Error,
                    );
                }
            }
        }
        Event::AgentsManagerDelete { name } => {
            match agent_profiles::delete_profile(&name) {
                Ok(_) => {
                    state.set_transient(
                        format!("Deleted profile '{}'", name),
                        crate::event::TransientLevel::Info,
                    );
                    open_root(state);
                }
                Err(e) => {
                    state.set_transient(
                        format!("Delete failed: {}", e),
                        crate::event::TransientLevel::Error,
                    );
                }
            }
        }
        _ => {}
    }
}

fn open_root(state: &mut AppState) {
    let stack = agents::build_root_panel();
    state.open_dialog = Some(DialogState::PanelStack(stack));
    state.mark_dirty();
}
