use super::*;

const VALID_COMMIT: &str = "0123456789abcdef";
const INVALID_COMMIT: &str = "01234xz";
const DASH_COMMIT: &str = "-0123456";

#[tokio::test]
async fn status_is_read_only_and_returns_structured_metadata() {
    let result = GitStatusTool
        .execute("1", serde_json::json!({}), None, None)
        .await
        .unwrap();
    assert_eq!(result.details["command"][0], "status");
    assert!(!result.details["truncated"].as_bool().unwrap());
}

#[tokio::test]
async fn diff_stat_is_a_valid_projection() {
    let result = GitDiffTool
        .execute("1", serde_json::json!({"stat":true}), None, None)
        .await
        .unwrap();
    assert_eq!(result.details["command"][0], "diff");
}

#[tokio::test]
async fn review_reports_a_machine_readable_clean_state() {
    let result = GitReviewTool
        .execute("1", serde_json::json!({}), None, None)
        .await
        .unwrap();
    assert!(result.details["clean"].is_boolean());
    assert!(result.details["stat"].is_string());
}

#[tokio::test]
async fn worktree_listing_is_read_only() {
    let result = GitWorktreeTool
        .execute("1", serde_json::json!({}), None, None)
        .await
        .unwrap();
    assert_eq!(result.details["command"][0], "worktree");
}

#[test]
fn commit_preparation_validates_subjects_without_mutation() {
    let tool = GitCommitPrepareTool;
    assert!(tool
        .validate_arguments(&serde_json::json!({"message":"Add tool"}))
        .is_ok());
    assert!(tool
        .validate_arguments(&serde_json::json!({"message":" "}))
        .is_err());
    assert!(tool
        .validate_arguments(&serde_json::json!({"message":"x".repeat(73)}))
        .is_err());
}

#[test]
fn commit_tool_shares_strict_message_validation() {
    let tool = GitCommitTool;
    assert!(tool
        .validate_arguments(&serde_json::json!({"message":"Ship change"}))
        .is_ok());
    assert!(tool
        .validate_arguments(&serde_json::json!({"message":" "}))
        .is_err());
}

#[test]
fn push_requires_explicit_safe_remote_and_reference() {
    let tool = GitPushTool;
    assert!(tool
        .validate_arguments(&serde_json::json!({"remote":"origin","reference":"main"}))
        .is_ok());
    for value in ["", "origin main", "-origin"] {
        assert!(tool
            .validate_arguments(&serde_json::json!({"remote":value,"reference":"main"}))
            .is_err());
    }
    assert!(tool
        .validate_arguments(&serde_json::json!({"remote":"origin","reference":"main branch"}))
        .is_err());
}

#[test]
fn revert_requires_a_hex_commit_reference() {
    let tool = GitRevertTool;
    assert!(tool
        .validate_arguments(&serde_json::json!({"commit":VALID_COMMIT}))
        .is_ok());
    for value in ["short", INVALID_COMMIT, DASH_COMMIT, "0123 456"] {
        assert!(tool
            .validate_arguments(&serde_json::json!({"commit":value}))
            .is_err());
    }
}

#[test]
fn conflict_projection_is_lossless_and_ignores_clean_changes() {
    let summary = classify_conflicts(" M clean.rs\nUU src/main.rs\nAA src/new.rs\n");
    assert_eq!(summary.conflicted_paths, ["src/main.rs", "src/new.rs"]);
    assert_eq!(summary.entries[0].code, "UU");
    assert_eq!(summary.entries[1].code, "AA");
    assert!(summary.recoverable);
    assert!(!classify_conflicts(" M clean.rs\n").recoverable);
}

#[test]
fn conflict_summary_owns_renderer_neutral_terminal_rows() {
    let summary = classify_conflicts("UU src/main.rs\nAA src/new.rs\n");
    assert_eq!(
        summary.terminal_lines(),
        [
            "Git conflicts: 2 recoverable=true",
            "Conflict: UU src/main.rs",
            "Conflict: AA src/new.rs"
        ]
    );
}

#[test]
fn conflict_recovery_plan_is_data_only_and_deterministic() {
    let plan = plan_conflict_recovery(&classify_conflicts("UU src/main.rs\n"));
    assert_eq!(plan.paths, ["src/main.rs"]);
    assert_eq!(
        plan.actions,
        [
            GitConflictAction::Inspect,
            GitConflictAction::Resolve,
            GitConflictAction::Abort
        ]
    );
    assert!(plan_conflict_recovery(&classify_conflicts(" M clean.rs\n"))
        .actions
        .is_empty());
}

#[test]
fn conflict_recovery_plan_admits_only_known_actions_and_paths() {
    let plan = plan_conflict_recovery(&classify_conflicts("UU src/main.rs\n"));
    assert!(plan.admits(&GitConflictAction::Inspect, Some("src/main.rs")));
    assert!(plan.admits(&GitConflictAction::Abort, None));
    assert!(!plan.admits(&GitConflictAction::Inspect, Some("other.rs")));
}

#[test]
fn conflict_recovery_reduces_interactive_events_and_rejects_dead_ends() {
    let plan = plan_conflict_recovery(&classify_conflicts("UU src/main.rs\n"));
    let state = begin_conflict_recovery(plan);
    assert!(reduce_conflict_recovery(state.clone(), GitConflictRecoveryEvent::Completed).is_err());
    let state = reduce_conflict_recovery(
        state,
        GitConflictRecoveryEvent::PathSelected {
            path: "src/main.rs".into(),
        },
    )
    .unwrap();
    let state = reduce_conflict_recovery(
        state,
        GitConflictRecoveryEvent::ActionSelected {
            action: GitConflictAction::Resolve,
        },
    )
    .unwrap();
    let state = reduce_conflict_recovery(state, GitConflictRecoveryEvent::Completed).unwrap();
    assert_eq!(state.status, GitConflictRecoveryStatus::Completed);
    assert!(reduce_conflict_recovery(state, GitConflictRecoveryEvent::Cancelled).is_err());
}

#[test]
fn conflict_recovery_projection_replays_to_stable_terminal_rows() {
    let state = begin_conflict_recovery(plan_conflict_recovery(&classify_conflicts(
        "UU src/main.rs\nAA src/new.rs\n",
    )));
    let state = reduce_conflict_recovery(
        state,
        GitConflictRecoveryEvent::PathSelected {
            path: "src/main.rs".into(),
        },
    )
    .unwrap();
    let state = reduce_conflict_recovery(
        state,
        GitConflictRecoveryEvent::ActionSelected {
            action: GitConflictAction::Inspect,
        },
    )
    .unwrap();
    assert_eq!(
        state.terminal_lines(),
        [
            "Git recovery: Ready",
            "Conflicted paths: 2",
            "Conflict: src/main.rs",
            "Conflict: src/new.rs",
            "Selected path: src/main.rs",
            "Selected action: Inspect",
            "Allowed actions: 3",
        ]
    );
}
