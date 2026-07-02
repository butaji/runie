//! Ractor-based `IoActor` implementation.
//!
//! Migrated from custom Actor trait to ractor for consistency with the rest
//! of the actor system.

use std::path::{Path, PathBuf};

use ractor::async_trait;
use ractor::{Actor, ActorProcessingErr, ActorRef};

use crate::actors::ractor_adapter::spawn_ractor;
use crate::bus::EventBus;
use crate::event::Event;
use crate::snapshot::GitInfo;
use crate::ChatMessage;

use super::messages::IoMsg;

/// Ractor-based IoActor handle.
#[derive(Clone, Debug)]
pub struct RactorIoHandle {
    inner: ActorRef<IoMsg>,
}

impl RactorIoHandle {
    /// Create a new handle wrapping an ActorRef.
    pub fn new(inner: ActorRef<IoMsg>) -> Self {
        Self { inner }
    }

    /// Send a message to the actor (fire-and-forget).
    pub async fn send(&self, msg: IoMsg) {
        let _ = self.inner.send_message(msg);
    }

    /// Request running a bash command.
    ///
    /// If `shell` is false (default), the command is parsed with `shell_words::split`
    /// and executed directly without a shell wrapper. This avoids shell indirection
    /// overhead and security risks for simple commands.
    ///
    /// If `shell` is true, the command is passed to `sh -c` to support shell
    /// metacharacters (pipes, redirects, command substitution, etc.).
    pub async fn run_bash(&self, command: String, shell: bool) {
        let _ = self.inner.send_message(IoMsg::RunBash { command, shell });
    }

    /// Request writing files.
    pub async fn write_files(&self, edits: Vec<(PathBuf, String)>) {
        let _ = self.inner.send_message(IoMsg::WriteFiles { edits });
    }

    /// Request environment detection.
    pub async fn detect_env(&self) {
        let _ = self.inner.send_message(IoMsg::DetectEnv);
    }

    /// Request sharing session to gist.
    pub async fn share_session(&self, messages: Vec<ChatMessage>, display_name: Option<String>) {
        let _ = self.inner.send_message(IoMsg::ShareSession {
            messages,
            display_name,
        });
    }

    /// Request opening external editor.
    pub async fn open_external_editor(&self, text: String) {
        let _ = self.inner.send_message(IoMsg::OpenExternalEditor { text });
    }

    /// Request clipboard write.
    pub async fn write_clipboard(&self, text: String) {
        let _ = self.inner.send_message(IoMsg::WriteClipboard { text });
    }

    /// Request clipboard read.
    pub async fn read_clipboard(&self) {
        let _ = self.inner.send_message(IoMsg::ReadClipboard);
    }

    /// Request process suspend.
    pub async fn suspend_process(&self) {
        let _ = self.inner.send_message(IoMsg::SuspendProcess);
    }

    /// Request loading skills from disk and emitting SkillsLoaded.
    pub async fn load_skills(&self) {
        let _ = self.inner.send_message(IoMsg::LoadSkills);
    }

    /// Request loading auth storage and emitting AuthLoaded.
    pub async fn load_auth(&self) {
        let _ = self.inner.send_message(IoMsg::LoadAuth);
    }

    /// Try to send a message (non-blocking).
    pub fn try_send(&self, msg: IoMsg) -> Result<(), ractor::MessagingErr<IoMsg>> {
        self.inner.send_message(msg)
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
            IoMsg::RunBash { command, shell } => self.run_bash(command, shell).await,
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
            IoMsg::LoadSkills => self.load_skills().await,
            IoMsg::LoadAuth => self.load_auth().await,
        }
        Ok(())
    }
}

impl RactorIoActor {
    async fn run_bash(&self, command: String, shell: bool) {
        let cmd = command.clone();
        let output = match tokio::task::spawn_blocking(move || {
            use runie_core::shell::run_bash_sync;
            let cwd = std::env::current_dir().unwrap_or_default();
            let env = std::collections::HashMap::new();
            run_bash_sync(&cmd, &cwd, &env, shell).output
        }).await {
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
            .unwrap_or_else(|e| {
                tracing::warn!("env detection failed: {}", e);
                (None, String::new())
            });
        self.emit(Event::EnvDetected { git_info, cwd_name });
    }

    async fn load_skills(&self) {
        let skills = tokio::task::spawn_blocking(crate::skills::load_all)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!("skills load failed: {}", e);
                Vec::new()
            });
        self.emit(Event::SkillsLoaded { skills });
    }

    async fn load_auth(&self) {
        let auth = tokio::task::spawn_blocking(crate::auth::AuthStorage::load)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!("auth load failed: {}", e);
                crate::auth::AuthStorage::default()
            });
        let providers: Vec<String> = auth.providers().map(String::from).collect();
        self.emit(Event::AuthLoaded { providers });
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

#[cfg(test)]
mod tests {
    use super::*;
    use runie_core::shell::{format_command_output, run_bash_sync};
    use std::collections::HashMap;
    use std::path::Path;

    #[test]
    fn execute_echo_command_shell() {
        let output = run_bash_sync("echo hello", Path::new("."), &HashMap::new(), true).output;
        assert!(output.contains("hello"), "Should contain hello");
    }

    #[test]
    fn execute_echo_command_direct() {
        let output = run_bash_sync("echo hello", Path::new("."), &HashMap::new(), false).output;
        assert!(output.contains("hello"), "Should contain hello");
    }

    #[test]
    fn execute_pwd_command() {
        let output = run_bash_sync("pwd", Path::new("."), &HashMap::new(), true).output;
        assert!(!output.is_empty(), "pwd should return output");
    }

    #[test]
    fn command_not_found() {
        let output = run_bash_sync("nonexistent_command_xyz", Path::new("."), &HashMap::new(), true).output;
        assert!(
            output.contains("Error") || output.contains("not found"),
            "Should show error for invalid command"
        );
    }

    #[test]
    fn quoted_args_direct_mode() {
        // In shell mode, quoting works as expected
        let output = run_bash_sync("echo 'hello world'", Path::new("."), &HashMap::new(), true).output;
        assert!(output.contains("hello world"), "Shell mode should preserve quotes");

        // In direct mode, single quotes are not special to shell_words
        let output = run_bash_sync("echo 'hello world'", Path::new("."), &HashMap::new(), false).output;
        // shell_words preserves the quoted string as a single argument
        // which is then passed to echo as a literal string
        assert!(!output.is_empty(), "Direct mode should work");
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

        handle.run_bash("echo test".to_string(), true).await;

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

    #[tokio::test]
    async fn ractor_io_load_skills_emits_skills_loaded() {
        let bus = EventBus::<Event>::new(16);
        let mut sub = bus.subscribe();
        let (handle, _cell, _) = RactorIoActor::spawn(bus).await.unwrap();

        handle.load_skills().await;

        // Wait for SkillsLoaded event
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
        let mut found = false;
        while !found && tokio::time::Instant::now() < deadline {
            match tokio::time::timeout(deadline - tokio::time::Instant::now(), sub.recv()).await {
                Ok(Ok(evt)) => {
                    if matches!(evt, Event::SkillsLoaded { .. }) {
                        found = true;
                    }
                }
                Ok(Err(_)) | Err(_) => break,
            }
        }
        assert!(found, "Expected SkillsLoaded event");
    }

    #[tokio::test]
    async fn ractor_io_load_auth_emits_auth_loaded() {
        let bus = EventBus::<Event>::new(16);
        let mut sub = bus.subscribe();
        let (handle, _cell, _) = RactorIoActor::spawn(bus).await.unwrap();

        handle.load_auth().await;

        // Wait for AuthLoaded event
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
        let mut found = false;
        while !found && tokio::time::Instant::now() < deadline {
            match tokio::time::timeout(deadline - tokio::time::Instant::now(), sub.recv()).await {
                Ok(Ok(evt)) => {
                    if matches!(evt, Event::AuthLoaded { .. }) {
                        found = true;
                    }
                }
                Ok(Err(_)) | Err(_) => break,
            }
        }
        assert!(found, "Expected AuthLoaded event");
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
        run_bash_sync(
            &format!("git init {} --quiet", tmp.display()),
            Path::new("."),
            &HashMap::new(),
            true,
        );
        run_bash_sync(
            &format!("git -C {} config user.email 'test@test.com'", tmp.display()),
            Path::new("."),
            &HashMap::new(),
            true,
        );
        run_bash_sync(
            &format!("git -C {} config user.name 'Test'", tmp.display()),
            Path::new("."),
            &HashMap::new(),
            true,
        );
        run_bash_sync(
            &format!(
                "touch {}/.gitkeep && git -C {} add .gitkeep && git -C {} commit -m 'init' --quiet",
                tmp.display(),
                tmp.display(),
                tmp.display()
            ),
            Path::new("."),
            &HashMap::new(),
            true,
        );

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
        run_bash_sync(
            &format!("git init {} --quiet", tmp.display()),
            Path::new("."),
            &HashMap::new(),
            true,
        );
        run_bash_sync(
            &format!("git -C {} config user.email 'test@test.com'", tmp.display()),
            Path::new("."),
            &HashMap::new(),
            true,
        );
        run_bash_sync(
            &format!("git -C {} config user.name 'Test'", tmp.display()),
            Path::new("."),
            &HashMap::new(),
            true,
        );
        run_bash_sync(
            &format!(
                "touch {}/.gitkeep && git -C {} add .gitkeep && git -C {} commit -m 'init' --quiet",
                tmp.display(),
                tmp.display(),
                tmp.display()
            ),
            Path::new("."),
            &HashMap::new(),
            true,
        );
        // Detach HEAD
        run_bash_sync(
            &format!("git -C {} checkout --detach HEAD --quiet", tmp.display()),
            Path::new("."),
            &HashMap::new(),
            true,
        );

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
