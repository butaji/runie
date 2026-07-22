//! Messages and handle for `IoActor`.

use std::path::PathBuf;

use crate::ChatMessage;

/// Messages accepted by `IoActor`.
#[derive(Debug, Clone)]
pub enum IoMsg {
    /// Run a bash command and publish the output.
    ///
    /// If `shell` is false (default), the command is parsed with `shell_words::split`
    /// and executed directly without a shell wrapper. This avoids shell indirection
    /// overhead and security risks for simple commands.
    ///
    /// If `shell` is true, the command is passed to `sh -c` to support shell
    /// metacharacters (pipes, redirects, command substitution, etc.).
    RunBash { command: String, shell: bool },
    /// Write multiple files and publish the result.
    WriteFiles { edits: Vec<(PathBuf, String)> },
    /// Detect environment info (cwd name, git info) asynchronously.
    DetectEnv,
    /// Share session messages to a GitHub gist.
    ShareSession { messages: Vec<ChatMessage>, display_name: Option<String> },
    /// Open external editor with text, return edited text.
    OpenExternalEditor { text: String },
    /// Copy text to clipboard.
    #[cfg(feature = "clipboard")]
    WriteClipboard { text: String },
    /// Read text from clipboard.
    #[cfg(feature = "clipboard")]
    ReadClipboard,
    /// Suspend/resume the process.
    SuspendProcess,
    /// Load skills from disk and emit a SkillsLoaded event.
    LoadSkills,
    /// Load auth storage and emit an AuthLoaded event.
    LoadAuth,
}
