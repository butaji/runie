//! Messages and handle for `IoActor`.

use std::path::PathBuf;

use crate::actors::GenericActorHandle;
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

/// Handle for sending commands to an `IoActor`.
pub type IoActorHandle = GenericActorHandle<IoMsg>;

impl IoActorHandle {
    /// Request running a bash command.
    pub async fn run_bash(&self, command: String) {
        self.send(IoMsg::RunBash { command }).await;
    }

    /// Request writing files.
    pub async fn write_files(&self, edits: Vec<(PathBuf, String)>) {
        self.send(IoMsg::WriteFiles { edits }).await;
    }

    /// Request environment detection (cwd name, git info).
    pub async fn detect_env(&self) {
        self.send(IoMsg::DetectEnv).await;
    }

    /// Request sharing session to gist.
    pub async fn share_session(&self, messages: Vec<ChatMessage>, display_name: Option<String>) {
        self.send(IoMsg::ShareSession { messages, display_name }).await;
    }

    /// Request opening external editor.
    pub async fn open_external_editor(&self, text: String) {
        self.send(IoMsg::OpenExternalEditor { text }).await;
    }

    /// Request clipboard write.
    pub async fn write_clipboard(&self, text: String) {
        self.send(IoMsg::WriteClipboard { text }).await;
    }

    /// Request clipboard read.
    pub async fn read_clipboard(&self) {
        self.send(IoMsg::ReadClipboard).await;
    }

    /// Request process suspend.
    pub async fn suspend_process(&self) {
        self.send(IoMsg::SuspendProcess).await;
    }
}
