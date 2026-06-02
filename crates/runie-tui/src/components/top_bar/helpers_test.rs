#[cfg(test)]
mod tests {
    use ratatui::{
        style::{Color, Style},
    };
    use crate::components::top_bar::{shorten_path, build_left_spans, TopBarViewModel};
    use crate::style::selection::GIT_BRANCH_SYMBOL;
    use crate::tui::state::TuiMode;

    // =============================================================================
    // shorten_path tests
    // =============================================================================

    #[test]
    fn test_shorten_path_inside_home() {
        let home = std::env::var("HOME").unwrap();
        let path = format!("{}/Code/GitHub/runie", home);
        let result = shorten_path(&path);
        assert!(result.starts_with("~/"), "Expected ~/ prefix, got: {}", result);
        assert!(!result.starts_with(&home), "Should not contain absolute home path: {}", result);
    }

    #[test]
    fn test_shorten_path_subdirectory() {
        let home = std::env::var("HOME").unwrap();
        let path = format!("{}/Code/GitHub/runie/src/components", home);
        let result = shorten_path(&path);
        assert_eq!(result, "~/Code/GitHub/runie/src/components");
    }

    #[test]
    fn test_shorten_path_outside_home() {
        // Path that is outside home directory
        let path = "/usr/local/lib/project";
        let result = shorten_path(path);
        assert_eq!(result, path, "Should return absolute path when outside home");
    }

    #[test]
    fn test_shorten_path_no_trailing_slash() {
        let home = std::env::var("HOME").unwrap();
        let path = format!("{}/Code/", home);
        let result = shorten_path(&path);
        assert!(!result.ends_with('/'), "Should not end with trailing slash: {}", result);
    }

    #[test]
    fn test_shorten_path_root_home() {
        let home = std::env::var("HOME").unwrap();
        let path = home.clone();
        let result = shorten_path(&path);
        assert_eq!(result, "~", "Root home should shorten to ~");
    }

    #[test]
    fn test_shorten_path_exactly_home_plus_slash() {
        let home = std::env::var("HOME").unwrap();
        let path = format!("{}/", home);
        let result = shorten_path(&path);
        assert_eq!(result, "~", "Home with trailing slash should shorten to ~");
    }

    #[test]
    fn test_shorten_path_partial_home_match() {
        // Path that starts with same prefix as home but is not inside home
        // e.g., home = /Users/admin, path = /Users/adminxyz/foo
        let home = std::env::var("HOME").unwrap();
        let path = format!("{}xyz/some/path", &home[..5]); // /User/somethingxyz
        let result = shorten_path(&path);
        assert_eq!(result, path, "Should not shorten if not actually inside home");
    }

    #[test]
    fn test_shorten_path_empty() {
        let result = shorten_path("");
        assert_eq!(result, "");
    }

    #[test]
    fn test_shorten_path_git_root() {
        let home = std::env::var("HOME").unwrap();
        // Simulate git root - a directory that could be a repo root
        let path = format!("{}/Code/GitHub/runie", home);
        let result = shorten_path(&path);
        // Git root should be shown as ~/Code/GitHub/runie
        assert!(result.starts_with("~/"));
    }

    // =============================================================================
    // build_left_spans tests
    // =============================================================================

    fn create_test_vm(repo: &str, branch: &str, path: &str) -> TopBarViewModel {
        TopBarViewModel {
            repo: repo.to_string(),
            branch: branch.to_string(),
            path: path.to_string(),
            context_window: 128_000,
            estimated_tokens: 0,
            agent_running: false,
            braille_frame: 0,
            mode: TuiMode::Chat,
        }
    }

    #[test]
    fn test_build_left_spans_with_repo_branch_and_path() {
        let vm = create_test_vm("runie", "main", "/Users/test/Code/project");
        let bright = Color::White;
        let dim_style = Style::new().fg(Color::DarkGray);
        let bg = Color::Black;

        let spans = build_left_spans(&vm, bright, Color::DarkGray, &dim_style, bg);

        // Should skip repo "runie" and show branch + path
        assert_eq!(spans.len(), 1);
        let content = spans[0].content.as_ref();
        assert!(content.contains("main"), "Should contain branch name");
        assert!(content.contains("project"), "Should contain shortened path");
    }

    #[test]
    fn test_build_left_spans_with_different_repo() {
        let vm = create_test_vm("myapp", "feature", "/Users/test/Code/myapp/src");
        let bright = Color::White;
        let dim_style = Style::new().fg(Color::DarkGray);
        let bg = Color::Black;

        let spans = build_left_spans(&vm, bright, Color::DarkGray, &dim_style, bg);

        assert_eq!(spans.len(), 2, "Should have repo + branch+path");
        // First span is repo
        assert_eq!(spans[0].content.as_ref(), "myapp");
    }

    #[test]
    fn test_build_left_spans_skip_runie_repo() {
        let vm = create_test_vm("runie", "main", "/Users/test/Code/runie");
        let bright = Color::White;
        let dim_style = Style::new().fg(Color::DarkGray);
        let bg = Color::Black;

        let spans = build_left_spans(&vm, bright, Color::DarkGray, &dim_style, bg);

        // Should skip "runie" repo name
        assert_eq!(spans.len(), 1);
        let content = spans[0].content.as_ref();
        assert!(content.contains("main"));
    }

    #[test]
    fn test_build_left_spans_no_branch_only_path() {
        let vm = create_test_vm("", "", "/Users/test/Code/project/src");
        let bright = Color::White;
        let dim_style = Style::new().fg(Color::DarkGray);
        let bg = Color::Black;

        let spans = build_left_spans(&vm, bright, Color::DarkGray, &dim_style, bg);

        assert_eq!(spans.len(), 1);
        // Should show shortened path without branch prefix
        let content = spans[0].content.as_ref();
        assert!(content.contains("project"));
    }

    #[test]
    fn test_build_left_spans_empty_vm() {
        let vm = create_test_vm("", "", "");
        let bright = Color::White;
        let dim_style = Style::new().fg(Color::DarkGray);
        let bg = Color::Black;

        let spans = build_left_spans(&vm, bright, Color::DarkGray, &dim_style, bg);

        assert!(spans.is_empty());
    }

    #[test]
    fn test_build_left_spans_branch_only_no_path() {
        let vm = create_test_vm("myapp", "main", "");
        let bright = Color::White;
        let dim_style = Style::new().fg(Color::DarkGray);
        let bg = Color::Black;

        let spans = build_left_spans(&vm, bright, Color::DarkGray, &dim_style, bg);

        assert_eq!(spans.len(), 2); // repo + branch
        let branch_span = &spans[1];
        let content = branch_span.content.as_ref();
        // Branch only, no path
        assert!(content.contains("main"));
        assert!(content.contains(GIT_BRANCH_SYMBOL));
    }

    #[test]
    fn test_build_left_spans_git_branch_symbol_present() {
        let vm = create_test_vm("myapp", "feature/test", "/Users/test/Code/myapp");
        let bright = Color::White;
        let dim_style = Style::new().fg(Color::DarkGray);
        let bg = Color::Black;

        let spans = build_left_spans(&vm, bright, Color::DarkGray, &dim_style, bg);

        let combined_content = spans.iter()
            .map(|s| s.content.as_ref())
            .collect::<Vec<_>>()
            .join("");

        assert!(combined_content.contains(GIT_BRANCH_SYMBOL), "Should contain git branch symbol");
    }

    #[test]
    fn test_build_left_spans_path_shortened_to_home() {
        let home = std::env::var("HOME").unwrap();
        let path = format!("{}/Code/myapp", home);
        let vm = create_test_vm("myapp", "main", &path);
        let bright = Color::White;
        let dim_style = Style::new().fg(Color::DarkGray);
        let bg = Color::Black;

        let spans = build_left_spans(&vm, bright, Color::DarkGray, &dim_style, bg);

        let combined_content = spans.iter()
            .map(|s| s.content.as_ref())
            .collect::<Vec<_>>()
            .join("");

        assert!(combined_content.contains("~/"), "Path should be shortened to ~/...");
        assert!(!combined_content.contains(&home), "Should not contain absolute home path");
    }

    #[test]
    fn test_build_left_spans_no_leading_space_before_branch_symbol() {
        let vm = create_test_vm("myapp", "main", "/Users/test/Code/myapp");
        let bright = Color::White;
        let dim_style = Style::new().fg(Color::DarkGray);
        let bg = Color::Black;

        let spans = build_left_spans(&vm, bright, Color::DarkGray, &dim_style, bg);

        // Find the branch span
        let branch_span = spans.iter()
            .find(|s| s.content.as_ref().contains("main"))
            .expect("Should have branch span");

        let content = branch_span.content.as_ref();
        // Should start with branch symbol, not a space
        assert!(content.starts_with(GIT_BRANCH_SYMBOL), "Should start with git branch symbol");
    }

    #[test]
    fn test_build_left_spans_space_before_path() {
        let vm = create_test_vm("myapp", "main", "/Users/test/Code/myapp/src");
        let bright = Color::White;
        let dim_style = Style::new().fg(Color::DarkGray);
        let bg = Color::Black;

        let spans = build_left_spans(&vm, bright, Color::DarkGray, &dim_style, bg);

        let branch_span = spans.iter()
            .find(|s| s.content.as_ref().contains("main"))
            .expect("Should have branch span");

        let content = branch_span.content.as_ref();
        // There should be a space between branch and path
        assert!(content.contains(" ~/") || content.ends_with("main"), 
            "Should have space before path: {}", content);
    }

    #[test]
    fn test_build_left_spans_all_fields_empty() {
        let vm = TopBarViewModel {
            repo: String::new(),
            branch: String::new(),
            path: String::new(),
            context_window: 0,
            estimated_tokens: 0,
            agent_running: false,
            braille_frame: 0,
            mode: TuiMode::Chat,
        };

        let spans = build_left_spans(&vm, Color::White, Color::DarkGray, &Style::new(), Color::Black);
        assert!(spans.is_empty());
    }

    #[test]
    fn test_build_left_spans_git_root_path_shown() {
        let home = std::env::var("HOME").unwrap();
        // When path IS the git root (e.g., opened folder is the repo root)
        let path = format!("{}/Code/GitHub/runie", home);
        let vm = create_test_vm("runie", "main", &path);
        let bright = Color::White;
        let dim_style = Style::new().fg(Color::DarkGray);
        let bg = Color::Black;

        let spans = build_left_spans(&vm, bright, Color::DarkGray, &dim_style, bg);

        assert_eq!(spans.len(), 1);
        let content = spans[0].content.as_ref();
        // Should show branch with path that points to git root
        assert!(content.contains("main"));
        // Git root should be visible as ~ path
        assert!(content.contains("~/Code/GitHub/runie") || content.contains("runie"));
    }
}
