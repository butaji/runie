use std::collections::BTreeMap;

pub const IDE_INVALID_REQUEST_CODE: i64 = -32_600;
pub const IDE_MAX_FRAME_BYTES: usize = 1024 * 1024;
const IDE_DIAGNOSTIC_MESSAGE_MAX_CHARS: usize = 512;
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

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct IdeDiagnosticRow {
    pub uri: String,
    pub line: u32,
    pub column: u32,
    pub severity: IdeSeverity,
    pub message: String,
}

impl IdeDiagnosticRow {
    pub fn terminal_line(&self) -> String {
        format!(
            "{}:{}:{} · {:?} · {}",
            self.uri, self.line, self.column, self.severity, self.message
        )
    }
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

include!("ide_transport.inc");

impl IdeSnapshot {
    /// Apply one host-owned JSON-RPC notification through the typed event
    /// boundary. Socket lifecycle remains outside the reducer actor.
    pub fn apply_rpc_notification(&mut self, input: &str) -> Result<(), String> {
        let request = decode_ide_request(input)?;
        let event = ide_event_from_rpc(&request.method, request.params)?;
        reduce_ide_event(self, event)
    }

    pub fn terminal_lines(&self) -> Vec<String> {
        let mut lines = vec![
            format!("connection: {:?}", self.connection),
            format!("workspace: {}", self.workspace.as_deref().unwrap_or("none")),
            format!("documents: {}", self.documents.len()),
            format!(
                "diagnostics: {}",
                self.diagnostics.values().map(Vec::len).sum::<usize>()
            ),
        ];
        lines.extend(
            self.diagnostic_rows()
                .into_iter()
                .map(|row| format!("diagnostic: {}", row.terminal_line())),
        );
        lines
    }

    pub fn diagnostic_rows(&self) -> Vec<IdeDiagnosticRow> {
        self.diagnostics
            .values()
            .flatten()
            .map(|diagnostic| IdeDiagnosticRow {
                uri: diagnostic.uri.clone(),
                line: diagnostic.line,
                column: diagnostic.column,
                severity: diagnostic.severity,
                message: diagnostic
                    .message
                    .chars()
                    .take(IDE_DIAGNOSTIC_MESSAGE_MAX_CHARS)
                    .collect(),
            })
            .collect()
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
pub struct IdeRpcNotification {
    pub jsonrpc: String,
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

pub fn encode_ide_notification(method: &str, params: serde_json::Value) -> Result<String, String> {
    if method.trim().is_empty() {
        return Err("IDE notification method must not be empty".into());
    }
    serde_json::to_string(&IdeRpcNotification {
        jsonrpc: "2.0".into(),
        method: method.into(),
        params,
    })
    .map(|value| format!("{value}\n"))
    .map_err(|error| format!("encode IDE notification: {error}"))
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
        "textDocument/publishDiagnostics" => diagnostics_from_params(&params),
        _ => Err(format!("unsupported IDE notification: {method}")),
    }
}

fn diagnostics_from_params(value: &serde_json::Value) -> Result<IdeEvent, String> {
    let uri = value
        .get("uri")
        .and_then(serde_json::Value::as_str)
        .filter(|uri| !uri.trim().is_empty())
        .ok_or("IDE diagnostics notification is missing URI")?;
    let items = value
        .get("diagnostics")
        .and_then(serde_json::Value::as_array)
        .ok_or("IDE diagnostics notification is missing diagnostics")?
        .iter()
        .map(|item| {
            let start = item
                .get("range")
                .and_then(|range| range.get("start"))
                .ok_or("IDE diagnostic is missing range start")?;
            Ok(IdeDiagnostic {
                uri: uri.into(),
                line: start
                    .get("line")
                    .and_then(serde_json::Value::as_u64)
                    .ok_or("IDE diagnostic is missing line")? as u32,
                column: start
                    .get("character")
                    .and_then(serde_json::Value::as_u64)
                    .ok_or("IDE diagnostic is missing character")? as u32,
                severity: ide_severity(item.get("severity").and_then(serde_json::Value::as_u64)),
                message: item
                    .get("message")
                    .and_then(serde_json::Value::as_str)
                    .ok_or("IDE diagnostic is missing message")?
                    .into(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(IdeEvent::DiagnosticsReplaced {
        uri: uri.into(),
        items,
    })
}

fn ide_severity(value: Option<u64>) -> IdeSeverity {
    match value {
        Some(1) => IdeSeverity::Error,
        Some(2) => IdeSeverity::Warning,
        Some(4) => IdeSeverity::Hint,
        _ => IdeSeverity::Info,
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
#[path = "ide_tests.rs"]
mod tests;
