use std::collections::BTreeMap;

pub const IDE_INVALID_REQUEST_CODE: i64 = -32_600;
pub const IDE_MAX_FRAME_BYTES: usize = 1024 * 1024;
#[path = "ide_wire.rs"]
mod ide_wire;
pub use ide_wire::IdeWireBuffer;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum IdeRpcId {
    Number(u64),
    String(String),
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct IdeDocument {
    pub uri: String,
    pub language_id: String,
    pub version: u64,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct IdeDiagnostic {
    pub uri: String,
    pub line: u32,
    pub column: u32,
    pub severity: IdeSeverity,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IdeSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdeConnectionStatus {
    #[default]
    Disconnected,
    Connected,
    Reconnecting,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum IdeEvent {
    Initialized {
        workspace: String,
    },
    DocumentOpened(IdeDocument),
    DocumentChanged(IdeDocument),
    DiagnosticsReplaced {
        uri: String,
        items: Vec<IdeDiagnostic>,
    },
    DocumentClosed {
        uri: String,
    },
    ConnectionLost,
    ReconnectStarted,
    ConnectionRestored,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct IdeSnapshot {
    pub workspace: Option<String>,
    pub connection: IdeConnectionStatus,
    pub documents: BTreeMap<String, IdeDocument>,
    pub diagnostics: BTreeMap<String, Vec<IdeDiagnostic>>,
}

enum IdeCommand {
    Apply {
        event: IdeEvent,
        reply: tokio::sync::oneshot::Sender<Result<(), String>>,
    },
    Snapshot {
        reply: tokio::sync::oneshot::Sender<IdeSnapshot>,
    },
}

#[derive(Clone)]
pub struct IdeActor {
    tx: tokio::sync::mpsc::Sender<IdeCommand>,
    _owner: std::sync::Arc<crate::task_owner::TaskOwner>,
}

impl IdeActor {
    pub fn new() -> Self {
        let (tx, owner) =
            crate::spawn_actor_worker!(32, move |mut rx: tokio::sync::mpsc::Receiver<
                IdeCommand,
            >| async move {
                let mut snapshot = IdeSnapshot::default();
                while let Some(command) = rx.recv().await {
                    match command {
                        IdeCommand::Apply { event, reply } => {
                            let _ = reply.send(reduce_ide_event(&mut snapshot, event));
                        }
                        IdeCommand::Snapshot { reply } => {
                            let _ = reply.send(snapshot.clone());
                        }
                    }
                }
            });
        Self { tx, _owner: owner }
    }

    pub async fn apply(&self, event: IdeEvent) -> Result<(), String> {
        let (reply, response) = tokio::sync::oneshot::channel();
        self.tx
            .send(IdeCommand::Apply { event, reply })
            .await
            .map_err(|_| "IDE actor is closed".to_owned())?;
        response
            .await
            .map_err(|_| "IDE actor response was dropped".to_owned())?
    }

    pub async fn apply_rpc(&self, method: &str, params: serde_json::Value) -> Result<(), String> {
        self.apply(ide_event_from_rpc(method, params)?).await
    }

    pub async fn apply_wire_frames(
        &self,
        buffer: &mut IdeWireBuffer,
        bytes: &str,
    ) -> Result<usize, String> {
        let frames = buffer.push(bytes)?;
        for frame in &frames {
            let request = decode_ide_request(frame)?;
            self.apply_rpc(&request.method, request.params).await?;
        }
        Ok(frames.len())
    }

    pub async fn snapshot(&self) -> Result<IdeSnapshot, String> {
        let (reply, response) = tokio::sync::oneshot::channel();
        self.tx
            .send(IdeCommand::Snapshot { reply })
            .await
            .map_err(|_| "IDE actor is closed".to_owned())?;
        response
            .await
            .map_err(|_| "IDE actor snapshot was dropped".to_owned())
    }
}

impl Default for IdeActor {
    fn default() -> Self {
        Self::new()
    }
}

impl IdeSnapshot {
    /// Apply one host-owned JSON-RPC notification through the typed event
    /// boundary. Socket lifecycle remains outside the reducer actor.
    pub fn apply_rpc_notification(&mut self, input: &str) -> Result<(), String> {
        let request = decode_ide_request(input)?;
        let event = ide_event_from_rpc(&request.method, request.params)?;
        reduce_ide_event(self, event)
    }

    pub fn terminal_lines(&self) -> Vec<String> {
        vec![
            format!("connection: {:?}", self.connection),
            format!("workspace: {}", self.workspace.as_deref().unwrap_or("none")),
            format!("documents: {}", self.documents.len()),
            format!(
                "diagnostics: {}",
                self.diagnostics.values().map(Vec::len).sum::<usize>()
            ),
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct IdeRpcRequest {
    pub jsonrpc: String,
    pub id: IdeRpcId,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct IdeRpcError {
    pub code: i64,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct IdeRpcResponse {
    pub jsonrpc: String,
    pub id: IdeRpcId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<IdeRpcError>,
}

pub fn decode_ide_request(input: &str) -> Result<IdeRpcRequest, String> {
    let request: IdeRpcRequest = serde_json::from_str(input)
        .map_err(|error| format!("invalid IDE JSON-RPC request: {error}"))?;
    if request.jsonrpc != "2.0" || request.method.trim().is_empty() {
        return Err("IDE request must use JSON-RPC 2.0 and a method".into());
    }
    Ok(request)
}

pub fn encode_ide_response(response: &IdeRpcResponse) -> Result<String, String> {
    if response.result.is_some() == response.error.is_some() {
        return Err("IDE response must contain exactly one result or error".into());
    }
    serde_json::to_string(response).map_err(|error| format!("encode IDE response: {error}"))
}

/// Convert common LSP/ACP notifications into the replayable IDE event model.
pub fn ide_event_from_rpc(method: &str, params: serde_json::Value) -> Result<IdeEvent, String> {
    match method {
        "initialized" => Ok(IdeEvent::Initialized {
            workspace: params
                .get("workspace")
                .and_then(serde_json::Value::as_str)
                .ok_or("IDE initialized notification is missing workspace")?
                .into(),
        }),
        "textDocument/didOpen" => Ok(IdeEvent::DocumentOpened(document_from_params(&params)?)),
        "textDocument/didChange" => Ok(IdeEvent::DocumentChanged(document_from_params(&params)?)),
        "textDocument/didClose" => Ok(IdeEvent::DocumentClosed {
            uri: params
                .get("textDocument")
                .and_then(|value| value.get("uri"))
                .and_then(serde_json::Value::as_str)
                .ok_or("IDE close notification is missing document URI")?
                .into(),
        }),
        _ => Err(format!("unsupported IDE notification: {method}")),
    }
}

fn document_from_params(value: &serde_json::Value) -> Result<IdeDocument, String> {
    let text_document = value.get("textDocument").unwrap_or(value);
    Ok(IdeDocument {
        uri: text_document
            .get("uri")
            .and_then(serde_json::Value::as_str)
            .ok_or("IDE notification is missing document URI")?
            .into(),
        language_id: text_document
            .get("languageId")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("text")
            .into(),
        version: text_document
            .get("version")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or_default(),
        text: text_document
            .get("text")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .into(),
    })
}

pub fn reduce_ide_event(snapshot: &mut IdeSnapshot, event: IdeEvent) -> Result<(), String> {
    match event {
        IdeEvent::Initialized { workspace } => {
            if workspace.trim().is_empty() {
                return Err("IDE workspace must not be empty".into());
            }
            snapshot.workspace = Some(workspace);
            snapshot.connection = IdeConnectionStatus::Connected;
        }
        IdeEvent::DocumentOpened(document) | IdeEvent::DocumentChanged(document) => {
            validate_document(&document)?;
            snapshot.documents.insert(document.uri.clone(), document);
        }
        IdeEvent::DiagnosticsReplaced { uri, items } => {
            if uri.trim().is_empty() || items.iter().any(|item| item.uri != uri) {
                return Err("IDE diagnostics must target their document URI".into());
            }
            snapshot.diagnostics.insert(uri, items);
        }
        IdeEvent::DocumentClosed { uri } => {
            snapshot.documents.remove(&uri);
            snapshot.diagnostics.remove(&uri);
        }
        IdeEvent::ConnectionLost => snapshot.connection = IdeConnectionStatus::Disconnected,
        IdeEvent::ReconnectStarted => snapshot.connection = IdeConnectionStatus::Reconnecting,
        IdeEvent::ConnectionRestored => {
            if snapshot.workspace.is_none() {
                return Err("IDE connection cannot restore before initialization".into());
            }
            snapshot.connection = IdeConnectionStatus::Connected;
        }
    }
    Ok(())
}

fn validate_document(document: &IdeDocument) -> Result<(), String> {
    if document.uri.trim().is_empty() {
        return Err("IDE document URI must not be empty".into());
    }
    if document.language_id.trim().is_empty() {
        return Err("IDE document language must not be empty".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
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
            vec![
                "connection: Connected",
                "workspace: /workspace",
                "documents: 1",
                "diagnostics: 1",
            ]
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
        assert_eq!(snapshot.workspace.as_deref(), Some("/workspace"));
        actor
            .apply_rpc(
                "textDocument/didOpen",
                serde_json::json!({
                    "textDocument": {
                        "uri": "file:///main.rs",
                        "languageId": "rust",
                        "version": 1,
                        "text": "fn main() {}"
                    }
                }),
            )
            .await
            .unwrap();
        assert_eq!(actor.snapshot().await.unwrap().documents.len(), 1);
    }

    #[test]
    fn ide_wire_buffer_replays_split_and_multiple_frames() {
        let mut buffer = IdeWireBuffer::default();
        let first =
            r#"{"jsonrpc":"2.0","id":1,"method":"initialized","params":{"workspace":"/w"}}"#;
        let second = r#"{"jsonrpc":"2.0","id":2,"method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///a","languageId":"rust","version":1,"text":"fn main() {}"}}}"#;
        assert!(buffer.push(&first[..20]).unwrap().is_empty());
        let frames = buffer
            .push(&format!("{}\n{}\n", &first[20..], second))
            .unwrap();
        assert_eq!(frames, [first, second]);
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
        .expect("request");
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
        .expect("response");
        assert!(encoded.contains(&IDE_INVALID_REQUEST_CODE.to_string()));
        let string_id =
            decode_ide_request(r#"{"jsonrpc":"2.0","id":"editor-7","method":"shutdown"}"#)
                .expect("string id");
        assert_eq!(string_id.id, IdeRpcId::String("editor-7".into()));
    }

    #[test]
    fn lsp_notifications_project_into_replayable_document_events() {
        let event = ide_event_from_rpc(
            "textDocument/didOpen",
            serde_json::json!({"textDocument":{"uri":"file:///a","languageId":"rust","version":2,"text":"fn main() {}"}}),
        )
        .unwrap();
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
        snapshot
            .apply_rpc_notification(
                r#"{"jsonrpc":"2.0","id":2,"method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///main.rs","languageId":"rust","version":1,"text":"fn main() {}"}}}"#,
            )
            .unwrap();
        assert_eq!(snapshot.workspace.as_deref(), Some("/repo"));
        assert_eq!(snapshot.documents.len(), 1);
    }
}
