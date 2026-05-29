use super::*;

#[test]
fn test_e2e_set_git_info() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::SetGitInfo {
        repo: "myrepo".to_string(),
        branch: "main".to_string(),
        path: "src/lib.rs".to_string(),
    });

    assert_eq!(state.top_bar.repo, "myrepo");
    assert_eq!(state.top_bar.branch, "main");
    assert_eq!(state.top_bar.path, "src/lib.rs");
}

#[test]
fn test_e2e_set_top_bar_checks() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::SetTopBarMockChecks {
        checks_passed: Some(8),
        checks_total: Some(10),
        percentage: Some(80.0),
        context_badges: vec!["rust".to_string(), "fmt".to_string()],
    });

    assert_eq!(state.top_bar.checks_passed, Some(8));
    assert_eq!(state.top_bar.checks_total, Some(10));
    assert_eq!(state.top_bar.percentage, Some(80.0));
    assert_eq!(state.top_bar.context_badges, vec!["rust", "fmt"]);
}
