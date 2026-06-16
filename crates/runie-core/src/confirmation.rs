//! Confirmation router for tool execution.
//!
//! Classifies tools into confirmation kinds and produces the appropriate
//! confirmation UI payload. The TUI or headless mode uses this to decide
//! what UI to show and how to process the approval/rejection.
//!
//! Confirmation kinds:
//!   - `None`        — read-only or already approved, no user action needed
//!   - `Diff`        — file edit: show unified diff + approve/reject
//!   - `Write`       — file write: show path + byte count
//!   - `Bash`        — shell command: show command + confirm/cancel

use crate::edit_preview::EditPreview;
use crate::event::{EditEvent, Event};

/// What kind of user confirmation is required for a tool call.
#[derive(Debug, Clone, PartialEq)]
pub enum ConfirmationKind {
    /// No confirmation needed — the tool is read-only or auto-approved.
    None,
    /// Show a unified diff and ask the user to approve or reject.
    Diff {
        preview: EditPreview,
    },
    /// Show file write details and ask for confirmation.
    Write {
        path: String,
        content: String,
        byte_count: usize,
    },
    /// Show the shell command and ask for confirmation.
    Bash {
        command: String,
        reason: Option<String>,
    },
}

impl ConfirmationKind {
    /// Whether this confirmation type blocks execution until the user responds.
    pub fn is_blocking(&self) -> bool {
        !matches!(self, Self::None)
    }

    /// Human-readable summary for display.
    pub fn summary(&self) -> String {
        match self {
            Self::None => "No confirmation needed".to_string(),
            Self::Diff { preview } => format!("Edit {}", preview.path.display()),
            Self::Write { path, byte_count, .. } => {
                format!("Write {} ({} bytes)", path, byte_count)
            }
            Self::Bash { command, .. } => {
                let preview = if command.len() > 60 {
                    format!("{}...", &command[..60])
                } else {
                    command.clone()
                };
                format!("Run: {}", preview)
            }
        }
    }
}

/// Routes a tool to its confirmation kind.
///
/// The `AgentCommand` fields match the `Tool` enum variants so this can be
/// called without matching the full tool enum — just pass the tool name and
/// relevant fields.
pub struct ConfirmationRouter;

impl ConfirmationRouter {
    /// Route an edit-file tool to `ConfirmationKind::Diff`.
    pub fn for_edit(path: &str, original: &str, proposed: &str) -> ConfirmationKind {
        ConfirmationKind::Diff {
            preview: EditPreview::new(
                std::path::PathBuf::from(path),
                original.to_string(),
                proposed.to_string(),
            ),
        }
    }

    /// Route a write-file tool to `ConfirmationKind::Write`.
    pub fn for_write(path: &str, content: &str) -> ConfirmationKind {
        ConfirmationKind::Write {
            path: path.to_string(),
            content: content.to_string(),
            byte_count: content.len(),
        }
    }

    /// Route a bash tool to `ConfirmationKind::Bash`.
    /// `reason` describes why confirmation is needed (e.g. "non-read-only tool").
    pub fn for_bash(command: &str, reason: Option<&str>) -> ConfirmationKind {
        ConfirmationKind::Bash {
            command: command.to_string(),
            reason: reason.map(String::from),
        }
    }

    /// Route a read-only tool — always `ConfirmationKind::None`.
    pub fn for_read_only() -> ConfirmationKind {
        ConfirmationKind::None
    }

    /// Build the event to emit when the user approves this confirmation kind.
    pub fn approval_event(kind: &ConfirmationKind) -> Option<Event> {
        match kind {
            ConfirmationKind::Diff { .. } => Some(EditEvent::ApproveEdit),
            _ => None,
        }
    }

    /// Build the event to emit when the user rejects this confirmation kind.
    pub fn rejection_event(kind: &ConfirmationKind) -> Option<Event> {
        match kind {
            ConfirmationKind::Diff { .. } => Some(EditEvent::RejectEdit),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_only_returns_none() {
        assert_eq!(ConfirmationRouter::for_read_only(), ConfirmationKind::None);
        assert!(!ConfirmationRouter::for_read_only().is_blocking());
    }

    #[test]
    fn diff_confirmation_contains_preview() {
        let kind = ConfirmationRouter::for_edit(
            "/tmp/foo.txt",
            "hello",
            "hello world",
        );
        match kind {
            ConfirmationKind::Diff { preview } => {
                assert_eq!(preview.path, std::path::PathBuf::from("/tmp/foo.txt"));
                assert_eq!(preview.original, "hello");
                assert_eq!(preview.proposed, "hello world");
            }
            _ => panic!("expected Diff, got {:?}", kind),
        }
    }

    #[test]
    fn write_confirmation_contains_bytes() {
        let kind = ConfirmationRouter::for_write("/tmp/new.txt", "some content");
        match kind {
            ConfirmationKind::Write { path, byte_count, .. } => {
                assert_eq!(path, "/tmp/new.txt");
                assert_eq!(byte_count, 12);
            }
            _ => panic!("expected Write, got {:?}", kind),
        }
    }

    #[test]
    fn bash_confirmation_contains_reason() {
        let kind = ConfirmationRouter::for_bash("rm -rf /tmp/*", Some("dangerous command"));
        match kind {
            ConfirmationKind::Bash { command, reason } => {
                assert_eq!(command, "rm -rf /tmp/*");
                assert_eq!(reason, Some("dangerous command".to_string()));
            }
            _ => panic!("expected Bash, got {:?}", kind),
        }
    }

    #[test]
    fn confirmation_kind_summary() {
        assert_eq!(ConfirmationRouter::for_read_only().summary(), "No confirmation needed");

        let diff = ConfirmationRouter::for_edit("/a.txt", "a", "b");
        assert!(diff.summary().contains("Edit"));

        let write = ConfirmationRouter::for_write("/b.txt", "hi");
        assert!(write.summary().contains("Write"));

        let bash = ConfirmationRouter::for_bash("echo hi", None);
        assert!(bash.summary().contains("echo hi"));
    }

    #[test]
    fn diff_is_blocking() {
        assert!(ConfirmationRouter::for_read_only().is_blocking() == false);
        assert!(ConfirmationRouter::for_edit("/a", "a", "b").is_blocking());
        assert!(ConfirmationRouter::for_write("/a", "x").is_blocking());
        assert!(ConfirmationRouter::for_bash("echo hi", None).is_blocking());
    }

    #[test]
    fn approval_rejection_events() {
        let diff = ConfirmationRouter::for_edit("/a", "a", "b");
        assert_eq!(ConfirmationRouter::approval_event(&diff), Some(EditEvent::ApproveEdit));
        assert_eq!(ConfirmationRouter::rejection_event(&diff), Some(EditEvent::RejectEdit));

        let write = ConfirmationRouter::for_write("/a", "x");
        assert_eq!(ConfirmationRouter::approval_event(&write), None);
        assert_eq!(ConfirmationRouter::rejection_event(&write), None);

        let bash = ConfirmationRouter::for_bash("echo", None);
        assert_eq!(ConfirmationRouter::approval_event(&bash), None);
        assert_eq!(ConfirmationRouter::rejection_event(&bash), None);
    }
}
