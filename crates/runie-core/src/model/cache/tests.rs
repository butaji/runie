use super::*;
use crate::Event;

#[test]
fn auth_providers_use_cached_list() {
    let mut state = AppState::default();
    state.set_auth_providers(vec!["openai".into(), "anthropic".into()]);
    let snap = state.snapshot();
    let providers: Vec<_> = snap.auth_providers.iter().cloned().collect();
    assert_eq!(providers, vec!["openai", "anthropic"]);
}

#[test]
fn snapshot_rebuilds_palette_cache_without_blocking_io() {
    let mut state = AppState::default();
    state.skills = vec![crate::skills::Skill {
        name: "rust".into(),
        description: "rust helper".into(),
        context: String::new(),
        user_invocable: true,
        file_path: std::path::PathBuf::new(),
    }];
    state.update(Event::ToggleCommandPalette);
    let snap = state.snapshot();
    assert!(
        snap.palette_items.iter().any(|(n, _, c)| n == "rust" && c == "Skill"),
        "skill should appear in palette items"
    );
}

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
