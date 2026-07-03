use runie_core::permissions::{
    is_read_only_tool, is_sensitive_path, PermissionAction, PermissionSet,
};

#[test]
fn policy_matches_core() {
    let rules = PermissionSet::default_rules();

    assert_read_only_tools_allowed(&rules);
    assert_mutating_tools_ask(&rules);
    assert_sensitive_paths_denied(&rules);
    assert_non_sensitive_paths_follow_rules(&rules);
}

fn assert_read_only_tools_allowed(rules: &PermissionSet) {
    for tool in ["read_file", "list_dir", "grep", "find", "fetch_docs"] {
        assert_eq!(
            rules.effective_action(tool, None, None),
            PermissionAction::Allow,
            "{} should be allowed by default",
            tool
        );
        assert!(
            is_read_only_tool(tool),
            "{} should be classified read-only",
            tool
        );
    }
}

fn assert_mutating_tools_ask(rules: &PermissionSet) {
    for tool in ["write_file", "edit_file", "bash"] {
        assert_eq!(
            rules.effective_action(tool, None, None),
            PermissionAction::Ask,
            "{} should ask by default",
            tool
        );
        assert!(
            !is_read_only_tool(tool),
            "{} should not be classified read-only",
            tool
        );
    }
}

fn assert_sensitive_paths_denied(rules: &PermissionSet) {
    assert!(is_sensitive_path("/home/user/.ssh/id_rsa"));
    assert_eq!(
        rules.effective_action("read_file", Some("/home/user/.ssh/id_rsa"), None),
        PermissionAction::Deny
    );
    assert_eq!(
        rules.effective_action("write_file", Some("/project/.env"), None),
        PermissionAction::Deny
    );
}

fn assert_non_sensitive_paths_follow_rules(rules: &PermissionSet) {
    assert!(!is_sensitive_path("/project/src/main.rs"));
    assert_eq!(
        rules.effective_action("read_file", Some("/project/src/main.rs"), None),
        PermissionAction::Allow
    );
}

// ============================================================================
// Layer 1: Approval timeout and cancellation tests
// ============================================================================

use crate::emit_approval_sink::EmitApprovalSink;
use runie_core::actors::permission::RactorPermissionActor;
use runie_core::bus::EventBus;
use runie_core::event::Event;
use runie_core::permissions::ApprovalSink;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

/// Layer 1: CancellationToken cancels pending approval and returns Deny.
#[tokio::test]
async fn approval_cancel_token_returns_deny() {
    let bus = EventBus::<Event>::new(16);
    let (perm_handle, _, _) = RactorPermissionActor::spawn_for_testing(bus.clone())
        .await
        .unwrap();

    // Use a short timeout (100ms) to keep the test fast.
    let sink = EmitApprovalSink::with_cancel(
        perm_handle,
        60, // timeout (not relevant when cancelled)
        CancellationToken::new(),
    );

    // Cancel the token before calling ask().
    sink.cancel_pending();

    let result = sink.ask("bash", &serde_json::json!({})).await;
    assert_eq!(
        result,
        PermissionAction::Deny,
        "Cancelled approval should return Deny"
    );
}

/// Layer 1: Timeout returns Deny even without explicit cancellation.
#[tokio::test]
async fn approval_timeout_returns_deny() {
    let bus = EventBus::<Event>::new(16);
    let (perm_handle, _, _) = RactorPermissionActor::spawn_for_testing(bus.clone())
        .await
        .unwrap();

    // Use a very short timeout (50ms) so the test completes quickly.
    let sink = EmitApprovalSink::with_cancel(perm_handle, 0, CancellationToken::new());

    let start = std::time::Instant::now();
    let result = sink.ask("bash", &serde_json::json!({})).await;
    let elapsed = start.elapsed();

    assert_eq!(
        result,
        PermissionAction::Deny,
        "Timed-out approval should return Deny"
    );
    // Should have waited approximately 0ms (timeout = 0).
    assert!(
        elapsed < Duration::from_millis(500),
        "Should not wait for real permission: elapsed={elapsed:?}"
    );
}

/// Layer 1: Cancelling the token mid-ask returns Deny quickly.
#[tokio::test]
async fn approval_cancelled_during_ask_returns_deny_quickly() {
    let bus = EventBus::<Event>::new(16);
    let (perm_handle, _, _) = RactorPermissionActor::spawn_for_testing(bus.clone())
        .await
        .unwrap();

    let cancel_token = CancellationToken::new();

    // Spawn ask in background.
    let handle = tokio::spawn({
        let perm = perm_handle.clone();
        let cancel = cancel_token.clone();
        async move {
            let sink = EmitApprovalSink::with_cancel(perm, 60, cancel);
            sink.ask("bash", &serde_json::json!({})).await
        }
    });

    // Give the ask() call time to start and register the request.
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Cancel while ask() is waiting.
    cancel_token.cancel();

    let result = handle.await.expect("task should complete");
    assert_eq!(
        result,
        PermissionAction::Deny,
        "Cancelled mid-ask should return Deny"
    );
}
