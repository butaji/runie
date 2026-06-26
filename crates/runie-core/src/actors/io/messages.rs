//! Messages and handle for `IoActor`.

use std::path::PathBuf;

use crate::actors::GenericActorHandle;

/// Messages accepted by `IoActor`.
#[derive(Debug, Clone)]
pub enum IoMsg {
    /// Run a bash command and publish the output.
    RunBash { command: String },
    /// Write multiple files and publish the result.
    WriteFiles { edits: Vec<(PathBuf, String)> },
    /// Detect environment info (cwd name, git info) asynchronously.
    DetectEnv,
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
}
