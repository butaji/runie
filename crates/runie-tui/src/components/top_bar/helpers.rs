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
    if !vm.repo.is_empty() {
        parts.push(Span::styled(&vm.repo, Style::default().fg(bright).bg(bg)));
    }
    if !vm.branch.is_empty() {
        // Add git branch symbol before branch name
        parts.push(Span::styled(GIT_BRANCH_SYMBOL.to_string(), dim_style.clone().bg(bg)));
        parts.push(Span::styled(&vm.branch, dim_style.clone().bg(bg)));
    }
    if !vm.path.is_empty() {
        let short_path = shorten_path(&vm.path);
        parts.push(Span::styled(format!("  {}", short_path), dim_style.clone().bg(bg)));
    }
    parts
}
