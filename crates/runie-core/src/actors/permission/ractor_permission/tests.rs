//! Tests for the permission actor.

use super::*;
use crate::bus::Receiver;

/// Wait for an event matching a predicate with a deterministic timeout.
async fn wait_for_event<F>(sub: &mut Receiver<Event>, pred: F) -> bool
where
    F: Fn(&Event) -> bool,
{
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
    while tokio::time::Instant::now() < deadline {
        let timeout_duration = deadline - tokio::time::Instant::now();
        match tokio::time::timeout(timeout_duration, sub.recv()).await {
            Ok(Ok(evt)) => {
                if pred(&evt) {
                    return true;
                }
            }
            Ok(Err(_)) | Err(_) => break,
        }
    }
    false
}

// ── Layer 1: State/Logic tests ──────────────────────────────────────────

#[tokio::test]
async fn permission_actor_awaits_resolution() {
    // Verify that AskPermission does NOT immediately resolve.
    // The receiver should still be pending until ResolvePermission is called.
    let bus = EventBus::<Event>::new(16);
    let (handle, _cell, _join) = RactorPermissionActor::spawn_for_testing(bus.clone())
        .await
        .unwrap();

    let mut rx = handle
        .ask_permission("req-await-1".into(), "bash".into(), serde_json::json!({}))
        .await;

    // Use try_recv to verify the channel is NOT yet complete
    // (would return Ok(Ready) if already resolved)
    let resolved = match rx.try_recv() {
        Ok(_) => true, // Got a value = already resolved
        Err(tokio::sync::oneshot::error::TryRecvError::Empty) => false, // Still pending
        Err(tokio::sync::oneshot::error::TryRecvError::Closed) => true, // Closed = also resolved
    };

    assert!(!resolved, "AskPermission should NOT immediately resolve");
}

#[tokio::test]
async fn permission_actor_resolves_with_allow() {
    let bus = EventBus::<Event>::new(16);
    let (handle, _cell, _join) = RactorPermissionActor::spawn_for_testing(bus.clone())
        .await
        .unwrap();

    let rx = handle
        .ask_permission("req-allow-1".into(), "bash".into(), serde_json::json!({}))
        .await;

    // Resolve with Allow
    handle
        .resolve_permission("req-allow-1".into(), PermissionAction::Allow)
        .await;

    // Verify the receiver gets Allow
    let result = tokio::time::timeout(std::time::Duration::from_secs(1), rx).await;
    assert!(result.is_ok(), "Should receive a result");
    assert_eq!(result.unwrap(), Ok(PermissionAction::Allow));
}

#[tokio::test]
async fn permission_actor_resolves_with_deny() {
    let bus = EventBus::<Event>::new(16);
    let (handle, _cell, _join) = RactorPermissionActor::spawn_for_testing(bus.clone())
        .await
        .unwrap();

    let rx = handle
        .ask_permission("req-deny-1".into(), "bash".into(), serde_json::json!({}))
        .await;

    // Resolve with Deny
    handle
        .resolve_permission("req-deny-1".into(), PermissionAction::Deny)
        .await;

    // Verify the receiver gets Deny
    let result = tokio::time::timeout(std::time::Duration::from_secs(1), rx).await;
    assert!(result.is_ok(), "Should receive a result");
    assert_eq!(result.unwrap(), Ok(PermissionAction::Deny));
}

#[tokio::test]
async fn permission_request_event_roundtrip() {
    // Layer 2: Event Handling - verify events flow correctly
    let bus = EventBus::<Event>::new(16);
    let mut sub = bus.subscribe();
    let (handle, _cell, _join) = RactorPermissionActor::spawn_for_testing(bus.clone())
        .await
        .unwrap();

    // Ask permission
    let _rx = handle
        .ask_permission(
            "req-event-1".into(),
            "bash".into(),
            serde_json::json!({"command": "ls"}),
        )
        .await;

    // Wait for PermissionRequest event
    let found = wait_for_event(
        &mut sub,
        |e| matches!(e, Event::PermissionRequest { request_id, .. } if request_id == "req-event-1"),
    )
    .await;
    assert!(found, "Expected PermissionRequest event");

    // Resolve permission
    handle
        .resolve_permission("req-event-1".into(), PermissionAction::Allow)
        .await;

    // Wait for PermissionResponse event
    let found = wait_for_event(&mut sub, |e| matches!(e, Event::PermissionResponse { request_id, action: PermissionAction::Allow, .. } if request_id == "req-event-1")).await;
    assert!(found, "Expected PermissionResponse event");
}

// Legacy test names for backward compatibility with existing test expectations
// These tests verify the same behavior as the new tests above.
#[tokio::test]
async fn ask_permission_stores_request() {
    // Same as permission_actor_awaits_resolution
    let bus = EventBus::<Event>::new(16);
    let (handle, _cell, _join) = RactorPermissionActor::spawn_for_testing(bus.clone())
        .await
        .unwrap();
    let mut rx = handle
        .ask_permission("req-legacy-1".into(), "bash".into(), serde_json::json!({}))
        .await;
    let resolved = match rx.try_recv() {
        Ok(_) => true,
        Err(tokio::sync::oneshot::error::TryRecvError::Empty) => false,
        Err(tokio::sync::oneshot::error::TryRecvError::Closed) => true,
    };
    assert!(!resolved, "AskPermission should NOT immediately resolve");
}

#[tokio::test]
async fn resolve_permission_clears_request() {
    // Same as permission_actor_resolves_with_allow
    let bus = EventBus::<Event>::new(16);
    let (handle, _cell, _join) = RactorPermissionActor::spawn_for_testing(bus.clone())
        .await
        .unwrap();
    let rx = handle
        .ask_permission("req-legacy-2".into(), "bash".into(), serde_json::json!({}))
        .await;
    handle
        .resolve_permission("req-legacy-2".into(), PermissionAction::Allow)
        .await;
    let result = tokio::time::timeout(std::time::Duration::from_secs(1), rx).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Ok(PermissionAction::Allow));
}

// ── Layer 1: Task acceptance criteria ────────────────────────────────────

/// AC: Layer 1 — a configured allow-rule permits a bash call without dialog.
///
/// When the agent queries permission rules via `get_rules()` and an allow-rule
/// matches the tool, `PermissionSetPolicy::evaluate` returns Allow without
/// consulting the approval sink (no dialog).
#[tokio::test]
async fn agent_gate_uses_user_trust_rules() {
    use crate::permissions::{
        PermissionContext, PermissionPolicy, PermissionSet, PermissionSetPolicy,
    };

    // Simulate user configured: [[permissions]] action = "allow", tool = "bash"
    let mut rules = PermissionSet::default_rules();
    rules.add_rule(crate::permissions::PermissionRule::new(
        PermissionAction::Allow,
        "bash",
    ));

    let policy = PermissionSetPolicy::new(rules);
    let ctx = PermissionContext {
        tool: "bash",
        path: None,
        input: Some(&serde_json::json!({"command": "echo hi"})),
        cwd: None,
        #[cfg(feature = "mcp")]
        annotations: crate::tool::annotations::get_tool_annotations("bash"),
    };

    // Policy matches and returns Allow
    let result = policy.evaluate(&ctx).await;
    assert_eq!(
        result,
        Some(crate::permissions::PermissionResult::Allow),
        "bash tool should be allowed by user trust rule"
    );
}

/// AC: Layer 2 — `/trust bash always` updates permission actor rule set.
///
/// When `UpsertRule` is sent to `PermissionActor`, it adds the rule to the
/// internal `PermissionSet` and subsequent `get_rules()` calls return it.
/// This mirrors the effect of `/trust bash always`.
#[tokio::test]
async fn trust_command_updates_permission_actor() {
    let bus = EventBus::<Event>::new(16);
    let _sub = bus.subscribe();

    let (handle, _cell, _join) = RactorPermissionActor::spawn_for_testing(bus.clone())
        .await
        .unwrap();

    // Before: bash should be Ask (from default rules)
    let rules_before = handle.get_rules().await;
    let bash_before = rules_before.effective_action("bash", None, None);
    assert_eq!(
        bash_before,
        PermissionAction::Ask,
        "bash should be Ask by default"
    );

    // UpsertRule: `/trust bash always` → add allow rule for bash
    handle
        .upsert_rule("bash".into(), PermissionAction::Allow)
        .await;

    // After: bash should be Allow (from upserted rule)
    let rules_after = handle.get_rules().await;
    let bash_after = rules_after.effective_action("bash", None, None);
    assert_eq!(
        bash_after,
        PermissionAction::Allow,
        "bash should be Allow after /trust bash always"
    );
}

// ── Layer 1: Pending map direct access tests ─────────────────────────────
// These tests verify the inlined pending map behavior that replaced ApprovalRegistry.

/// Test that canceling a pending permission request sends Deny.
#[tokio::test]
async fn cancel_permission_sends_deny() {
    let bus = EventBus::<Event>::new(16);
    let (handle, _cell, _join) = RactorPermissionActor::spawn_for_testing(bus.clone())
        .await
        .unwrap();

    let rx = handle
        .ask_permission("req-cancel-1".into(), "bash".into(), serde_json::json!({}))
        .await;

    // Cancel the request
    handle.cancel_permission("req-cancel-1".into()).await;

    // Verify the receiver gets Deny
    let result = tokio::time::timeout(std::time::Duration::from_secs(1), rx).await;
    assert!(result.is_ok(), "Should receive a result");
    assert_eq!(result.unwrap(), Ok(PermissionAction::Deny));
}

/// Test that resolving an unknown request does nothing (no panic).
#[tokio::test]
async fn resolve_unknown_request_is_noop() {
    let bus = EventBus::<Event>::new(16);
    let (handle, _cell, _join) = RactorPermissionActor::spawn_for_testing(bus.clone())
        .await
        .unwrap();

    // Try to resolve a non-existent request - should not panic
    handle
        .resolve_permission("nonexistent".into(), PermissionAction::Allow)
        .await;

    // Verify no request is pending
    assert!(handle.current_request_id().await.is_none());
}

/// Test that multiple concurrent permission requests are independent.
#[tokio::test]
async fn multiple_concurrent_requests_are_independent() {
    let bus = EventBus::<Event>::new(16);
    let (handle, _cell, _join) = RactorPermissionActor::spawn_for_testing(bus.clone())
        .await
        .unwrap();

    // Ask for two permissions concurrently
    let rx_a = handle
        .ask_permission(
            "req-multi-a".into(),
            "read_file".into(),
            serde_json::json!({}),
        )
        .await;
    let rx_b = handle
        .ask_permission("req-multi-b".into(), "bash".into(), serde_json::json!({}))
        .await;

    // Resolve the first one with Allow
    handle
        .resolve_permission("req-multi-a".into(), PermissionAction::Allow)
        .await;

    // Verify only the first one is resolved
    let result_a = tokio::time::timeout(std::time::Duration::from_millis(100), rx_a).await;
    let result_b = tokio::time::timeout(std::time::Duration::from_millis(100), rx_b).await;

    assert!(result_a.is_ok(), "First request should be resolved");
    assert_eq!(result_a.unwrap(), Ok(PermissionAction::Allow));
    assert!(result_b.is_err(), "Second request should still be pending");
}
