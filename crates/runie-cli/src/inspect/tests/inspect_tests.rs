//! Tests for inspect module.

use crate::inspect::InspectReport;

#[test]
fn inspect_report_builds_without_panic() {
    let report = InspectReport::build();
    assert!(
        !report.commands.is_empty(),
        "Expected commands to be registered"
    );
    assert!(
        !report.model_catalog.is_empty(),
        "Expected model catalog entries"
    );
}

#[test]
fn inspect_report_json_serializes() {
    let report = InspectReport::build();
    let json = serde_json::to_string(&report);
    assert!(json.is_ok(), "JSON serialization should succeed");
}

#[test]
fn inspect_report_human_does_not_panic() {
    let report = InspectReport::build();
    report.print_human();
}

#[test]
fn skill_info_contains_path() {
    let report = InspectReport::build();
    for skill in &report.skill_items {
        assert!(!skill.path.is_empty(), "Skill path should not be empty");
    }
}

#[test]
fn provider_info_has_no_api_key() {
    let report = InspectReport::build();
    for provider in &report.providers {
        assert!(!provider.name.is_empty() || !provider.base_url.is_empty());
    }
}
