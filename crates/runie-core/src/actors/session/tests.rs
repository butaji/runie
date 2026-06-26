//! Tests for `SessionActor`.

use std::path::PathBuf;

use tempfile::TempDir;

use crate::actors::session::SessionActor;
use crate::bus::EventBus;
use crate::edit_preview::EditPreview;
use crate::message::now;
use crate::model::{Role, SessionState};
use crate::session::store::SessionStore;
use crate::trust::{TrustDecision, TrustManager};
use crate::Event;

fn make_actor() -> SessionActor {
    let dir = TempDir::new().unwrap();
    SessionActor {
        bus: EventBus::new(4),
        trust: TrustManager::default(),
        store: SessionStore::new(dir.into_path()),
        session_id: String::new(),
        display_name: String::new(),
        message_count: 0,
        summary_buffer: String::new(),
        started_at: now(),
        session_state: SessionState::default(),
        next_id: 0,
        thought_seq: 0,
    }
}

#[tokio::test]
async fn actor_loads_and_emits_trust_and_history() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = tmp.path().join("cfg");
    let data = tmp.path().join("data");
    std::fs::create_dir_all(&cfg).unwrap();
    std::fs::create_dir_all(&data).unwrap();
    // FIXME: Audit that the environment access only happens in single-threaded code.
    unsafe { std::env::set_var("RUNIE_TEST_CONFIG_DIR", &cfg) };
    // FIXME: Audit that the environment access only happens in single-threaded code.
    unsafe { std::env::set_var("RUNIE_TEST_DATA_DIR", &data) };

    let bus = EventBus::<Event>::new(4);
    let mut sub = bus.subscribe();
    let (handle, _actor_handle) = SessionActor::spawn(bus);

    let mut saw_trust = false;
    let mut saw_history = false;
    for _ in 0..60 {
        if saw_trust && saw_history {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        while let Ok(evt) = sub.try_recv() {
            match evt {
                Event::TrustLoaded { .. } => saw_trust = true,
                Event::HistoryLoaded { .. } => saw_history = true,
                _ => {}
            }
        }
    }
    assert!(saw_trust);
    assert!(saw_history);

    handle
        .set_trust(PathBuf::from("/tmp/project"), TrustDecision::Trusted)
        .await;

    let mut saw_changed = false;
    for _ in 0..60 {
        if saw_changed {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        while let Ok(evt) = sub.try_recv() {
            if matches!(evt, Event::TrustChanged { .. }) {
                saw_changed = true;
            }
        }
    }
    assert!(saw_changed);

    // FIXME: Audit that the environment access only happens in single-threaded code.
    unsafe { std::env::remove_var("RUNIE_TEST_CONFIG_DIR") };
    // FIXME: Audit that the environment access only happens in single-threaded code.
    unsafe { std::env::remove_var("RUNIE_TEST_DATA_DIR") };
}

#[test]
fn session_actor_add_user_message_updates_timestamps() {
    let mut actor = make_actor();
    let before = actor.session_state.session_updated_at;
    std::thread::sleep(std::time::Duration::from_millis(1));
    actor.handle_add_user_message("hello".into(), vec![]);
    assert!(
        actor.session_state.session_updated_at > before,
        "timestamp must advance after mutation"
    );
    assert_eq!(actor.session_state.messages.len(), 1);
}

#[test]
fn session_actor_add_system_message_works() {
    let mut actor = make_actor();
    actor.handle_add_system_message("system note".into());
    assert_eq!(actor.session_state.messages.len(), 1);
    assert_eq!(actor.session_state.messages[0].role, Role::System);
}

#[test]
fn session_actor_handle_reset_clears_messages() {
    let mut actor = make_actor();
    actor.handle_add_user_message("hello".into(), vec![]);
    actor.handle_add_system_message("sys".into());
    assert_eq!(actor.session_state.messages.len(), 2);
    actor.handle_reset();
    assert!(
        actor.session_state.messages.is_empty(),
        "reset must clear all messages"
    );
}

#[test]
fn session_actor_pending_edit_lifecycle() {
    let mut actor = make_actor();
    let edit = EditPreview::new(
        std::path::PathBuf::from("test.txt"),
        "old".into(),
        "new".into(),
    );
    actor.handle_push_pending_edit(edit);
    assert_eq!(actor.session_state.pending_edits.len(), 1);
    actor.handle_clear_pending_edits();
    assert!(
        actor.session_state.pending_edits.is_empty(),
        "clear should remove all pending edits"
    );
}

#[test]
fn session_actor_fork_at_creates_session_tree() {
    let mut actor = make_actor();
    actor.handle_add_user_message("first".into(), vec![]);
    actor.handle_add_user_message("second".into(), vec![]);
    actor.handle_fork_at(0);
    assert!(
        actor.session_state.session_tree.is_some(),
        "fork should create session tree"
    );
}
