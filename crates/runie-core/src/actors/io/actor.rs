//! `IoActor` — owns user-initiated blocking IO (bash, file writes).

use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::actors::{spawn_actor, Actor, ActorHandle};
use crate::bus::EventBus;
use crate::event::Event;

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
        let (count, errors) = match tokio::task::spawn_blocking(move || write_files_sync(&edits)).await {
            Ok(res) => res,
            Err(e) => (0, vec![format!("write task failed: {e}")]),
        };
        self.emit(Event::FilesWritten { count, errors });
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
}
