//! Renderer-neutral IDE protocol projection.
//! Transport adapters (ACP, JSON-RPC, or editor bridges) emit these events;
//! the reducer owns no sockets and is deterministic under replay.

use std::collections::BTreeMap;

pub const IDE_INVALID_REQUEST_CODE: i64 = -32_600;

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

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct IdeRpcRequest {
    pub jsonrpc: String,
    pub id: u64,
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
    pub id: u64,
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

    #[test]
    fn ide_json_rpc_codec_is_typed_and_lossless() {
        let request = decode_ide_request(
            r#"{"jsonrpc":"2.0","id":7,"method":"textDocument/didOpen","params":{"uri":"file:///a"}}"#,
        )
        .expect("request");
        assert_eq!(request.id, 7);
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
}
