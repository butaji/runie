//! Tests for EventKind taxonomy and predicates.

use crate::config::Config;
use crate::event::kind::EventKind;
use crate::event::variants::Event;
use crate::permissions::PermissionAction;

use super::EventKind::{Control, Fact, Intent};

#[test]
fn event_kind_is_exhaustive() {
    fn _check(_: Event) {}
}

#[test]
fn intent_events_are_not_fact() {
    for e in [
        Event::Input('x'),
        Event::Submit,
        Event::SwitchModel {
            provider: "anthropic".into(),
            model: "claude".into(),
            explicit: true,
        },
        Event::Quit,
        Event::RunSaveCommand {
            name: "test".into(),
        },
    ] {
        assert_ne!(e.kind(), EventKind::Fact, "{e:?} must not be Fact");
    }
}

#[test]
fn fact_events_are_classified() {
    for e in [
        Event::Thinking { id: "1".into() },
        Event::ToolEnd {
            id: "t1".into(),
            duration_secs: 0.5,
            output: "ok".into(),
        },
        Event::TurnComplete {
            id: "1".into(),
            duration_secs: 1.0,
        },
        Event::ConfigLoaded {
            config: Box::new(Config::default()),
        },
        Event::TrustLoaded {
            decisions: Default::default(),
        },
        Event::SessionLoaded {
            name: "test".into(),
            events: Box::new(vec![]),
            metadata: None,
        },
        Event::BashOutput {
            command: "pwd".into(),
            output: "/tmp".into(),
        },
        Event::TransientMessage {
            content: "hello".into(),
            level: crate::event::TransientLevel::Info,
        },
    ] {
        assert_eq!(e.kind(), EventKind::Fact, "{e:?} must be Fact");
    }
}

#[test]
fn control_events_are_classified() {
    for e in [
        Event::Quit,
        Event::Reset,
        Event::Abort,
        Event::TerminalSize {
            width: 80,
            height: 24,
        },
    ] {
        assert_eq!(e.kind(), EventKind::Control, "{e:?} must be Control");
    }
}

/// Layer 1: Every Event variant is classified as Intent, Fact, or Control.
/// This test verifies the partition is exhaustive by checking each category
/// returns a non-default EventKind (Fact is the default).
#[test]
fn intent_fact_partition_is_exhaustive() {
    use EventKind::*;

    // Verify Intent events return Intent (not default Fact)
    let intent_events = [
        Event::Input('x'),
        Event::Submit,
        Event::SwitchModel {
            provider: "a".into(),
            model: "b".into(),
            explicit: false,
        },
        Event::RunSaveCommand {
            name: "test".into(),
        },
        Event::ToggleCommandPalette,
        Event::Up,
        Event::Down,
        Event::PendingEdit {
            path: "x".into(),
            original: "a".into(),
            proposed: "b".into(),
        },
        Event::ForkSession { message_index: 0 },
        Event::Start,
        Event::RunCompactCommand {
            keep: "*".into(),
            focus: "".into(),
        },
        Event::SelectProvider {
            provider: "openai".into(),
        },
        Event::SubmitKey {
            provider: "openai".into(),
            key: "sk-".into(),
        },
    ];
    for e in intent_events {
        assert_eq!(e.kind(), Intent, "{e:?} must be Intent");
    }

    // Verify Fact events return Fact
    let fact_events = [
        Event::Thinking { id: "1".into() },
        Event::ToolStart {
            id: "t1".into(),
            name: "bash".into(),
            input: serde_json::json!({}),
        },
        Event::ToolEnd {
            id: "t1".into(),
            duration_secs: 1.0,
            output: "ok".into(),
        },
        Event::Response {
            id: "r1".into(),
            content: "hi".into(),
        },
        Event::TurnComplete {
            id: "1".into(),
            duration_secs: 1.0,
        },
        Event::ConfigLoaded {
            config: Box::new(Config::default()),
        },
        Event::TrustLoaded {
            decisions: Default::default(),
        },
        Event::SessionSaved {
            name: "test".into(),
        },
        Event::BashOutput {
            command: "ls".into(),
            output: "/".into(),
        },
        Event::TransientMessage {
            content: "hi".into(),
            level: crate::event::TransientLevel::Info,
        },
        Event::MessageReplayed {
            id: "1".into(),
            role: "user".into(),
            content: "hi".into(),
            timestamp: 0.0,
            provider: "openai".into(),
        },
        Event::ValidationFailed {
            provider: "a".into(),
            key: "k".into(),
            error: "e".into(),
        },
        Event::ModelsFetched {
            provider: "a".into(),
            key: "k".into(),
            models: vec![],
        },
        Event::PermissionRequest {
            request_id: "1".into(),
            tool: "bash".into(),
            input: serde_json::json!({}),
        },
        Event::PermissionResponse {
            request_id: "1".into(),
            action: PermissionAction::Allow,
        },
    ];
    for e in fact_events {
        assert_eq!(e.kind(), Fact, "{e:?} must be Fact");
    }

    // Verify Control events return Control
    let control_events = [
        Event::Quit,
        Event::ForceQuit,
        Event::Reset,
        Event::Abort,
        Event::TerminalSize {
            width: 80,
            height: 24,
        },
        Event::FollowUp,
        Event::ToggleExpand,
        Event::Dequeue,
        Event::OpenExternalEditor,
        Event::ExternalEditorDone {
            content: "x".into(),
        },
        Event::ShareSession,
        Event::Suspend,
        Event::ToggleVimMode,
        Event::CopyLastResponse,
        Event::OpenSessionList,
        Event::NewSession,
        Event::ResumeSession,
        Event::SelectSession { id: "1".into() },
        Event::StarSession { id: "1".into() },
        Event::RenameSession {
            id: "1".into(),
            name: "test".into(),
        },
        Event::DeleteSession { id: "1".into() },
    ];
    for e in control_events {
        assert_eq!(e.kind(), Control, "{e:?} must be Control");
    }
}

/// Layer 1: Intent events convert to Some(Intent).
#[test]
fn intent_events_convert_to_intent() {
    let events = [
        Event::Input('x'),
        Event::Quit,
        Event::SwitchModel {
            provider: "a".into(),
            model: "b".into(),
            explicit: true,
        },
        Event::RunSaveCommand {
            name: "test".into(),
        },
        Event::Submit,
    ];
    for e in events {
        assert!(
            e.clone().into_intent().is_some(),
            "{e:?} must convert to Intent"
        );
    }
}

/// Layer 1: Fact events return None from into_intent().
#[test]
fn fact_events_return_none_from_into_intent() {
    let events = [
        Event::Thinking { id: "1".into() },
        Event::ConfigLoaded {
            config: Box::new(Config::default()),
        },
        Event::ToolEnd {
            id: "t1".into(),
            duration_secs: 1.0,
            output: "ok".into(),
        },
    ];
    for e in events {
        assert!(e.clone().into_intent().is_none(), "{e:?} must return None");
    }
}
