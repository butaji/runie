use super::*;

#[test]
fn input_title_default_is_base() {
    let title = build_input_title("openai", "gpt-4o", false);
    assert_eq!(title, "openai/gpt-4o");
}

#[test]
fn input_title_includes_read_only() {
    let title = build_input_title("openai", "gpt-4o", true);
    assert!(
        title.contains("read-only"),
        "title should contain read-only: {title}"
    );
}

#[test]
fn input_title_no_suffix_for_default() {
    let title = build_input_title("anthropic", "claude-3-5-sonnet", false);
    assert!(
        !title.contains("read-only"),
        "read-only should not appear: {title}"
    );
}

#[test]
fn input_title_uses_provider_and_model() {
    let title = build_input_title("google", "gemini-2.5", false);
    assert!(
        title.starts_with("google/"),
        "title should start with provider: {title}"
    );
    assert!(
        title.contains("gemini-2.5"),
        "title should contain model: {title}"
    );
}
