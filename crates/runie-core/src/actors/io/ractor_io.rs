//! Ractor-based `IoActor` implementation.
//!
//! Migrated from custom Actor trait to ractor for consistency with the rest
//! of the actor system.

use std::path::{Path, PathBuf};
use std::process::Command;

use ractor::async_trait;
use ractor::{Actor, ActorProcessingErr, ActorRef};

use crate::actors::ractor_adapter::{spawn_ractor, RactorHandle};
use crate::bus::EventBus;
use crate::event::Event;
use crate::snapshot::GitInfo;
use crate::ChatMessage;

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

    /// Send a message to the actor (fire-and-forget).
    pub async fn send(&self, msg: IoMsg) {
        let _ = self.inner.send(msg).await;
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
        self.inner
            .send(IoMsg::ShareSession {
                messages,
                display_name,
            })
            .await;
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

    /// Try to send a message (non-blocking).
    pub fn try_send(&self, msg: IoMsg) -> Result<(), ractor::MessagingErr<IoMsg>> {
        self.inner.try_send(msg)
    }
}

/// Ractor-based IoActor.
pub struct RactorIoActor {
    bus: EventBus<Event>,
}

impl RactorIoActor {
    fn new(bus: EventBus<Event>) -> Self {
        Self { bus }
    }

    /// Spawn a `RactorIoActor` on the given event bus.
    pub async fn spawn(
        bus: EventBus<Event>,
    ) -> Result<(RactorIoHandle, ractor::ActorCell, tokio::task::JoinHandle<()>), ractor::SpawnErr> {
        let actor = Self::new(bus.clone());
        let (handle, join, cell) = spawn_ractor(None, actor, bus).await?;
        Ok((RactorIoHandle::new(handle), cell, join))
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
            IoMsg::ShareSession {
                messages,
                display_name,
            } => {
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
        let success =
            tokio::task::spawn_blocking(move || super::effects::write_clipboard_sync(&text))
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
        let bus = self.bus.clone();
        tokio::task::spawn_blocking(move || {
            let _ =
                crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen,);
            let _ = crossterm::terminal::disable_raw_mode();
            let _ =
                nix::sys::signal::kill(nix::unistd::Pid::this(), nix::sys::signal::Signal::SIGTSTP);
            let _ = crossterm::terminal::enable_raw_mode();
            let _ =
                crossterm::execute!(std::io::stdout(), crossterm::terminal::EnterAlternateScreen,);
            bus.publish(Event::ProcessResumed);
        });
    }

    #[cfg(not(unix))]
    async fn suspend_process(&self) {}

    fn emit(&self, event: Event) {
        self.bus.publish(event);
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
    let repo = git2::Repository::discover(start).ok()?;

    // Current branch name
    let branch = repo
        .head()
        .ok()
        .and_then(|h| h.shorthand().map(String::from));

    // Origin remote URL → repo name
    let repo_name = repo
        .find_remote("origin")
        .ok()
        .and_then(|r| r.url().map(String::from))
        .and_then(|url| {
            url.rsplit('/')
                .next()
                .map(|n| n.trim_end_matches(".git").to_owned())
        });

    // Worktree detection: is_worktree() is true when inside a worktree
    let is_worktree = repo.is_worktree();

    // For worktrees, the main repo is the parent of .git (where the worktree was created)
    let worktree_source = if is_worktree {
        repo.path()
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.to_string_lossy().to_string())
    } else {
        None
    };

    Some(GitInfo {
        repo_name,
        branch,
        is_worktree,
        worktree_source,
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
        let (handle, _cell, _) = RactorIoActor::spawn(bus).await.unwrap();

        handle.run_bash("echo test".to_string()).await;

        // Wait for BashOutput event
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
        let mut found = false;
        while !found && tokio::time::Instant::now() < deadline {
            match tokio::time::timeout(deadline - tokio::time::Instant::now(), sub.recv()).await {
                Ok(Ok(evt)) => {
                    if matches!(evt, Event::BashOutput { .. }) {
                        found = true;
                    }
                }
                Ok(Err(_)) | Err(_) => break,
            }
        }
        assert!(found, "Expected BashOutput event");
    }

    // ── Git detection tests ──────────────────────────────────────────────────────

    #[test]
    fn detect_git_in_real_repo() {
        // Test against the actual runie-dev repo
        let start = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
        let info = detect_git_info_sync(&start);
        assert!(info.is_some(), "Should detect git in runie-dev repo: {:?}", info);
        let info = info.unwrap();
        assert!(
            info.branch.is_some(),
            "Should detect branch: {:?}",
            info.branch
        );
        assert!(
            info.repo_name.is_some(),
            "Should detect repo name: {:?}",
            info.repo_name
        );
        // is_worktree depends on where the test is run; just verify info is returned
    }

    #[test]
    fn detect_git_non_git_dir_returns_none() {
        // /tmp should not be a git repo (usually)
        let info = detect_git_info_sync(Path::new("/tmp"));
        assert!(
            info.is_none(),
            "Non-git directory should return None: {:?}",
            info
        );
    }

    #[test]
    fn detect_git_in_tmp_git_repo() {
        // Create a temp git repo
        let tmp = std::env::temp_dir().join("runie_git_test_").join(
            uuid::Uuid::new_v4().to_string(),
        );
        std::fs::create_dir_all(&tmp).unwrap();
        run_bash_sync(&format!("git init {} --quiet", tmp.display()));
        run_bash_sync(&format!(
            "git -C {} config user.email 'test@test.com'",
            tmp.display()
        ));
        run_bash_sync(&format!(
            "git -C {} config user.name 'Test'",
            tmp.display()
        ));
        run_bash_sync(&format!(
            "touch {}/.gitkeep && git -C {} add .gitkeep && git -C {} commit -m 'init' --quiet",
            tmp.display(),
            tmp.display(),
            tmp.display()
        ));

        let info = detect_git_info_sync(&tmp);
        assert!(info.is_some(), "Should detect git in temp repo: {:?}", info);
        let info = info.unwrap();
        assert_eq!(info.branch, Some("main".to_string()), "Should detect 'main' branch");
        assert!(info.repo_name.is_none(), "No origin → no repo name");
        assert!(!info.is_worktree, "Not a worktree");

        // Cleanup
        std::fs::remove_dir_all(tmp.parent().unwrap()).ok();
    }

    #[test]
    fn detect_git_detached_head() {
        let tmp = std::env::temp_dir().join("runie_git_detached_").join(
            uuid::Uuid::new_v4().to_string(),
        );
        std::fs::create_dir_all(&tmp).unwrap();
        run_bash_sync(&format!("git init {} --quiet", tmp.display()));
        run_bash_sync(&format!(
            "git -C {} config user.email 'test@test.com'",
            tmp.display()
        ));
        run_bash_sync(&format!(
            "git -C {} config user.name 'Test'",
            tmp.display()
        ));
        run_bash_sync(&format!(
            "touch {}/.gitkeep && git -C {} add .gitkeep && git -C {} commit -m 'init' --quiet",
            tmp.display(),
            tmp.display(),
            tmp.display()
        ));
        // Detach HEAD
        run_bash_sync(&format!("git -C {} checkout --detach HEAD --quiet", tmp.display()));

        let info = detect_git_info_sync(&tmp);
        assert!(info.is_some(), "Should detect detached HEAD repo");
        let info = info.unwrap();
        // git2 returns Some("HEAD") for detached HEAD shorthand
        assert_eq!(
            info.branch.as_deref(),
            Some("HEAD"),
            "Detached HEAD shorthand: {:?}",
            info.branch
        );

        // Cleanup
        std::fs::remove_dir_all(tmp.parent().unwrap()).ok();
    }
}
