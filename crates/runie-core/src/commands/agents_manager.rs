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
        Event::AgentsManagerSetField { name, field, value } => {
            handle_set_field(state, &name, &field, value);
        }
        Event::AgentsManagerSave { name } => {
            handle_save(state, &name);
        }
        Event::AgentsManagerDelete { name } => {
            handle_delete(state, &name);
        }
        _ => {}
    }
}

fn handle_set_field(state: &mut AppState, name: &str, field: &str, value: String) {
    let profile = state.pending_agent_edit.get_or_insert_with(|| {
        agent_profiles::load_profile_from_file(
            &agent_profiles::profiles_dir().join(format!("{}.toml", name)),
        )
        .unwrap_or_else(|_| AgentProfile::new(name, ""))
    });
    profile.name = name.to_string();
    match field {
        "system_prompt" => profile.system_prompt = value,
        "description" => profile.description = value,
        _ => {}
    }
    state.mark_dirty();
}

fn handle_save(state: &mut AppState, name: &str) {
    let profile = match &state.pending_agent_edit {
        Some(p) if p.name == name => p.clone(),
        _ => {
            state.set_transient(
                "No pending edits to save".to_string(),
                crate::event::TransientLevel::Info,
            );
            return;
        }
    };
    if profile.name.trim().is_empty() {
        state.set_transient(
            "Profile name cannot be empty".to_string(),
            crate::event::TransientLevel::Error,
        );
        return;
    }
    match agent_profiles::save_profile(&profile) {
        Ok(_) => {
            state.pending_agent_edit = None;
            state.set_transient(
                format!("Profile '{}' saved", profile.name),
                crate::event::TransientLevel::Info,
            );
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

fn handle_delete(state: &mut AppState, name: &str) {
    match agent_profiles::delete_profile(name) {
        Ok(_) => {
            state.pending_agent_edit = None;
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

fn open_root(state: &mut AppState) {
    let stack = agents::build_root_panel();
    state.open_dialog = Some(DialogState::PanelStack(stack));
    state.mark_dirty();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppState;

    #[test]
    fn set_field_updates_pending_edit() {
        let mut state = AppState::default();
        agents_manager_event(
            &mut state,
            Event::AgentsManagerSetField {
                name: "test".into(),
                field: "system_prompt".into(),
                value: "You are helpful.".into(),
            },
        );
        let edit = state.pending_agent_edit.as_ref().expect("pending edit");
        assert_eq!(edit.name, "test");
        assert_eq!(edit.system_prompt, "You are helpful.");
    }

    #[test]
    fn save_empty_name_fails() {
        let mut state = AppState::default();
        state.pending_agent_edit = Some(AgentProfile {
            name: "".into(),
            description: "".into(),
            system_prompt: "prompt".into(),
            tools: vec![],
            max_turns: None,
            allowlist_tools: None,
            denylist_tools: None,
        });
        agents_manager_event(&mut state, Event::AgentsManagerSave { name: "".into() });
        assert!(state.transient_level == Some(crate::event::TransientLevel::Error));
    }

    #[test]
    fn save_persists_pending_profile() {
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("HOME", dir.path());
        let mut state = AppState::default();
        state.pending_agent_edit = Some(AgentProfile {
            name: "persist_test".into(),
            description: "desc".into(),
            system_prompt: "prompt".into(),
            tools: vec!["read".into()],
            max_turns: None,
            allowlist_tools: None,
            denylist_tools: None,
        });
        agents_manager_event(&mut state, Event::AgentsManagerSave { name: "persist_test".into() });
        assert!(state.pending_agent_edit.is_none());
        let loaded = agent_profiles::load_profile_from_file(
            &agent_profiles::profiles_dir().join("persist_test.toml"),
        ).expect("profile saved");
        assert_eq!(loaded.system_prompt, "prompt");
    }
}
