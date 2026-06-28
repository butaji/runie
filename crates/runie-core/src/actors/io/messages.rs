//! Messages and handle for `IoActor`.

use std::path::PathBuf;

use crate::ChatMessage;

/// Messages accepted by `IoActor`.
#[derive(Debug, Clone)]
pub enum IoMsg {
    /// Run a bash command and publish the output.
    RunBash { command: String },
    /// Write multiple files and publish the result.
    WriteFiles { edits: Vec<(PathBuf, String)> },
    /// Detect environment info (cwd name, git info) asynchronously.
    DetectEnv,
    /// Share session messages to a GitHub gist.
    ShareSession {
        messages: Vec<ChatMessage>,
        display_name: Option<String>,
    },
    /// Open external editor with text, return edited text.
    OpenExternalEditor { text: String },
    /// Copy text to clipboard.
    WriteClipboard { text: String },
    /// Read text from clipboard.
    ReadClipboard,
    /// Suspend/resume the process.
    SuspendProcess,
}


