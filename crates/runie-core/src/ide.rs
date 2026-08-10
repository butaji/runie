//! Renderer-neutral IDE protocol projection.
//! Transport adapters (ACP, JSON-RPC, or editor bridges) emit these events;
//! the reducer owns no sockets and is deterministic under replay.

use std::collections::BTreeMap;

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
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct IdeSnapshot {
    pub workspace: Option<String>,
    pub documents: BTreeMap<String, IdeDocument>,
    pub diagnostics: BTreeMap<String, Vec<IdeDiagnostic>>,
}

pub fn reduce_ide_event(snapshot: &mut IdeSnapshot, event: IdeEvent) -> Result<(), String> {
    match event {
        IdeEvent::Initialized { workspace } => {
            if workspace.trim().is_empty() {
                return Err("IDE workspace must not be empty".into());
            }
            snapshot.workspace = Some(workspace);
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
    }
}
