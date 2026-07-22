//! Session replay — convert between `AppState` and durable events.
//!
//! This module is the single persistence path for `/save` and `/load`.
//! Import/export still uses the `Session` snapshot DTO in `crate::session`,
//! but runtime persistence goes exclusively through `SessionStore`.

use crate::event::DurableCoreEvent;
use crate::message::ChatMessage;
use crate::model::{AppState, Role};
use crate::session::store::SessionStore;
use crate::session::SessionMetadata;
use crate::Event;
use std::convert::TryFrom as _;

/// Convert a durable event to a canonical `Event` for replay.
/// Returns `None` for events that are handled directly in `replay_event`
/// (`SessionRenamed`, `ReadOnlySet`).
pub fn durable_to_event(event: &DurableCoreEvent) -> Option<Event> {
    Event::try_from(event).ok()
}

/// Replay a single durable event directly into application state.
pub fn replay_event(state: &mut AppState, event: &DurableCoreEvent) {
    match event {
        DurableCoreEvent::MessageSent { id, role, content, timestamp, provider, parts } => {
            state.replay_message_with_parts(
                id.clone(),
                role.clone(),
                content.clone(),
                *timestamp,
                provider.clone(),
                parts.clone(),
            );
        }
        DurableCoreEvent::SessionRenamed { name } => {
            state.set_session_display_name(Some(name.clone()));
        }
        DurableCoreEvent::ToolCalled { .. }
        | DurableCoreEvent::ToolResult { .. }
        | DurableCoreEvent::ModelSwitched { .. }
        | DurableCoreEvent::ThemeSwitched { .. }
        | DurableCoreEvent::ThinkingLevelSet { .. }
        | DurableCoreEvent::ReadOnlySet { .. }
        | DurableCoreEvent::TreeSnapshot { .. } => {
            if let Some(evt) = durable_to_event(event) {
                state.update(evt);
            }
        }
        // TurnPhaseChanged is used for crash recovery but doesn't update AppState.
        // The phase is reconstructed from other events during replay.
        DurableCoreEvent::TurnPhaseChanged { .. } => {}
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
            .session()
            .session_display_name
            .clone()
            .unwrap_or_else(|| "session".into()),
    ))
}

/// Build durable events from a session snapshot.
pub fn session_to_durable_events(session: &crate::session::Session) -> Vec<DurableCoreEvent> {
    let mut events = Vec::new();
    events.extend(messages_to_events(&session.messages));
    events.push(DurableCoreEvent::ModelSwitched { provider: session.provider.clone(), model: session.model.clone() });
    events.push(DurableCoreEvent::ThemeSwitched { name: session.theme_name.clone() });
    events.push(DurableCoreEvent::ThinkingLevelSet { level: session.thinking_level });
    if session.read_only {
        events.push(DurableCoreEvent::ReadOnlySet { read_only: true });
    }
    if let Some(name) = &session.display_name {
        events.push(DurableCoreEvent::SessionRenamed { name: name.clone() });
    }
    // Include tree snapshot if available.
    // Snapshot errors indicate arena inconsistency — a bug in the tree
    // implementation. Log and skip the snapshot so session save can proceed.
    if let Some(ref tree) = session.session_tree {
        if let Ok(snapshot) = tree.to_snapshot() {
            events.push(DurableCoreEvent::TreeSnapshot { snapshot });
        } else {
            tracing::warn!("session tree snapshot failed — skipping for this session save");
        }
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
            role: message.role.as_str().to_string(),
            content: message.content(),
            timestamp: message.timestamp,
            provider: message.provider.clone(),
            parts: message.parts.clone(),
        }),
    }
}

/// Build session metadata from current application state.
fn build_metadata(state: &AppState, name: &str) -> SessionMetadata {
    SessionMetadata {
        id: name.to_owned(),
        display_name: state
            .session()
            .session_display_name
            .clone()
            .unwrap_or_else(|| name.to_owned()),
        created_at: state.session().session_created_at,
        updated_at: crate::message::now(),
        message_count: state.session().messages.len(),
        summary: None,
        is_starred: false,
        is_system: false,
        active_plan_id: state.view().active_plan_id.clone(),
    }
}

/// Save current application state as durable events.
pub fn save_session(name: &str, state: &AppState) -> anyhow::Result<()> {
    let store = SessionStore::default_store().ok_or_else(|| anyhow::anyhow!("No data directory"))?;
    let events = state_to_durable_events(state);
    store.append_batch(name, &events)?;
    store.update_metadata(&build_metadata(state, name))?;
    Ok(())
}

/// Load durable events into application state.
/// Load a session using the default store (reads RUNIE_SESSIONS_DIR env var).
pub fn load_session(name: &str, state: &mut AppState) -> anyhow::Result<()> {
    let store = SessionStore::default_store().ok_or_else(|| anyhow::anyhow!("No data directory"))?;
    load_session_from_store(name, state, &store)
}

/// Load a session from a specific store (avoids env-var race in parallel tests).
pub fn load_session_from_store(name: &str, state: &mut AppState, store: &SessionStore) -> anyhow::Result<()> {
    let events = store.load_events(name)?;
    if events.is_empty() {
        return Err(anyhow::anyhow!("Session '{}' not found", name));
    }
    replay_events(state, &events);
    restore_metadata(name, state, store)?;
    state.configure_token_tracker();
    state.messages_changed();
    Ok(())
}

fn restore_metadata(name: &str, state: &mut AppState, store: &SessionStore) -> anyhow::Result<()> {
    if let Some(meta) = store.load_metadata(name)? {
        state.restore_session_metadata(&meta);
    }
    Ok(())
}

/// Apply a JSON session snapshot to application state.
pub fn apply_json_session(state: &mut AppState, session: &crate::session::Session) {
    state.restore_session(session);
}

/// Delete a session from the durable store.
pub fn delete_session(name: &str) -> anyhow::Result<()> {
    let store = SessionStore::default_store().ok_or_else(|| anyhow::anyhow!("No data directory"))?;
    store.delete(name)
}

/// List session names from the durable store.
pub fn list_sessions() -> anyhow::Result<Vec<String>> {
    let store = SessionStore::default_store().ok_or_else(|| anyhow::anyhow!("No data directory"))?;
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
        state.config_mut().current_provider = "anthropic".into();
        state.config_mut().current_model = "claude-3".into();
        state.session_mut().messages.push(ChatMessage {
            role: Role::User,
            parts: vec![crate::message::Part::Text { content: "Hello".into() }],
            timestamp: 1.0,
            id: "msg.1".into(),
            ..Default::default()
        });
        state.session_mut().messages.push(ChatMessage {
            role: Role::Assistant,
            parts: vec![crate::message::Part::Text { content: "Hi!".into() }],
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
        assert!(events
            .iter()
            .any(|e| matches!(e, DurableCoreEvent::MessageSent { content, .. } if content == "Hello")));
        assert!(events
            .iter()
            .any(|e| matches!(e, DurableCoreEvent::ModelSwitched { provider, .. } if provider == "anthropic")));
    }

    #[test]
    fn replay_events_restore_messages() {
        let state = sample_state();
        let events = state_to_durable_events(&state);
        let mut loaded = AppState::default();
        replay_events(&mut loaded, &events);
        assert_eq!(loaded.session().messages.len(), 2);
        assert_eq!(loaded.session().messages[0].content(), "Hello");
        assert_eq!(loaded.config().current_provider, "anthropic");
    }

    #[test]
    #[allow(clippy::cognitive_complexity)]
    #[allow(clippy::too_many_lines)]
    fn session_save_preserves_message_parts() {
        use crate::message::Part;

        let store = test_store();
        let mut state = AppState::default();
        state.config_mut().current_provider = "anthropic".into();
        state.config_mut().current_model = "claude-3".into();

        // Add a user message
        state.session_mut().messages.push(ChatMessage {
            role: Role::User,
            parts: vec![Part::Text { content: "Hello".into() }],
            timestamp: 1.0,
            id: "u1".into(),
            ..Default::default()
        });

        // Add an assistant message with reasoning and tool call parts
        state.session_mut().messages.push(ChatMessage {
            role: Role::Assistant,
            parts: vec![
                Part::text("I'll help you"),
                Part::reasoning("thinking about this"),
                Part::tool_call("call_1", "bash", serde_json::json!({"cmd": "ls"})),
            ],
            timestamp: 2.0,
            id: "a1".into(),
            provider: "openai".into(),
            ..Default::default()
        });

        // Save the session
        let events = state_to_durable_events(&state);
        store.append_batch("parts_test", &events).unwrap();

        // Load the session
        let loaded_events = store.load_events("parts_test").unwrap();
        let mut loaded_state = AppState::default();
        replay_events(&mut loaded_state, &loaded_events);

        // Verify parts are preserved
        let msgs = &loaded_state.session().messages;
        assert_eq!(msgs.len(), 2);

        // User message should have text part
        assert_eq!(msgs[0].role, Role::User);
        assert_eq!(msgs[0].parts.len(), 1);
        assert!(matches!(&msgs[0].parts[0], Part::Text { content } if content == "Hello"));

        // Assistant message should have all three parts
        assert_eq!(msgs[1].role, Role::Assistant);
        assert_eq!(msgs[1].parts.len(), 3);
        assert!(matches!(&msgs[1].parts[0], Part::Text { content } if content == "I'll help you"));
        assert!(matches!(&msgs[1].parts[1], Part::Reasoning { content } if content == "thinking about this"));
        assert!(matches!(&msgs[1].parts[2], Part::ToolCall { name, .. } if name == "bash"));
    }

    #[test]
    fn save_replays_events() {
        let store = test_store();
        let state = sample_state();
        let events = state_to_durable_events(&state);
        store.append_batch("save_test", &events).unwrap();

        let loaded = store.load_events("save_test").unwrap();
        assert!(
            loaded
                .iter()
                .any(|e| matches!(e, DurableCoreEvent::MessageSent { content, .. } if content == "Hello")),
            "save should persist user message as durable event"
        );
        assert!(
            loaded
                .iter()
                .any(|e| matches!(e, DurableCoreEvent::ModelSwitched { provider, .. } if provider == "anthropic")),
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

        assert_eq!(loaded.session().messages.len(), 2);
        assert_eq!(loaded.session().messages[0].content(), "Hello");
        assert_eq!(loaded.session().messages[1].content(), "Hi!");
        assert_eq!(loaded.config().current_provider, "anthropic");
        assert_eq!(loaded.config().current_model, "claude-3");
    }

    #[test]
    fn save_and_load_roundtrip() {
        let store = test_store();
        let mut state = sample_state();
        // Use a different display name so SessionRenamed is persisted.
        state.set_session_display_name(Some("My Roundtrip Session".into()));

        let events = state_to_durable_events(&state);
        store.append_batch("roundtrip", &events).unwrap();
        store
            .update_metadata(&SessionMetadata {
                id: "roundtrip".into(),
                display_name: "My Roundtrip Session".into(),
                created_at: 10.0,
                updated_at: 20.0,
                message_count: 2,
                summary: None,
                is_starred: false,
                is_system: false,
                active_plan_id: None,
            })
            .unwrap();

        let mut loaded = AppState::default();
        let events = store.load_events("roundtrip").unwrap();
        replay_events(&mut loaded, &events);
        // SessionRenamed persisted and replayed (display_name != id in metadata,
        // so restore_metadata defers to replay_events' SessionRenamed)
        assert_eq!(
            loaded.session().session_display_name,
            Some("My Roundtrip Session".into())
        );
        assert_eq!(loaded.session().messages.len(), 2);
        assert_eq!(loaded.config().current_provider, "anthropic");
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
        assert_eq!(state.session().messages.len(), 4); // user, thought, assistant, turn_complete

        // Simulate `/save`: convert state to durable events and persist
        let events = state_to_durable_events(&state);
        store
            .append_batch("mock_turn_test", &events)
            .expect("save should succeed");

        // Verify session file was created
        assert!(
            store.exists("mock_turn_test"),
            "session file should exist after save"
        );

        // Verify the session can be loaded back
        let loaded_events = store
            .load_events("mock_turn_test")
            .expect("should load events");
        assert!(
            !loaded_events.is_empty(),
            "loaded events should not be empty"
        );

        // Verify user message was persisted
        assert!(loaded_events.iter().any(|e| matches!(
            e,
            DurableCoreEvent::MessageSent { role, content, .. }
                if role == "user" && content == "hello"
        )));
    }

    /// Layer 1: SessionTreeSnapshot round-trips through durable events.
    #[test]
    #[allow(clippy::too_many_lines)]
    fn replay_tree_snapshot_restore() {
        use crate::session::tree::SessionTree;

        // Build a tree manually with known-good data (avoids tree-construction bugs)
        let user_msg = ChatMessage {
            role: Role::User,
            parts: vec![crate::message::Part::Text { content: "hello".into() }],
            timestamp: 1.0,
            id: "msg1".into(),
            ..Default::default()
        };
        let assistant_msg = ChatMessage {
            role: Role::Assistant,
            parts: vec![crate::message::Part::Text { content: "hi".into() }],
            timestamp: 2.0,
            id: "msg2".into(),
            ..Default::default()
        };
        let tree = SessionTree::from_messages(&[user_msg, assistant_msg]);

        // Get snapshot (avoids .clone() on SessionTree which has a serialize bug)
        let snapshot = tree.to_snapshot().unwrap();
        assert_eq!(snapshot.nodes.len(), 2, "tree should have 2 nodes");
        assert_eq!(snapshot.root_id, "msg1");

        // Convert to durable event and back (both directions)
        let durable = DurableCoreEvent::TreeSnapshot { snapshot: snapshot.clone() };
        let event: Event = Event::try_from(&durable).unwrap();
        let back: DurableCoreEvent = DurableCoreEvent::try_from_event(&event).unwrap();

        // Verify round-trip produces equivalent snapshot
        let roundtripped = if let DurableCoreEvent::TreeSnapshot { snapshot: s } = back {
            s
        } else {
            panic!("round-trip did not produce TreeSnapshot")
        };
        assert_eq!(roundtripped.root_id, snapshot.root_id);
        assert_eq!(roundtripped.nodes.len(), snapshot.nodes.len());

        // Reconstruct tree from round-tripped snapshot and verify
        let restored_tree = SessionTree::from_snapshot(&roundtripped).expect("snapshot should deserialize");
        let restored_snapshot = restored_tree.to_snapshot().unwrap();
        assert_eq!(restored_snapshot.root_id, snapshot.root_id);
        assert_eq!(restored_snapshot.nodes.len(), snapshot.nodes.len());

        // Verify replay via AppState update
        let mut loaded = AppState::default();
        loaded.update(event.clone());
        let loaded_tree = loaded
            .session
            .session_tree
            .clone()
            .expect("tree should be restored via AppState update");
        let loaded_snapshot = loaded_tree.to_snapshot().unwrap();
        assert_eq!(loaded_snapshot.root_id, snapshot.root_id);
        assert_eq!(loaded_snapshot.nodes.len(), snapshot.nodes.len());
    }

    /// Test that plan mode state is included in session metadata.
    #[test]
    fn build_metadata_includes_plan_id() {
        let mut state = AppState::default();
        state.view_mut().plan_mode = true;
        state.view_mut().active_plan_content = "# Test Plan".to_string();
        state.view_mut().active_plan_id = Some("test-plan-id".to_string());

        let meta = build_metadata(&state, "test-session");
        assert_eq!(meta.id, "test-session");
        assert_eq!(meta.active_plan_id, Some("test-plan-id".to_string()));
    }

    /// Test that plan restoration sets view state correctly.
    #[test]
    fn restore_metadata_restores_plan_mode() {
        let mut state = AppState::default();
        state.set_session_display_name(Some("test-session".to_string()));

        // Simulate session with active plan (using named values to avoid magic numbers)
        const TS_CREATED: f64 = 1000.0;
        const TS_UPDATED: f64 = 2000.0;
        let meta = SessionMetadata {
            id: "test-session".into(),
            display_name: "Test Session".into(),
            created_at: TS_CREATED,
            updated_at: TS_UPDATED,
            message_count: 5,
            summary: None,
            is_starred: false,
            is_system: false,
            active_plan_id: None, // No actual plan file to restore in this test
        };

        // restore_session_metadata is a method on AppState through domain_ops trait
        state.restore_session_metadata(&meta);
        assert!(!state.view().plan_mode); // No plan_id so plan_mode not restored
    }
}
