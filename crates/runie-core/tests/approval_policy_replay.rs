use runie_core::tools::{approval_decision, ApprovalDecision, ApprovalMode};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Case {
    mode: String,
    tool: String,
    decision: String,
}

fn mode(value: &str) -> ApprovalMode {
    match value {
        "ask" => ApprovalMode::Ask,
        "auto" => ApprovalMode::Auto,
        "deny" => ApprovalMode::Deny,
        "yolo" => ApprovalMode::Yolo,
        other => panic!("unknown mode {other}"),
    }
}

#[test]
fn yaml_policy_matrix_replays_pure_decisions() {
    let cases: Vec<Case> = serde_yaml::from_str(include_str!("fixtures/approval-policy.yaml"))
        .expect("approval fixture");
    for case in cases {
        let decision = approval_decision(mode(&case.mode), &case.tool);
        let matches = match case.decision.as_str() {
            "allow" => decision == ApprovalDecision::Allow,
            "ask" => matches!(decision, ApprovalDecision::Ask { .. }),
            "deny" => matches!(decision, ApprovalDecision::Deny { .. }),
            other => panic!("unknown decision {other}"),
        };
        assert!(matches, "case failed: {case:?}");
    }
}
