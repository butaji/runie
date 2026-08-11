use runie_core::session::{context_report, CompactionSettings};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ContextReportFixture {
    context_tokens: u64,
    context_window: u64,
    settings: CompactionSettings,
    expected: Vec<String>,
}

#[test]
fn yaml_context_policy_replays_to_the_renderer_neutral_report() {
    let fixture: ContextReportFixture =
        serde_yaml::from_str(include_str!("fixtures/context-report.yaml"))
            .expect("context report fixture");
    let report = context_report(
        fixture.context_tokens,
        fixture.context_window,
        fixture.settings,
        None,
    );
    assert_eq!(report.terminal_lines(), fixture.expected);
}

#[test]
fn yaml_unknown_context_window_replays_to_disabled_recovery() {
    let fixture: ContextReportFixture =
        serde_yaml::from_str(include_str!("fixtures/context-recovery-disabled.yaml"))
            .expect("disabled context report fixture");
    let report = context_report(
        fixture.context_tokens,
        fixture.context_window,
        fixture.settings,
        None,
    );
    assert_eq!(report.terminal_lines(), fixture.expected);
}

#[test]
fn yaml_disabled_policy_replays_without_recovery() {
    let fixture: ContextReportFixture =
        serde_yaml::from_str(include_str!("fixtures/context-policy-disabled.yaml"))
            .expect("disabled policy fixture");
    let report = context_report(
        fixture.context_tokens,
        fixture.context_window,
        fixture.settings,
        None,
    );
    assert_eq!(report.terminal_lines(), fixture.expected);
}
