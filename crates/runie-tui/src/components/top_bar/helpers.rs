//! Top bar render helpers.

use ratatui::{
    style::{Color, Style},
    text::Span,
};
use crate::components::top_bar::TopBarViewModel;

/// Git branch symbol (Powerline style)
const GIT_BRANCH_SYMBOL: char = '\u{E0A0}';

/// Shorten a path to be relative to home directory
pub fn shorten_path(path: &str) -> String {
    if let Ok(home) = std::env::var("HOME") {
        if path.starts_with(&home) {
            let suffix = &path[home.len()..];
            if suffix.is_empty() || suffix.starts_with('/') {
                return format!("~{}", suffix);
            }
        }
    }
    path.to_string()
}

pub fn build_left_spans<'a>(
    vm: &'a TopBarViewModel,
    bright: Color,
    _dim: Color,
    dim_style: &'a Style,
    bg: Color,
) -> Vec<Span<'a>> {
    let mut parts = Vec::new();
    // Skip repo if it equals "runie" to avoid showing app name in header
    if !vm.repo.is_empty() && vm.repo != "runie" {
        parts.push(Span::styled(&vm.repo, Style::default().fg(bright).bg(bg)));
    }
    // Build combined branch + path span with proper formatting
    if !vm.branch.is_empty() {
        let short_path = shorten_path(&vm.path);
        let path_str = if short_path.is_empty() {
            String::new()
        } else {
            format!(" {}/", short_path)
        };
        // Format: " branch ~/path" (no leading space - padding handled by padded_area)
        let combined = format!("{} {}{}", GIT_BRANCH_SYMBOL, &vm.branch, path_str);
        parts.push(Span::styled(combined, dim_style.clone().bg(bg)));
    } else if !vm.path.is_empty() {
        // No branch, just show path (no leading space - padding handled by padded_area)
        let short_path = shorten_path(&vm.path);
        parts.push(Span::styled(format!("{}/", short_path), dim_style.clone().bg(bg)));
    }
    parts
}
