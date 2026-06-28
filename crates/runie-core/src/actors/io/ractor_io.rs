//! Ractor-based `IoActor` implementation.
//!
//! Migrated from custom Actor trait to ractor for consistency with the rest
//! of the actor system.

use std::path::{Path, PathBuf};
use std::process::Command;

use ractor::{Actor, ActorProcessingErr, ActorRef};
use ractor::async_trait;

use crate::actors::ractor_adapter::{spawn_ractor, EventBusBridge, RactorHandle};
use crate::bus::EventBus;
use crate::ChatMessage;
use crate::event::Event;
use crate::snapshot::GitInfo;

use super::messages::IoMsg;

/// Ractor-based IoActor handle.
#[derive(Clone, Debug)]
pub struct RactorIoHandle {
    inner: RactorHandle<IoMsg>,
}

impl RactorIoHandle {
    /// Create a new handle wrapping the inner RactorHandle.
    pub fn new(inner: RactorHandle<IoMsg>) -> Self {
        Self { inner }
    }

    /// Request running a bash command.
    pub async fn run_bash(&self, command: String) {
        self.inner.send(IoMsg::RunBash { command }).await;
    }

    /// Request writing files.
    pub async fn write_files(&self, edits: Vec<(PathBuf, String)>) {
        self.inner.send(IoMsg::WriteFiles { edits }).await;
    }

    /// Request environment detection.
    pub async fn detect_env(&self) {
        self.inner.send(IoMsg::DetectEnv).await;
    }

    /// Request sharing session to gist.
    pub async fn share_session(&self, messages: Vec<ChatMessage>, display_name: Option<String>) {
        self.inner.send(IoMsg::ShareSession { messages, display_name }).await;
    }

    /// Request opening external editor.
    pub async fn open_external_editor(&self, text: String) {
        self.inner.send(IoMsg::OpenExternalEditor { text }).await;
    }

    /// Request clipboard write.
    pub async fn write_clipboard(&self, text: String) {
        self.inner.send(IoMsg::WriteClipboard { text }).await;
    }

    /// Request clipboard read.
    pub async fn read_clipboard(&self) {
        self.inner.send(IoMsg::ReadClipboard).await;
    }

    /// Request process suspend.
    pub async fn suspend_process(&self) {
        self.inner.send(IoMsg::SuspendProcess).await;
    }
}

/// Ractor-based IoActor.
pub struct RactorIoActor {
    bus_bridge: EventBusBridge<Event>,
}

impl RactorIoActor {
    fn new(bus: EventBus<Event>) -> Self {
        Self {
            bus_bridge: EventBusBridge::new(bus),
        }
    }

    /// Spawn a `RactorIoActor` on the given event bus.
    pub async fn spawn(bus: EventBus<Event>) -> Result<(RactorIoHandle, ractor::ActorCell), ractor::SpawnErr> {
        let actor = Self::new(bus.clone());
        let (handle, _join, cell) = spawn_ractor(None, actor, bus).await?;
        Ok((RactorIoHandle::new(handle), cell))
    }
}

#[async_trait]
impl Actor for RactorIoActor {
    type Msg = IoMsg;
    type State = ();
    type Arguments = EventBus<Event>;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        _args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(())
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        msg: Self::Msg,
        _state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match msg {
            IoMsg::RunBash { command } => self.run_bash(command).await,
            IoMsg::WriteFiles { edits } => self.write_files(edits).await,
            IoMsg::DetectEnv => self.detect_env().await,
            IoMsg::ShareSession { messages, display_name } => {
                self.share_session(messages, display_name).await;
            }
            IoMsg::OpenExternalEditor { text } => {
                self.open_external_editor(text).await;
            }
            IoMsg::WriteClipboard { text } => self.write_clipboard(text).await,
            IoMsg::ReadClipboard => self.read_clipboard().await,
            IoMsg::SuspendProcess => self.suspend_process().await,
        }
        Ok(())
    }
}

impl RactorIoActor {
    async fn run_bash(&self, command: String) {
        let cmd = command.clone();
        let output = match tokio::task::spawn_blocking(move || run_bash_sync(&cmd)).await {
            Ok(out) => out,
            Err(e) => format!("Error running command: {}", e),
        };
        self.emit(Event::BashOutput { command, output });
    }

    async fn write_files(&self, edits: Vec<(PathBuf, String)>) {
        let (count, errors) =
            match tokio::task::spawn_blocking(move || write_files_sync(&edits)).await {
                Ok(res) => res,
                Err(e) => (0, vec![format!("write task failed: {e}")]),
            };
        self.emit(Event::FilesWritten { count, errors });
    }

    async fn detect_env(&self) {
        let (git_info, cwd_name) = tokio::task::spawn_blocking(detect_env_sync)
            .await
            .unwrap_or_default();
        self.emit(Event::EnvDetected { git_info, cwd_name });
    }

    async fn share_session(&self, messages: Vec<ChatMessage>, display_name: Option<String>) {
        let messages_clone = messages.clone();
        let name_clone = display_name.clone();
        let result = tokio::task::spawn_blocking(move || {
            super::effects::share_session_sync(&messages_clone, name_clone.as_deref())
        })
        .await
        .unwrap_or_else(|e| Err(format!("join error: {}", e)));
        self.emit(Event::GistShared { result });
    }

    async fn open_external_editor(&self, text: String) {
        let result = tokio::task::spawn_blocking(move || super::effects::open_editor_sync(text))
            .await
            .unwrap_or_else(|e| Err(e.to_string()));
        self.emit(Event::ExternalEditorClosed { result });
    }

    async fn write_clipboard(&self, text: String) {
        let success = tokio::task::spawn_blocking(move || super::effects::write_clipboard_sync(&text))
            .await
            .unwrap_or(false);
        self.emit(Event::ClipboardWritten { success });
    }

    async fn read_clipboard(&self) {
        let result = tokio::task::spawn_blocking(super::effects::read_clipboard_sync)
            .await
            .unwrap_or_else(|e| Err(e.to_string()));
        self.emit(Event::ClipboardRead { result });
    }

    #[cfg(unix)]
    async fn suspend_process(&self) {
        let bus = self.bus_bridge.clone();
        tokio::task::spawn_blocking(move || {
            let _ = crossterm::execute!(
                std::io::stdout(),
                crossterm::terminal::LeaveAlternateScreen,
            );
            let _ = crossterm::terminal::disable_raw_mode();
            let _ = nix::sys::signal::kill(
                nix::unistd::Pid::this(),
                nix::sys::signal::Signal::SIGTSTP,
            );
            let _ = crossterm::terminal::enable_raw_mode();
            let _ = crossterm::execute!(
                std::io::stdout(),
                crossterm::terminal::EnterAlternateScreen,
            );
            let _ = bus.publish(Event::ProcessResumed);
        });
    }

    #[cfg(not(unix))]
    async fn suspend_process(&self) {}

    fn emit(&self, event: Event) {
        self.bus_bridge.publish(event);
    }
}

// Re-use the sync helper functions from the legacy actor
fn run_bash_sync(command: &str) -> String {
    let output = match Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
    {
        Ok(out) => out,
        Err(e) => return format!("Error running command: {}", e),
    };

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap_or(-1);

    format_command_output(&stdout, &stderr, exit_code)
}

fn format_command_output(stdout: &str, stderr: &str, exit_code: i32) -> String {
    let mut result = String::new();
    if !stdout.is_empty() {
        result.push_str(stdout);
    }
    if !stderr.is_empty() {
        if !result.is_empty() {
            result.push('\n');
        }
        result.push_str("stderr: ");
        result.push_str(stderr);
    }
    if result.is_empty() {
        result = format!("(exit code: {})", exit_code);
    }
    result
}

fn write_files_sync(edits: &[(PathBuf, String)]) -> (usize, Vec<String>) {
    let mut count = 0;
    let mut errors = Vec::new();
    for (path, content) in edits {
        if let Err(e) = std::fs::write(path, content) {
            errors.push(format!("{}: {}", path.display(), e));
        } else {
            count += 1;
        }
    }
    (count, errors)
}

fn detect_env_sync() -> (Option<GitInfo>, String) {
    let cwd = std::env::current_dir().ok();
    let cwd_name = cwd
        .as_ref()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let git_info = cwd.as_ref().and_then(|p| detect_git_info_sync(p));
    (git_info, cwd_name)
}

fn detect_git_info_sync(start: &Path) -> Option<GitInfo> {
    let mut current = Some(start);
    while let Some(dir) = current {
        let git_path = dir.join(".git");
        if git_path.is_dir() {
            return read_git_info_sync(&git_path);
        }
        if git_path.is_file() {
            if let Some(info) = read_worktree_git_info_sync(&git_path) {
                return Some(info);
            }
        }
        current = dir.parent();
    }
    None
}

fn read_git_info_sync(git_dir: &Path) -> Option<GitInfo> {
    let head_path = git_dir.join("HEAD");
    let branch = read_branch_sync(&head_path);
    let config_path = git_dir.join("config");
    let repo_name = read_origin_repo_name_sync(&config_path);
    Some(GitInfo {
        repo_name,
        branch,
        is_worktree: false,
        worktree_source: None,
    })
}

fn read_worktree_git_info_sync(git_file: &Path) -> Option<GitInfo> {
    let gitdir = std::fs::read_to_string(git_file).ok().and_then(|content| {
        content
            .trim()
            .strip_prefix("gitdir:")
            .map(|s| PathBuf::from(s.trim()))
    });
    let worktree_gitdir = gitdir?;
    let head_path = worktree_gitdir.join("HEAD");
    let branch = read_branch_sync(&head_path);
    let config_path = worktree_gitdir
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join("config"));
    let repo_name = config_path.and_then(|p| read_origin_repo_name_sync(&p));
    let worktree_source = worktree_gitdir
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .map(|p| p.to_string_lossy().to_string());
    Some(GitInfo {
        repo_name,
        branch,
        is_worktree: true,
        worktree_source,
    })
}

fn read_branch_sync(head_path: &Path) -> Option<String> {
    std::fs::read_to_string(head_path)
        .ok()
        .and_then(|content| {
            content
                .trim()
                .strip_prefix("ref: refs/heads/")
                .map(|b| b.to_owned())
        })
}

fn read_origin_repo_name_sync(config_path: &Path) -> Option<String> {
    std::fs::read_to_string(config_path)
        .ok()
        .and_then(|config| {
            config
                .lines()
                .skip_while(|line| !line.contains("[remote \"origin\"]"))
                .skip(1)
                .find(|line| line.trim().starts_with("url"))
                .and_then(|url_line| {
                    let url = url_line.split('=').nth(1)?;
                    let url = url.trim();
                    url.rsplit('/')
                        .next()
                        .map(|name| name.trim_end_matches(".git").to_owned())
                })
        })
}

use std::process::Stdio;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn execute_echo_command() {
        let output = run_bash_sync("echo hello");
        assert!(output.contains("hello"), "Should contain hello");
    }

    #[test]
    fn execute_pwd_command() {
        let output = run_bash_sync("pwd");
        assert!(!output.is_empty(), "pwd should return output");
    }

    #[test]
    fn command_not_found() {
        let output = run_bash_sync("nonexistent_command_xyz");
        assert!(
            output.contains("Error") || output.contains("not found"),
            "Should show error for invalid command"
        );
    }

    #[test]
    fn format_empty_output() {
        let result = format_command_output("", "", 0);
        assert_eq!(result, "(exit code: 0)");
    }

    #[test]
    fn format_stdout_only() {
        let result = format_command_output("hello\nworld", "", 0);
        assert_eq!(result, "hello\nworld");
    }

    #[test]
    fn format_stderr_included() {
        let result = format_command_output("", "error message", 1);
        assert!(result.contains("stderr: error message"));
    }

    #[test]
    fn format_combined_output() {
        let result = format_command_output("stdout\noutput", "stderr msg", 0);
        assert!(result.contains("stdout"));
        assert!(result.contains("stderr"));
    }

    #[tokio::test]
    async fn ractor_io_actor_spawns() {
        let bus = EventBus::<Event>::new(16);
        let result = RactorIoActor::spawn(bus).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn ractor_io_receives_messages() {
        let bus = EventBus::<Event>::new(16);
        let mut sub = bus.subscribe();
        let (handle, _cell) = RactorIoActor::spawn(bus).await.unwrap();

        handle.run_bash("echo test".to_string()).await;

        // Give actor time to process
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Verify we got a BashOutput event
        let mut found = false;
        while let Ok(evt) = sub.try_recv() {
            if matches!(evt, Event::BashOutput { .. }) {
                found = true;
                break;
            }
        }
        assert!(found, "Expected BashOutput event");
    }
}
