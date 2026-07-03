//! Tests for the IO actor.

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
    let output = run_bash_sync(
        "nonexistent_command_xyz",
        Path::new("."),
        &HashMap::new(),
        true,
    )
    .output;
    assert!(
        output.contains("Error") || output.contains("not found"),
        "Should show error for invalid command"
    );
}

#[test]
fn quoted_args_direct_mode() {
    // In shell mode, quoting works as expected
    let output = run_bash_sync("echo 'hello world'", Path::new("."), &HashMap::new(), true).output;
    assert!(
        output.contains("hello world"),
        "Shell mode should preserve quotes"
    );

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

#[cfg(feature = "git")]
#[test]
fn detect_git_in_real_repo() {
    // Test against the actual runie-dev repo
    let start = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let info = detect_git_info_sync(&start);
    assert!(
        info.is_some(),
        "Should detect git in runie-dev repo: {:?}",
        info
    );
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

#[cfg(feature = "git")]
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

#[cfg(feature = "git")]
#[test]
fn detect_git_in_tmp_git_repo() {
    // Create a temp git repo
    let tmp = std::env::temp_dir()
        .join("runie_git_test_")
        .join(uuid::Uuid::new_v4().to_string());
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
    assert_eq!(
        info.branch,
        Some("main".to_string()),
        "Should detect 'main' branch"
    );
    assert!(info.repo_name.is_none(), "No origin → no repo name");
    assert!(!info.is_worktree, "Not a worktree");

    // Cleanup
    std::fs::remove_dir_all(tmp.parent().unwrap()).ok();
}

#[cfg(feature = "git")]
#[test]
fn detect_git_detached_head() {
    let tmp = std::env::temp_dir()
        .join("runie_git_detached_")
        .join(uuid::Uuid::new_v4().to_string());
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
