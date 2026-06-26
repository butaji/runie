//! `IoActor` — owns user-initiated blocking IO (bash, file writes, git detection).

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::actors::{spawn_actor, Actor, ActorHandle};
use crate::bus::EventBus;
use crate::event::Event;
use crate::snapshot::GitInfo;

use super::messages::{IoActorHandle, IoMsg};

/// Actor that owns blocking IO effects initiated by the user.
pub struct IoActor {
    bus: EventBus<Event>,
}

impl IoActor {
    /// Spawn an `IoActor` on the given event bus.
    pub fn spawn(bus: EventBus<Event>) -> (IoActorHandle, ActorHandle) {
        let actor = Self { bus: bus.clone() };
        let (tx, handle) = spawn_actor(actor, bus);
        (IoActorHandle::new(tx), handle)
    }
}

impl Actor for IoActor {
    type Msg = IoMsg;
    type Event = Event;

    async fn run_body(self, mut rx: tokio::sync::mpsc::Receiver<Self::Msg>, _bus: EventBus<Event>) {
        while let Some(msg) = rx.recv().await {
            self.handle(msg).await;
        }
    }
}

impl IoActor {
    async fn handle(&self, msg: IoMsg) {
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
    }

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

    /// Detect cwd name and git info asynchronously.
    async fn detect_env(&self) {
        let (git_info, cwd_name) = tokio::task::spawn_blocking(detect_env_sync)
            .await
            .unwrap_or_default();
        self.emit(Event::EnvDetected { git_info, cwd_name });
    }

    async fn share_session(&self, messages: Vec<crate::ChatMessage>, display_name: Option<String>) {
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
        let bus = self.bus.clone();
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
    async fn suspend_process(&self) {
        // No-op on non-Unix platforms
    }

    fn emit(&self, event: Event) {
        let _ = self.bus.publish(event);
    }
}

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

/// Synchronous git and cwd detection for use in spawn_blocking.
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

/// Detect git repo name and current branch from the given directory.
/// Walks up the tree looking for `.git` (dir or file with `gitdir:` pointer).
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

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

    // Git detection tests

    #[test]
    fn git_detect_finds_branch_and_repo() {
        let temp_dir = tempfile::tempdir().unwrap();
        let git_dir = temp_dir.path().join(".git");
        fs::create_dir_all(&git_dir).unwrap();

        // Create HEAD with branch ref
        fs::write(git_dir.join("HEAD"), "ref: refs/heads/main").unwrap();

        // Create config with origin
        fs::write(
            git_dir.join("config"),
            "[remote \"origin\"]\n        url = https://github.com/test/repo.git\n",
        )
        .unwrap();

        let result = detect_git_info_sync(temp_dir.path());
        assert!(result.is_some());
        let info = result.unwrap();
        assert_eq!(info.branch, Some("main".to_string()));
        assert_eq!(info.repo_name, Some("repo".to_string()));
        assert!(!info.is_worktree);
    }

    #[test]
    fn git_detect_returns_none_for_non_git_dir() {
        let temp_dir = tempfile::tempdir().unwrap();
        let result = detect_git_info_sync(temp_dir.path());
        assert!(result.is_none());
    }

    #[test]
    fn git_detect_walks_up_directory_tree() {
        let temp_dir = tempfile::tempdir().unwrap();
        let sub_dir = temp_dir.path().join("subdir/nested");
        fs::create_dir_all(&sub_dir).unwrap();

        // Create git in parent, not in subdir
        let git_dir = temp_dir.path().join(".git");
        fs::create_dir_all(&git_dir).unwrap();
        fs::write(git_dir.join("HEAD"), "ref: refs/heads/feature").unwrap();

        // Detect from subdirectory should find parent git
        let result = detect_git_info_sync(&sub_dir);
        assert!(result.is_some());
        let info = result.unwrap();
        assert_eq!(info.branch, Some("feature".to_string()));
    }

    #[test]
    fn read_branch_extracts_branch_name() {
        let temp_dir = tempfile::tempdir().unwrap();
        fs::write(temp_dir.path().join("HEAD"), "ref: refs/heads/develop").unwrap();

        let result = read_branch_sync(temp_dir.path().join("HEAD").as_path());
        assert_eq!(result, Some("develop".to_string()));
    }

    #[test]
    fn read_branch_handles_detached_head() {
        let temp_dir = tempfile::tempdir().unwrap();
        // Detached HEAD contains commit hash, not branch ref
        fs::write(temp_dir.path().join("HEAD"), "abc123def456").unwrap();

        let result = read_branch_sync(temp_dir.path().join("HEAD").as_path());
        assert_eq!(result, None);
    }

    #[test]
    fn read_origin_repo_name_extracts_from_config() {
        let temp_dir = tempfile::tempdir().unwrap();
        fs::write(
            temp_dir.path().join("config"),
            "[core]\n        repositoryformatversion = 0\n[remote \"origin\"]\n        url = https://github.com/myuser/myproject.git\n",
        )
        .unwrap();

        let result = read_origin_repo_name_sync(temp_dir.path().join("config").as_path());
        assert_eq!(result, Some("myproject".to_string()));
    }

    #[test]
    fn read_origin_repo_name_handles_missing_origin() {
        let temp_dir = tempfile::tempdir().unwrap();
        fs::write(
            temp_dir.path().join("config"),
            "[core]\n        repositoryformatversion = 0\n",
        )
        .unwrap();

        let result = read_origin_repo_name_sync(temp_dir.path().join("config").as_path());
        assert_eq!(result, None);
    }
}
