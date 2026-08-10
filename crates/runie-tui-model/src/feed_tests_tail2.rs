#[test]
fn completed_tool_header_with_args_projects_web_search_site_count() {
    assert_eq!(
        super::completed_tool_header_with_args(
            "Web Search rust",
            "web_search",
            &serde_json::json!({}),
            &serde_json::json!("see https://docs.rs/a and https://crates.io/b"),
        ),
        "Web Search rust (2 sites)"
    );
}

#[test]
fn completed_tool_header_with_args_projects_memory_search_results() {
    assert_eq!(
            super::completed_tool_header_with_args(
                "Memory Search actors",
                "memory_search",
                &serde_json::json!({}),
                &serde_json::json!(
                    "### Result 1 (score: 0.72, source: global)\n**File:** /memory/MEMORY.md (lines 0-1)\n```\none\n```\n### Result 2 (score: 0.42, source: session)\n**File:** /memory/session.md (lines 2-3)\n```\ntwo\n```"
                ),
            ),
            "Memory Search actors (2 results)"
        );
}

#[test]
fn line_is_blank_pins_empty_text_predicate() {
    // Pin the smoke path: a line with empty text is blank, and the
    // free function and the method agree on the same definition.
    let blank = super::Line::new(super::LineKind::Separator, "");
    assert!(blank.is_blank());
    assert!(super::line_is_blank(&blank));
    // Pin the negative path: a line with non-empty text is not
    // blank, and the predicate returns false for both call sites.
    let non_blank = super::Line::new(super::LineKind::User, "hello");
    assert!(!non_blank.is_blank());
    assert!(!super::line_is_blank(&non_blank));
}

#[test]
fn find_first_containing_returns_first_match_index() {
    // Pin the smoke path: the first matching line is returned in
    // order, and a non-matching needle returns `None`.
    let lines = vec![
        super::Line::new(super::LineKind::User, "hello"),
        super::Line::new(super::LineKind::Assistant, "world"),
        super::Line::new(super::LineKind::Assistant, "hello again"),
    ];
    assert_eq!(super::find_first_containing(&lines, "hello"), Some(0));
    assert_eq!(super::find_first_containing(&lines, "world"), Some(1));
    assert_eq!(super::find_first_containing(&lines, "missing"), None);
}

#[test]
fn find_all_containing_returns_all_match_indices() {
    // Pin the smoke path: every matching line index is returned
    // in order, and a non-matching needle returns an empty vector.
    let lines = vec![
        super::Line::new(super::LineKind::User, "hello"),
        super::Line::new(super::LineKind::Assistant, "world"),
        super::Line::new(super::LineKind::Assistant, "hello again"),
    ];
    assert_eq!(super::find_all_containing(&lines, "hello"), vec![0, 2]);
    assert_eq!(
        super::find_all_containing(&lines, "missing"),
        Vec::<usize>::new()
    );
}
