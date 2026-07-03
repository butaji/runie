//! Tests for the IO actor.
//!
//! Shell execution tests have been moved to the shell module where they belong.
//! The actor tests here use deterministic timing with TestTimeGuard.

use super::*;
use runie_core::shell::format_command_output;

// ── Format tests (pure) ───────────────────────────────────────────────────────

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

// ── Actor tests ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn ractor_io_actor_spawns() {
    let _guard = runie_testing::TestTimeGuard::new()
        .expect("should support time pausing");
    let bus = EventBus::<Event>::new(16);
    let result = RactorIoActor::spawn(bus).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn ractor_io_load_skills_emits_skills_loaded() {
    let _guard = runie_testing::TestTimeGuard::new()
        .expect("should support time pausing");

    let bus = EventBus::<Event>::new(16);
    let mut sub = bus.subscribe();
    let (handle, _cell, _) = RactorIoActor::spawn(bus).await.unwrap();

    handle.load_skills().await;

    // Advance virtual time and wait for SkillsLoaded event
    let found = runie_testing::wait_for_event(
        &mut sub,
        std::time::Duration::from_secs(5),
        |evt| matches!(evt, Event::SkillsLoaded { .. }),
    )
    .await;
    assert!(found.is_some(), "Expected SkillsLoaded event");
}

#[tokio::test]
async fn ractor_io_load_auth_emits_auth_loaded() {
    let _guard = runie_testing::TestTimeGuard::new()
        .expect("should support time pausing");

    let bus = EventBus::<Event>::new(16);
    let mut sub = bus.subscribe();
    let (handle, _cell, _) = RactorIoActor::spawn(bus).await.unwrap();

    handle.load_auth().await;

    // Advance virtual time and wait for AuthLoaded event
    let found = runie_testing::wait_for_event(
        &mut sub,
        std::time::Duration::from_secs(5),
        |evt| matches!(evt, Event::AuthLoaded { .. }),
    )
    .await;
    assert!(found.is_some(), "Expected AuthLoaded event");
}

// ── Git detection tests ──────────────────────────────────────────────────────
// These use git2 directly, not shell commands.

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
}

#[cfg(feature = "git")]
#[test]
fn detect_git_non_git_dir_returns_none() {
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
    use std::fs;
    use std::process::Command;

    // Create a temp git repo using git2 directly
    let tmp = std::env::temp_dir()
        .join("runie_git_test_")
        .join(uuid::Uuid::new_v4().to_string());
    fs::create_dir_all(&tmp).unwrap();

    // Initialize repo using Command to ensure it exists
    let output = Command::new("git")
        .args(["init", &tmp.to_string_lossy()])
        .output();
    // If git command fails, skip this test
    if output.map(|o| !o.status.success()).unwrap_or(true) {
        std::fs::remove_dir_all(&tmp).ok();
        return;
    }

    // Configure git
    let _ = Command::new("git")
        .args(["-C", &tmp.to_string_lossy(), "config", "user.email", "test@test.com"])
        .output();
    let _ = Command::new("git")
        .args(["-C", &tmp.to_string_lossy(), "config", "user.name", "Test"])
        .output();

    // Create commit
    fs::write(tmp.join(".gitkeep"), "").unwrap();
    let _ = Command::new("git")
        .args(["-C", &tmp.to_string_lossy(), "add", "."])
        .output();
    let _ = Command::new("git")
        .args([
            "-C",
            &tmp.to_string_lossy(),
            "commit",
            "-m",
            "init",
            "--quiet",
        ])
        .output();

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
    use std::fs;
    use std::process::Command;

    let tmp = std::env::temp_dir()
        .join("runie_git_detached_")
        .join(uuid::Uuid::new_v4().to_string());
    fs::create_dir_all(&tmp).unwrap();

    // Initialize repo
    let output = Command::new("git")
        .args(["init", &tmp.to_string_lossy()])
        .output();
    if output.map(|o| !o.status.success()).unwrap_or(true) {
        std::fs::remove_dir_all(&tmp).ok();
        return;
    }

    // Configure and commit
    let _ = Command::new("git")
        .args(["-C", &tmp.to_string_lossy(), "config", "user.email", "test@test.com"])
        .output();
    let _ = Command::new("git")
        .args(["-C", &tmp.to_string_lossy(), "config", "user.name", "Test"])
        .output();
    fs::write(tmp.join(".gitkeep"), "").unwrap();
    let _ = Command::new("git")
        .args(["-C", &tmp.to_string_lossy(), "add", "."])
        .output();
    let _ = Command::new("git")
        .args([
            "-C",
            &tmp.to_string_lossy(),
            "commit",
            "-m",
            "init",
            "--quiet",
        ])
        .output();

    // Detach HEAD
    let _ = Command::new("git")
        .args(["-C", &tmp.to_string_lossy(), "checkout", "--detach", "HEAD", "--quiet"])
        .output();

    let info = detect_git_info_sync(&tmp);
    assert!(info.is_some(), "Should detect detached HEAD repo");
    let info = info.unwrap();
    assert_eq!(
        info.branch.as_deref(),
        Some("HEAD"),
        "Detached HEAD shorthand: {:?}",
        info.branch
    );

    // Cleanup
    std::fs::remove_dir_all(tmp.parent().unwrap()).ok();
}
