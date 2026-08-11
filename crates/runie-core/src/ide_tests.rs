use super::*;

#[test]
fn replayed_ide_events_reduce_to_documents_and_diagnostics() {
    let events = [
        IdeEvent::Initialized {
            workspace: "/workspace".into(),
        },
        IdeEvent::DocumentOpened(IdeDocument {
            uri: "file:///main.rs".into(),
            language_id: "rust".into(),
            version: 1,
            text: "fn main() {}".into(),
        }),
        IdeEvent::DiagnosticsReplaced {
            uri: "file:///main.rs".into(),
            items: vec![IdeDiagnostic {
                uri: "file:///main.rs".into(),
                line: 0,
                column: 3,
                severity: IdeSeverity::Warning,
                message: "unused".into(),
            }],
        },
    ];
    let mut snapshot = IdeSnapshot::default();
    for event in events {
        reduce_ide_event(&mut snapshot, event).expect("valid IDE event");
    }
    assert_eq!(snapshot.documents.len(), 1);
    assert_eq!(snapshot.diagnostics["file:///main.rs"].len(), 1);
    assert_eq!(
        snapshot.terminal_lines(),
        [
            "connection: Connected",
            "workspace: /workspace",
            "documents: 1",
            "diagnostics: 1",
            "diagnostic: file:///main.rs:0:3 · Warning · unused"
        ]
    );
}

#[test]
fn diagnostic_rows_are_bounded_serializable_data() {
    let mut snapshot = IdeSnapshot::default();
    reduce_ide_event(
        &mut snapshot,
        IdeEvent::DiagnosticsReplaced {
            uri: "file:///main.rs".into(),
            items: vec![IdeDiagnostic {
                uri: "file:///main.rs".into(),
                line: 4,
                column: 2,
                severity: IdeSeverity::Error,
                message: "x".repeat(IDE_DIAGNOSTIC_MESSAGE_MAX_CHARS + 1),
            }],
        },
    )
    .unwrap();
    let rows = snapshot.diagnostic_rows();
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].message.chars().count(),
        IDE_DIAGNOSTIC_MESSAGE_MAX_CHARS
    );
    assert!(
        serde_json::from_value::<IdeDiagnosticRow>(serde_json::to_value(&rows[0]).unwrap()).is_ok()
    );
}

#[test]
fn invalid_ide_events_are_rejected_without_partial_state() {
    let mut snapshot = IdeSnapshot::default();
    assert!(reduce_ide_event(
        &mut snapshot,
        IdeEvent::Initialized {
            workspace: " ".into()
        }
    )
    .is_err());
    assert!(snapshot.workspace.is_none());
    assert_eq!(snapshot.connection, IdeConnectionStatus::Disconnected);
}

#[test]
fn ide_connection_lifecycle_is_replayable_data() {
    let mut snapshot = IdeSnapshot::default();
    reduce_ide_event(
        &mut snapshot,
        IdeEvent::Initialized {
            workspace: "/workspace".into(),
        },
    )
    .unwrap();
    reduce_ide_event(&mut snapshot, IdeEvent::ConnectionLost).unwrap();
    assert_eq!(snapshot.connection, IdeConnectionStatus::Disconnected);
    reduce_ide_event(&mut snapshot, IdeEvent::ReconnectStarted).unwrap();
    assert_eq!(snapshot.connection, IdeConnectionStatus::Reconnecting);
    reduce_ide_event(&mut snapshot, IdeEvent::ConnectionRestored).unwrap();
    assert_eq!(snapshot.connection, IdeConnectionStatus::Connected);
}

#[tokio::test]
async fn ide_actor_reduces_events_and_returns_owned_snapshot() {
    let actor = IdeActor::new();
    actor
        .apply(IdeEvent::Initialized {
            workspace: "/workspace".into(),
        })
        .await
        .unwrap();
    let snapshot = actor.snapshot().await.unwrap();
    assert_eq!(snapshot.connection, IdeConnectionStatus::Connected);
    actor.apply_rpc("textDocument/didOpen", serde_json::json!({"textDocument":{"uri":"file:///main.rs","languageId":"rust","version":1,"text":"fn main() {}"}})).await.unwrap();
    assert_eq!(actor.snapshot().await.unwrap().documents.len(), 1);
}

#[test]
fn ide_wire_buffer_replays_split_and_multiple_frames() {
    let mut buffer = IdeWireBuffer::default();
    let first = r#"{"jsonrpc":"2.0","id":1,"method":"initialized","params":{"workspace":"/w"}}"#;
    let second = r#"{"jsonrpc":"2.0","id":2,"method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///a","languageId":"rust","version":1,"text":"fn main() {}"}}}"#;
    assert!(buffer.push(&first[..20]).unwrap().is_empty());
    assert_eq!(
        buffer
            .push(&format!("{}\n{}\n", &first[20..], second))
            .unwrap(),
        [first, second]
    );
    assert_eq!(buffer.pending_bytes(), 0);
}

#[test]
fn ide_wire_buffer_rejects_unbounded_incomplete_frames() {
    let mut buffer = IdeWireBuffer::default();
    let error = buffer
        .push(&"x".repeat(IDE_MAX_FRAME_BYTES + 1))
        .unwrap_err();
    assert!(error.contains("bounded byte limit"));
}

#[test]
fn ide_json_rpc_codec_is_typed_and_lossless() {
    let request = decode_ide_request(
        r#"{"jsonrpc":"2.0","id":7,"method":"textDocument/didOpen","params":{"uri":"file:///a"}}"#,
    )
    .unwrap();
    assert_eq!(request.id, IdeRpcId::Number(7));
    let encoded = encode_ide_response(&IdeRpcResponse {
        jsonrpc: "2.0".into(),
        id: request.id,
        result: None,
        error: Some(IdeRpcError {
            code: IDE_INVALID_REQUEST_CODE,
            message: "invalid request".into(),
        }),
    })
    .unwrap();
    assert!(encoded.contains(&IDE_INVALID_REQUEST_CODE.to_string()));
    assert_eq!(
        decode_ide_request(r#"{"jsonrpc":"2.0","id":"editor-7","method":"shutdown"}"#)
            .unwrap()
            .id,
        IdeRpcId::String("editor-7".into())
    );
}

#[test]
fn lsp_diagnostics_notification_becomes_typed_event_data() {
    let event = ide_event_from_rpc("textDocument/publishDiagnostics", serde_json::json!({"uri":"file:///a","diagnostics":[{"range":{"start":{"line":3,"character":7}},"severity":2,"message":"unused import"}]})).unwrap();
    let IdeEvent::DiagnosticsReplaced { items, .. } = event else {
        panic!("expected diagnostics event")
    };
    assert_eq!(items[0].severity, IdeSeverity::Warning);
    assert_eq!((items[0].line, items[0].column), (3, 7));
}

#[test]
fn malformed_lsp_diagnostics_are_rejected_at_rpc_boundary() {
    assert!(ide_event_from_rpc(
        "textDocument/publishDiagnostics",
        serde_json::json!({"uri":"file:///a","diagnostics":[{"message":"bad"}]})
    )
    .is_err());
}

#[test]
fn lsp_notifications_project_into_replayable_document_events() {
    let event = ide_event_from_rpc("textDocument/didOpen", serde_json::json!({"textDocument":{"uri":"file:///a","languageId":"rust","version":2,"text":"fn main() {}"}})).unwrap();
    let mut snapshot = IdeSnapshot::default();
    reduce_ide_event(&mut snapshot, event).unwrap();
    assert_eq!(snapshot.documents["file:///a"].version, 2);
    assert!(ide_event_from_rpc("workspace/unknown", serde_json::json!({})).is_err());
}

#[test]
fn rpc_notification_adapter_replays_through_one_snapshot_boundary() {
    let mut snapshot = IdeSnapshot::default();
    snapshot
        .apply_rpc_notification(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialized","params":{"workspace":"/repo"}}"#,
        )
        .unwrap();
    snapshot.apply_rpc_notification(r#"{"jsonrpc":"2.0","id":2,"method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///main.rs","languageId":"rust","version":1,"text":"fn main() {}"}}}"#).unwrap();
    assert_eq!(snapshot.workspace.as_deref(), Some("/repo"));
    assert_eq!(snapshot.documents.len(), 1);
}

#[tokio::test]
async fn ide_actor_applies_split_wire_frames_through_owned_snapshot() {
    let actor = IdeActor::new();
    let mut buffer = IdeWireBuffer::default();
    let initialized =
        r#"{"jsonrpc":"2.0","id":1,"method":"initialized","params":{"workspace":"/repo"}}"#;
    let diagnostics = r#"{"jsonrpc":"2.0","id":2,"method":"textDocument/publishDiagnostics","params":{"uri":"file:///main.rs","diagnostics":[{"range":{"start":{"line":1,"character":2}},"message":"error"}]}}"#;
    assert_eq!(
        actor
            .apply_wire_frames(&mut buffer, &initialized[..18])
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        actor
            .apply_wire_frames(
                &mut buffer,
                &format!("{}\n{}\n", &initialized[18..], diagnostics)
            )
            .await
            .unwrap(),
        2
    );
    let snapshot = actor.snapshot().await.unwrap();
    assert_eq!(snapshot.workspace.as_deref(), Some("/repo"));
    assert_eq!(
        snapshot.diagnostics["file:///main.rs"][0].severity,
        IdeSeverity::Info
    );
}
