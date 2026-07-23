//! `IoActor` — owns user-initiated blocking IO.
//!
//! No ractor dependency. Actor is a tokio task with mpsc channel.

use std::path::PathBuf;
#[cfg(feature = "git")]
use std::path::Path;

use tokio::sync::mpsc;
use tracing::instrument;

use crate::bus::EventBus;
use crate::event::Event;
use crate::snapshot::GitInfo;
use crate::ChatMessage;

use super::messages::IoMsg;

/// IoActor handle — cloneable, fire-and-forget sender.
#[derive(Clone, Debug)]
pub struct IoActorHandle {
    tx: mpsc::UnboundedSender<IoMsg>,
}

impl IoActorHandle {
    /// Create a new handle wrapping a sender.
    pub fn new(tx: mpsc::UnboundedSender<IoMsg>) -> Self {
        Self { tx }
    }

    /// Send a message to the actor (fire-and-forget).
    pub async fn send(&self, msg: IoMsg) {
        let _ = self.tx.send(msg);
    }

    /// Request running a bash command.
    pub async fn run_bash(&self, command: String, shell: bool) {
        let _ = self.tx.send(IoMsg::RunBash { command, shell });
    }

    /// Request writing files.
    pub async fn write_files(&self, edits: Vec<(PathBuf, String)>) {
        let _ = self.tx.send(IoMsg::WriteFiles { edits });
    }

    /// Request environment detection.
    pub async fn detect_env(&self) {
        let _ = self.tx.send(IoMsg::DetectEnv);
    }

    /// Request sharing session to gist.
    pub async fn share_session(&self, messages: Vec<ChatMessage>, display_name: Option<String>) {
        let _ = self.tx.send(IoMsg::ShareSession { messages, display_name });
    }

    /// Request opening external editor.
    pub async fn open_external_editor(&self, text: String) {
        let _ = self.tx.send(IoMsg::OpenExternalEditor { text });
    }

    /// Request clipboard write.
    #[cfg(feature = "clipboard")]
    pub async fn write_clipboard(&self, text: String) {
        let _ = self.tx.send(IoMsg::WriteClipboard { text });
    }

    /// Request clipboard read.
    #[cfg(feature = "clipboard")]
    pub async fn read_clipboard(&self) {
        let _ = self.tx.send(IoMsg::ReadClipboard);
    }

    /// Request process suspend.
    pub async fn suspend_process(&self) {
        let _ = self.tx.send(IoMsg::SuspendProcess);
    }

    /// Request loading skills from disk and emitting SkillsLoaded.
    pub async fn load_skills(&self) {
        let _ = self.tx.send(IoMsg::LoadSkills);
    }

    /// Request loading auth storage and emitting AuthLoaded.
    pub async fn load_auth(&self) {
        let _ = self.tx.send(IoMsg::LoadAuth);
    }

    /// Try to send a message (non-blocking).
    pub fn try_send(&self, msg: IoMsg) -> Result<(), mpsc::error::SendError<IoMsg>> {
        self.tx.send(msg)
    }
}

// Backward-compat aliases
#[allow(unused_imports)]
pub use IoActorHandle as RactorIoHandle;

// ── Actor state ──────────────────────────────────────────────────────────────

/// Mutable state owned by the IoActor.
struct IoActorState {
    bus: EventBus<Event>,
}

impl IoActorState {
    fn emit(&self, event: Event) {
        self.bus.publish(event);
    }
}

// ── Actor struct ────────────────────────────────────────────────────────────

/// The IoActor — processes IO messages and emits events.
struct IoActor {
    rx: mpsc::UnboundedReceiver<IoMsg>,
    state: IoActorState,
}

impl IoActor {
    /// Main loop.
    async fn run(&mut self) {
        while let Some(msg) = self.rx.recv().await {
            self.handle(msg).await;
        }
    }

    /// Handle one message.
    #[instrument(name = "io_actor", skip_all, fields(msg = ?msg))]
    async fn handle(&mut self, msg: IoMsg) {
        match msg {
            IoMsg::RunBash { command, shell } => self.run_bash(command, shell).await,
            IoMsg::WriteFiles { edits } => self.write_files(edits).await,
            IoMsg::DetectEnv => self.detect_env().await,
            IoMsg::ShareSession { messages, display_name } => {
                self.share_session(messages, display_name).await;
            }
            IoMsg::OpenExternalEditor { text } => {
                self.open_external_editor(text).await;
            }
            #[cfg(feature = "clipboard")]
            IoMsg::WriteClipboard { text } => self.write_clipboard(text).await,
            #[cfg(feature = "clipboard")]
            IoMsg::ReadClipboard => self.read_clipboard().await,
            IoMsg::SuspendProcess => self.suspend_process().await,
            IoMsg::LoadSkills => self.load_skills().await,
            IoMsg::LoadAuth => self.load_auth().await,
        }
    }

    // ── Per-message handlers ──────────────────────────────────────────────

    async fn run_bash(&self, command: String, shell: bool) {
        let cmd = command.clone();
        let output = match tokio::task::spawn_blocking(move || {
            use crate::shell::run_bash_sync;
            let cwd = std::env::current_dir().unwrap_or_default();
            let env = std::collections::HashMap::new();
            run_bash_sync(&cmd, &cwd, &env, shell).output
        })
        .await
        {
            Ok(out) => out,
            Err(e) => format!("Error running command: {}", e),
        };
        self.state.emit(Event::BashOutput { command, output });
    }

    async fn write_files(&self, edits: Vec<(PathBuf, String)>) {
        let (count, errors) = match tokio::task::spawn_blocking(move || write_files_sync(&edits)).await {
            Ok(res) => res,
            Err(e) => (0, vec![format!("write task failed: {e}")]),
        };
        self.state.emit(Event::FilesWritten { count, errors });
    }

    async fn detect_env(&self) {
        let (git_info, cwd_name) = tokio::task::spawn_blocking(detect_env_sync)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!("env detection failed: {}", e);
                (None, String::new())
            });
        self.state.emit(Event::EnvDetected { git_info, cwd_name });
    }

    async fn load_skills(&self) {
        let skills = tokio::task::spawn_blocking(crate::skills::load_all)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!("skills load failed: {}", e);
                Vec::new()
            });
        self.state.emit(Event::SkillsLoaded { skills });
    }

    async fn load_auth(&self) {
        let auth = tokio::task::spawn_blocking(crate::auth::AuthStorage::load)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!("auth load failed: {}", e);
                crate::auth::AuthStorage::default()
            });
        let providers: Vec<String> = auth.providers().map(String::from).collect();
        self.state.emit(Event::AuthLoaded { providers });
    }

    async fn share_session(&self, messages: Vec<ChatMessage>, display_name: Option<String>) {
        let messages_clone = messages.clone();
        let name_clone = display_name.clone();
        let result = tokio::task::spawn_blocking(move || {
            super::effects::share_session_sync(&messages_clone, name_clone.as_deref())
        })
        .await
        .unwrap_or_else(|e| Err(format!("join error: {}", e)));
        self.state.emit(Event::GistShared { result });
    }

    async fn open_external_editor(&self, text: String) {
        let result = tokio::task::spawn_blocking(move || super::effects::open_editor_sync(text))
            .await
            .unwrap_or_else(|e| Err(e.to_string()));
        self.state.emit(Event::ExternalEditorClosed { result });
    }

    #[cfg(feature = "clipboard")]
    async fn write_clipboard(&self, text: String) {
        let success = tokio::task::spawn_blocking(move || super::effects::write_clipboard_sync(&text))
            .await
            .unwrap_or(false);
        self.state.emit(Event::ClipboardWritten { success });
    }

    #[cfg(feature = "clipboard")]
    async fn read_clipboard(&self) {
        let result = tokio::task::spawn_blocking(super::effects::read_clipboard_sync)
            .await
            .unwrap_or_else(|e| Err(e.to_string()));
        self.state.emit(Event::ClipboardRead { result });
    }

    #[cfg(unix)]
    async fn suspend_process(&self) {
        let bus = self.state.bus.clone();
        tokio::task::spawn_blocking(move || {
            let _ = crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen,);
            let _ = crossterm::terminal::disable_raw_mode();
            let _ = nix::sys::signal::kill(nix::unistd::Pid::this(), nix::sys::signal::Signal::SIGTSTP);
            let _ = crossterm::terminal::enable_raw_mode();
            let _ = crossterm::execute!(std::io::stdout(), crossterm::terminal::EnterAlternateScreen,);
            bus.publish(Event::ProcessResumed);
        });
    }

    #[cfg(not(unix))]
    async fn suspend_process(&self) {}
}

// ── Spawn ───────────────────────────────────────────────────────────────────

/// Spawn an IoActor and return (handle, stop_cell, join_handle).
pub fn spawn_io_actor(
    bus: EventBus<Event>,
) -> (IoActorHandle, crate::actors::StopCell, tokio::task::JoinHandle<()>) {
    let (tx, rx) = mpsc::unbounded_channel();
    let state = IoActorState { bus };
    let mut actor = IoActor { rx, state };

    let join = tokio::spawn(async move {
        actor.run().await;
    });

    (IoActorHandle::new(tx), crate::actors::StopCell, join)
}



// ── Sync helpers ────────────────────────────────────────────────────────────

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
    #[cfg(feature = "git")]
    let git_info = cwd.as_ref().and_then(|p| detect_git_info_sync(p));
    #[cfg(not(feature = "git"))]
    let git_info = None;
    (git_info, cwd_name)
}

#[cfg(feature = "git")]
fn detect_git_info_sync(start: &Path) -> Option<GitInfo> {
    let repo = git2::Repository::discover(start).ok()?;

    // Current branch name
    let branch = repo.head().ok().and_then(|h| h.shorthand().map(String::from));

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

    // Worktree detection
    let is_worktree = repo.is_worktree();

    let worktree_source = if is_worktree {
        repo.path().parent().and_then(|p| p.parent()).map(|p| p.to_string_lossy().to_string())
    } else {
        None
    };

    Some(GitInfo { repo_name, branch, is_worktree, worktree_source })
}
