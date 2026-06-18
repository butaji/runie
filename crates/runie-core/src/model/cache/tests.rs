use super::*;

#[test]
fn input_title_default_is_base() {
    let title = build_input_title(
        "openai",
        "gpt-4o",
        &crate::orchestrator::ExecutionMode::Solo,
        false,
    );
    assert_eq!(title, "openai/gpt-4o");
}

#[test]
fn input_title_includes_team_mode() {
    let title = build_input_title(
        "openai",
        "gpt-4o",
        &crate::orchestrator::ExecutionMode::Team,
        false,
    );
    assert!(title.contains("Team"), "title should contain Team: {title}");
}

#[test]
fn input_title_includes_read_only() {
    let title = build_input_title(
        "openai",
        "gpt-4o",
        &crate::orchestrator::ExecutionMode::Solo,
        true,
    );
    assert!(
        title.contains("read-only"),
        "title should contain read-only: {title}"
    );
}

#[test]
fn input_title_includes_team_and_read_only() {
    let title = build_input_title(
        "openai",
        "gpt-4o",
        &crate::orchestrator::ExecutionMode::Team,
        true,
    );
    assert!(title.contains("Team"), "title should contain Team: {title}");
    assert!(
        title.contains("read-only"),
        "title should contain read-only: {title}"
    );
}

#[test]
fn input_title_no_mode_suffix_for_default() {
    let title = build_input_title(
        "anthropic",
        "claude-3-5-sonnet",
        &crate::orchestrator::ExecutionMode::Solo,
        false,
    );
    assert!(
        !title.contains("Solo"),
        "Solo mode should not appear: {title}"
    );
    assert!(
        !title.contains("read-only"),
        "read-only should not appear: {title}"
    );
}

#[test]
fn input_title_uses_provider_and_model() {
    let title = build_input_title(
        "google",
        "gemini-2.5",
        &crate::orchestrator::ExecutionMode::Solo,
        false,
    );
    assert!(
        title.starts_with("google/"),
        "title should start with provider: {title}"
    );
    assert!(
        title.contains("gemini-2.5"),
        "title should contain model: {title}"
    );
}
