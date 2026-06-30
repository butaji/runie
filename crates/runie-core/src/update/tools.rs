#![allow(clippy::items_after_test_module)]
//! Bash command execution for ! prefix (merged from bash.rs).

use crate::actors::IoMsg;
use std::process::{Command as SyncCommand, Stdio};

use shell_words;

/// Execute a bash command and return output string (sync fallback).
///
/// Used by the non-actor fallback path in test mode.
///
/// If `shell` is true, the command is passed to `sh -c` to support shell
/// metacharacters (pipes, redirects, command substitution, etc.).
///
/// If `shell` is false, the command is parsed with `shell_words::split`
/// and executed directly.
pub fn execute_bash_sync(command: &str, shell: bool) -> String {
    let output = if shell {
        // Shell mode: use sh -c to support metacharacters
        SyncCommand::new("sh")
            .arg("-c")
            .arg(command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
    } else {
        // Direct mode: parse with shell_words and execute directly
        match shell_words::split(command) {
            Ok(args) => {
                if args.is_empty() {
                    return String::new();
                }
                let (program, args) = (&args[0], &args[1..]);
                let mut cmd = SyncCommand::new(program);
                cmd.args(args);
                cmd.stdout(Stdio::piped());
                cmd.stderr(Stdio::piped());
                cmd.output()
            }
            Err(_) => {
                return format!("Error parsing command: {}", command);
            }
        }
    };

    let output = match output {
        Ok(out) => out,
        Err(e) => return format!("Error running command: {}", e),
    };

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap_or(-1);

    format_command_output(&stdout, &stderr, exit_code)
}

/// Format command output for display
pub fn format_command_output(stdout: &str, stderr: &str, exit_code: i32) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

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

// ── Form-submit and edit-event handling (merged from edit.rs) ─────────────────

use crate::model::AppState;
use crate::Event;

pub fn update(state: &mut AppState, event: Event) {
    match event {
        Event::PendingEdit {
            path,
            original,
            proposed,
        } => {
            state
                .session
                .pending_edits
                .push(crate::edit_preview::EditPreview::new(
                    std::path::PathBuf::from(path),
                    original,
                    proposed,
                ));
            state.view_mut().dirty = true;
        }
        Event::ApproveEdit => state.approve_edits(),
        Event::RejectEdit => state.reject_edits(),
        // intentionally ignored: other edit events fall through
        _ => {}
    }
}

// ── Edit approval/rejection (merged from edit_approval.rs) ───────────────────

impl AppState {
    /// Try to spawn IO write via actor_handles, else fallback to sync.
    fn try_spawn_io_write(&mut self) -> bool {
        // Extract handles before async work to avoid borrow conflicts.
        let handles = self.actor_handles().cloned();
        let can_spawn = handles.as_ref().is_some() && tokio::runtime::Handle::try_current().is_ok();

        if can_spawn {
            let edits: Vec<_> = self
                .session_mut()
                .pending_edits
                .drain(..)
                .map(|p| (p.path, p.proposed))
                .collect();
            let handles = handles.unwrap();
            let _ = handles.io.try_send(IoMsg::WriteFiles { edits });
            return true;
        }
        false
    }

    pub(crate) fn approve_edits(&mut self) {
        if self.session().pending_edits.is_empty() {
            self.add_system_msg("No pending edits to approve.".to_owned());
            return;
        }
        if self.try_spawn_io_write() {
            return;
        }
        let mut applied = 0;
        let mut errors = Vec::new();
        for preview in self.session_mut().pending_edits.drain(..) {
            let path = preview.path.clone();
            let content = preview.proposed.clone();
            match tokio::task::block_in_place(|| std::fs::write(&path, content)) {
                Ok(()) => applied += 1,
                Err(e) => errors.push(format!("{}: {}", preview.path.display(), e)),
            }
        }
        let mut msg = format!("Applied {} edit(s).", applied);
        if !errors.is_empty() {
            msg.push_str(" Errors: ");
            msg.push_str(&errors.join(", "));
        }
        self.add_system_msg(msg);
    }

    pub(crate) fn reject_edits(&mut self) {
        let count = self.session().pending_edits.len();
        if count == 0 {
            self.add_system_msg("No pending edits to reject.".to_owned());
            return;
        }
        self.session_mut().pending_edits.clear();
        self.add_system_msg(format!("Rejected {} edit(s).", count));
    }
}
