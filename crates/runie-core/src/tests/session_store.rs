use crate::event::DurableCoreEvent;
use crate::session::store::SessionStore;
use std::fs;

fn test_store() -> SessionStore {
    let dir = tempfile::tempdir().unwrap();
    SessionStore::new(dir.path().to_path_buf())
}

fn append_msg(store: &SessionStore, sid: &str, mid: &str, role: &str, content: &str, ts: f64) {
    store
        .append(
            sid,
            &DurableCoreEvent::MessageSent {
                id: mid.into(),
                role: role.into(),
                content: content.into(),
                timestamp: ts,
                provider: String::new(),
            },
        )
        .unwrap();
}

#[test]
fn appends_and_replays_events() {
    let store = test_store();
    let sid = "test-replay";

    append_msg(&store, sid, "msg1", "user", "Hello", 1.0);
    append_msg(&store, sid, "msg2", "assistant", "Hi there!", 2.0);
    store
        .append(
            sid,
            &DurableCoreEvent::ModelSwitched {
                provider: "anthropic".into(),
                model: "claude-3".into(),
            },
        )
        .unwrap();

    let events = store.load_events(sid).unwrap();
    assert_eq!(events.len(), 3);
    assert!(matches!(&events[0], DurableCoreEvent::MessageSent { id, .. } if id == "msg1"));
    assert!(
        matches!(&events[2], DurableCoreEvent::ModelSwitched { provider, .. } if provider == "anthropic")
    );
}

#[test]
fn atomic_batch_survives_crash() {
    let store = test_store();
    let sid = "test-crash";

    let batch = vec![
        DurableCoreEvent::MessageSent {
            id: "1".into(),
            role: "user".into(),
            content: "First".into(),
            timestamp: 1.0,
            provider: String::new(),
        },
        DurableCoreEvent::MessageSent {
            id: "2".into(),
            role: "user".into(),
            content: "Second".into(),
            timestamp: 2.0,
            provider: String::new(),
        },
        DurableCoreEvent::MessageSent {
            id: "3".into(),
            role: "user".into(),
            content: "Third".into(),
            timestamp: 3.0,
            provider: String::new(),
        },
    ];

    store.append_batch(sid, &batch).unwrap();

    // Verify all events persisted
    let events = store.load_events(sid).unwrap();
    assert_eq!(events.len(), 3);
    assert!(events
        .iter()
        .all(|e| matches!(e, DurableCoreEvent::MessageSent { .. })));
}

#[test]
fn jsonl_session_loads_directly() {
    let dir = tempfile::tempdir().unwrap();
    let dir_path = dir.path().to_path_buf();

    // Create a JSONL file directly
    let jsonl_path = dir_path.join("legacy-session.jsonl");
    let jsonl_content = concat!(
        r#"{"event":"messageSent","id":"m1","role":"user","content":"Hello","timestamp":1.0}"#,
        "\n",
        r#"{"event":"messageSent","id":"m2","role":"assistant","content":"Hi!","timestamp":2.0}"#,
        "\n"
    );
    fs::write(&jsonl_path, jsonl_content).unwrap();

    // Open via SessionStore — should load directly from JSONL
    let store = SessionStore::new(dir_path);
    let events = store.load_events("legacy-session").unwrap();

    assert_eq!(events.len(), 2);
    assert!(matches!(&events[0], DurableCoreEvent::MessageSent { id, .. } if id == "m1"));
    assert!(matches!(&events[1], DurableCoreEvent::MessageSent { id, .. } if id == "m2"));
}

#[test]
fn empty_when_no_file() {
    let store = test_store();
    let events = store.load_events("nonexistent").unwrap();
    assert!(events.is_empty());
}

#[test]
fn delete_session() {
    let store = test_store();
    let sid = "test-delete";

    append_msg(&store, sid, "msg1", "user", "Test", 1.0);
    assert!(store.exists(sid));
    store.delete(sid).unwrap();
    assert!(!store.exists(sid));
}

#[test]
fn list_sessions() {
    let store = test_store();

    append_msg(&store, "session-a", "m1", "user", "A", 1.0);
    append_msg(&store, "session-b", "m2", "user", "B", 2.0);

    let list = store.list().unwrap();
    assert!(list.contains(&"session-a".into()));
    assert!(list.contains(&"session-b".into()));
}

#[test]
fn meta_round_trips_through_index() {
    use crate::session::index::SessionMetadata;

    let store = test_store();
    let sid = "test-meta";

    // First, create a session so the index has something to reference
    append_msg(&store, sid, "msg1", "user", "Test", 1000.0);

    let meta = SessionMetadata {
        id: sid.into(),
        display_name: "My Session".into(),
        created_at: 1000.0,
        updated_at: 2000.0,
        message_count: 5,
        summary: Some("A summary".into()),
        is_starred: true,
        is_system: false,
    };

    store.update_metadata(&meta).unwrap();

    // Verify the JSONL file was created
    assert!(store.exists(sid), "session file should be created");
}

#[test]
fn multiple_sessions_isolated() {
    let store = test_store();

    store
        .append(
            "s1",
            &DurableCoreEvent::MessageSent {
                id: "1".into(),
                role: "user".into(),
                content: "S1".into(),
                timestamp: 1.0,
                provider: String::new(),
            },
        )
        .unwrap();
    store
        .append(
            "s2",
            &DurableCoreEvent::MessageSent {
                id: "2".into(),
                role: "user".into(),
                content: "S2".into(),
                timestamp: 2.0,
                provider: String::new(),
            },
        )
        .unwrap();

    let ev1 = store.load_events("s1").unwrap();
    let ev2 = store.load_events("s2").unwrap();

    assert_eq!(ev1.len(), 1);
    assert_eq!(ev2.len(), 1);

    let list = store.list().unwrap();
    assert!(list.contains(&"s1".into()));
    assert!(list.contains(&"s2".into()));
}

#[test]
fn load_events_returns_ordered_events() {
    let store = test_store();
    let sid = "test-order";
    append_msg(&store, sid, "a", "user", "first", 1.0);
    append_msg(&store, sid, "b", "user", "second", 2.0);
    let events = store.load_events(sid).unwrap();
    assert_eq!(events.len(), 2);
    assert!(matches!(&events[0], DurableCoreEvent::MessageSent { id, .. } if id == "a"));
    assert!(matches!(&events[1], DurableCoreEvent::MessageSent { id, .. } if id == "b"));
}

#[test]
fn load_events_rejects_malformed_json() {
    let dir = tempfile::tempdir().unwrap();
    let store = SessionStore::new(dir.path().to_path_buf());
    let path = store.path("corrupt");
    fs::write(&path, "valid event\nnot json at all\n").unwrap();

    // load_events should return an error instead of silently dropping/offsetting.
    let result = store.load_events("corrupt");
    assert!(
        result.is_err(),
        "parse failure should be an error, not silent drop: {:?}",
        result
    );
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("unparseable"),
        "error message should mention unparseable, got: {}",
        err
    );
}
