//! Session replay — convert between `AppState` and durable events.
//!
//! This module is the single persistence path for `/save` and `/load`.
//! Import/export still uses the `Session` snapshot DTO in `crate::session`,
//! but runtime persistence goes exclusively through `SessionStore`.

use crate::event::DurableCoreEvent;
use crate::message::ChatMessage;
use crate::model::{AppState, Role};
use crate::session::SessionMetadata;
use crate::session::store::SessionStore;
use crate::Event;

/// Convert a durable event to a bus event for replay.
pub fn durable_to_event(event: &DurableCoreEvent) -> Option<Event> {
    match event {
        DurableCoreEvent::MessageSent {
            id,
            role,
            content,
            timestamp,
            provider,
        } => Some(Event::MessageReplayed {
            id: id.clone(),
            role: role.clone(),
            content: content.clone(),
            timestamp: *timestamp,
            provider: provider.clone(),
        }),
        DurableCoreEvent::ToolCalled { id, name, input } => Some(Event::ToolStart {
            id: id.clone(),
            name: name.clone(),
            input: input.clone(),
        }),
        DurableCoreEvent::ToolResult { id, output, .. } => Some(Event::ToolEnd {
            id: id.clone(),
            duration_secs: 0.0,
            output: output.clone(),
        }),
        DurableCoreEvent::ModelSwitched { provider, model } => Some(Event::SwitchModel {
            provider: provider.clone(),
            model: model.clone(),
            explicit: false,
        }),
        DurableCoreEvent::SessionRenamed { .. } => None, // handled directly in replay_event
        DurableCoreEvent::ThemeSwitched { name } => Some(Event::SwitchTheme { name: name.clone() }),
        DurableCoreEvent::ThinkingLevelSet { level } => Some(Event::SetThinkingLevel(*level)),
        // ReadOnlySet is handled directly in replay_event to avoid lossy toggle conversion
        DurableCoreEvent::ReadOnlySet { .. } => None,
    }
}

/// Replay a single durable event directly into application state.
pub fn replay_event(state: &mut AppState, event: &DurableCoreEvent) {
    match event {
        DurableCoreEvent::MessageSent {
            id,
            role,
            content,
            timestamp,
            provider,
        } => {
            state.replay_message(
                id.clone(),
                role.clone(),
                content.clone(),
                *timestamp,
                provider.clone(),
            );
        }
        DurableCoreEvent::SessionRenamed { name } => {
            state.session_mut().session_display_name = Some(name.clone());
        }
        DurableCoreEvent::ToolCalled { .. }
        | DurableCoreEvent::ToolResult { .. }
        | DurableCoreEvent::ModelSwitched { .. }
        | DurableCoreEvent::ThemeSwitched { .. }
        | DurableCoreEvent::ThinkingLevelSet { .. }
        | DurableCoreEvent::ReadOnlySet { .. } => {
            if let Some(evt) = durable_to_event(event) {
                state.update(evt);
            }
        }
    }
}

/// Replay a sequence of durable events into application state.
pub fn replay_events(state: &mut AppState, events: &[DurableCoreEvent]) {
    for event in events {
        replay_event(state, event);
    }
}

/// Build durable events from current application state.
pub fn state_to_durable_events(state: &AppState) -> Vec<DurableCoreEvent> {
    session_to_durable_events(&crate::session::Session::from_state(
        state,
        state
            .session
            .session_display_name
            .clone()
            .unwrap_or_else(|| "session".into()),
    ))
}

/// Build durable events from a session snapshot.
pub fn session_to_durable_events(session: &crate::session::Session) -> Vec<DurableCoreEvent> {
    let mut events = Vec::new();
    events.extend(messages_to_events(&session.messages));
    events.push(DurableCoreEvent::ModelSwitched {
        provider: session.provider.clone(),
        model: session.model.clone(),
    });
    events.push(DurableCoreEvent::ThemeSwitched {
        name: session.theme_name.clone(),
    });
    events.push(DurableCoreEvent::ThinkingLevelSet {
        level: session.thinking_level,
    });
    if session.read_only {
        events.push(DurableCoreEvent::ReadOnlySet { read_only: true });
    }
    if let Some(name) = &session.display_name {
        events.push(DurableCoreEvent::SessionRenamed { name: name.clone() });
    }
    events
}

fn messages_to_events(messages: &[ChatMessage]) -> Vec<DurableCoreEvent> {
    messages.iter().filter_map(message_to_event).collect()
}

fn message_to_event(message: &ChatMessage) -> Option<DurableCoreEvent> {
    match message.role {
        Role::TurnComplete => None,
        _ => Some(DurableCoreEvent::MessageSent {
            id: message.id.clone(),
            role: message.role.as_str().to_owned(),
            content: message.content(),
            timestamp: message.timestamp,
            provider: message.provider.clone(),
        }),
    }
}

/// Build session metadata from current application state.
fn build_metadata(state: &AppState, name: &str) -> SessionMetadata {
    SessionMetadata {
        id: name.to_owned(),
        display_name: state
            .session
            .session_display_name
            .clone()
            .unwrap_or_else(|| name.to_owned()),
        created_at: state.session().session_created_at,
        updated_at: crate::message::now(),
        message_count: state.session().messages.len(),
        summary: None,
        is_starred: false,
        is_system: false,
    }
}

/// Save current application state as durable events.
pub fn save_session(name: &str, state: &AppState) -> anyhow::Result<()> {
    let store =
        SessionStore::default_store().ok_or_else(|| anyhow::anyhow!("No data directory"))?;
    let events = state_to_durable_events(state);
    store.append_batch(name, &events)?;
    store.update_metadata(&build_metadata(state, name))?;
    Ok(())
}

/// Load durable events into application state.
pub fn load_session(name: &str, state: &mut AppState) -> anyhow::Result<()> {
    let store =
        SessionStore::default_store().ok_or_else(|| anyhow::anyhow!("No data directory"))?;
    let events = store.load_events(name)?;
    if events.is_empty() {
        return Err(anyhow::anyhow!("Session '{}' not found", name));
    }
    replay_events(state, &events);
    restore_metadata(name, state, &store)?;
    state.configure_token_tracker();
    state.messages_changed();
    Ok(())
}

fn restore_metadata(name: &str, state: &mut AppState, store: &SessionStore) -> anyhow::Result<()> {
    if let Some(meta) = store.load_metadata(name)? {
        // Only overwrite session_display_name if the metadata's display_name
        // differs from the session name — identical names mean the metadata is
        // just storing the session name as a fallback, not a custom display.
        if meta.display_name != name {
            state.session_mut().session_display_name = Some(meta.display_name.clone());
        }
        state.session_mut().session_created_at = meta.created_at;
        state.session_mut().session_updated_at = meta.updated_at;
    }
    Ok(())
}

/// Apply a JSON session snapshot to application state.
pub fn apply_json_session(state: &mut AppState, session: &crate::session::Session) {
    state.restore_session(session);
}

/// Delete a session from the durable store.
pub fn delete_session(name: &str) -> anyhow::Result<()> {
    let store =
        SessionStore::default_store().ok_or_else(|| anyhow::anyhow!("No data directory"))?;
    store.delete(name)
}

/// List session names from the durable store.
pub fn list_sessions() -> anyhow::Result<Vec<String>> {
    let store =
        SessionStore::default_store().ok_or_else(|| anyhow::anyhow!("No data directory"))?;
    store.list()
}

/// Seed the durable store from a snapshot DTO. Test helper only.
#[cfg(test)]
pub fn save_snapshot(name: &str, session: &crate::session::Session) -> anyhow::Result<()> {
    let mut state = AppState::default();
    state.restore_session(session);
    save_session(name, &state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ChatMessage, Role};

    fn test_store() -> SessionStore {
        let dir = tempfile::tempdir().unwrap();
        SessionStore::new(dir.path().to_path_buf())
    }

    fn sample_state() -> AppState {
        let mut state = AppState::default();
        state.config.current_provider = "anthropic".into();
        state.config.current_model = "claude-3".into();
        state.session.messages.push(ChatMessage {
            role: Role::User,
            parts: vec![crate::message::Part::Text {
                content: "Hello".into(),
            }],
            timestamp: 1.0,
            id: "msg.1".into(),
            ..Default::default()
        });
        state.session.messages.push(ChatMessage {
            role: Role::Assistant,
            parts: vec![crate::message::Part::Text {
                content: "Hi!".into(),
            }],
            timestamp: 2.0,
            id: "msg.2".into(),
            provider: "anthropic".into(),
            ..Default::default()
        });
        state
    }

    #[test]
    fn state_to_events_includes_messages_and_config() {
        let state = sample_state();
        let events = state_to_durable_events(&state);
        assert!(events.iter().any(
            |e| matches!(e, DurableCoreEvent::MessageSent { content, .. } if content == "Hello")
        ));
        assert!(events.iter().any(|e| matches!(e, DurableCoreEvent::ModelSwitched { provider, .. } if provider == "anthropic")));
    }

    #[test]
    fn replay_events_restore_messages() {
        let state = sample_state();
        let events = state_to_durable_events(&state);
        let mut loaded = AppState::default();
        replay_events(&mut loaded, &events);
        assert_eq!(loaded.session.messages.len(), 2);
        assert_eq!(loaded.session.messages[0].content(), "Hello");
        assert_eq!(loaded.config.current_provider, "anthropic");
    }

    #[test]
    fn save_replays_events() {
        let store = test_store();
        let state = sample_state();
        let events = state_to_durable_events(&state);
        store.append_batch("save_test", &events).unwrap();

        let loaded = store.load_events("save_test").unwrap();
        assert!(
            loaded.iter().any(
                |e| matches!(e, DurableCoreEvent::MessageSent { content, .. } if content == "Hello")
            ),
            "save should persist user message as durable event"
        );
        assert!(
            loaded.iter().any(|e| matches!(e, DurableCoreEvent::ModelSwitched { provider, .. } if provider == "anthropic")),
            "save should persist provider/model as durable event"
        );
    }

    #[test]
    fn load_replays_events() {
        let store = test_store();
        let state = sample_state();
        let events = state_to_durable_events(&state);
        store.append_batch("load_test", &events).unwrap();

        let mut loaded = AppState::default();
        let loaded_events = store.load_events("load_test").unwrap();
        replay_events(&mut loaded, &loaded_events);

        assert_eq!(loaded.session.messages.len(), 2);
        assert_eq!(loaded.session.messages[0].content(), "Hello");
        assert_eq!(loaded.session.messages[1].content(), "Hi!");
        assert_eq!(loaded.config.current_provider, "anthropic");
        assert_eq!(loaded.config.current_model, "claude-3");
    }

    #[test]
    fn save_and_load_roundtrip() {
        let store = test_store();
        let mut state = sample_state();
        state.session_mut().session_display_name = Some("roundtrip".into());

        let events = state_to_durable_events(&state);
        store.append_batch("roundtrip", &events).unwrap();
        store
            .update_metadata(&SessionMetadata {
                id: "roundtrip".into(),
                display_name: "roundtrip".into(),
                created_at: 10.0,
                updated_at: 20.0,
                message_count: 2,
                summary: None,
                is_starred: false,
                is_system: false,
            })
            .unwrap();

        let mut loaded = AppState::default();
        let events = store.load_events("roundtrip").unwrap();
        replay_events(&mut loaded, &events);
        assert_eq!(loaded.session.messages.len(), 2);
        assert_eq!(loaded.config.current_provider, "anthropic");
    }

    /// Layer 4: Verify that after a completed mock turn (simulating a real turn
    /// that produced user + assistant messages), calling `/save` creates a session
    /// JSONL file.
    ///
    /// Note: session persistence is triggered by `/save` (explicit command), NOT
    /// by `TurnComplete`. Sessions are maintained in memory by `SessionActor` and
    /// only written to disk via `/save` or `/export`. This is the correct design —
    /// auto-saving on every turn would cause excessive disk writes.
    #[test]
    fn save_after_completed_turn_creates_session_file() {
        let store = test_store();

        // Simulate a completed mock turn: user message + assistant response
        let mut state = AppState::default();
        state.apply_user_message_submitted("req.0".into(), "hello".into());
        state.set_thinking("req.0".into());
        state.add_thought("req.0".into());
        state.append_response("req.0".into(), "hello ".into());
        state.complete_turn("req.0".into(), 0.5);

        // Verify session has the expected messages
        assert_eq!(state.session.messages.len(), 4); // user, thought, assistant, turn_complete

        // Simulate `/save`: convert state to durable events and persist
        let events = state_to_durable_events(&state);
        store.append_batch("mock_turn_test", &events).expect("save should succeed");

        // Verify session file was created
        assert!(
            store.exists("mock_turn_test"),
            "session file should exist after save"
        );

        // Verify the session can be loaded back
        let loaded_events = store
            .load_events("mock_turn_test")
            .expect("should load events");
        assert!(!loaded_events.is_empty(), "loaded events should not be empty");

        // Verify user message was persisted
        assert!(loaded_events.iter().any(|e| matches!(
            e,
            DurableCoreEvent::MessageSent { role, content, .. }
                if role == "user" && content == "hello"
        )));
    }
}
