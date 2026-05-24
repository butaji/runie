use similar::{TextDiff, ChangeTag};
use ratatui::{
    style::{Style, Color, Modifier},
    text::{Line, Span},
};

pub fn render_inline_diff(old_text: &str, new_text: &str) -> Vec<Line<'static>> {
    let diff = TextDiff::from_lines(old_text, new_text);
    let mut lines = Vec::new();

    for change in diff.iter_all_changes() {
        let (prefix, style) = match change.tag() {
            ChangeTag::Delete => ("-", Style::default().fg(Color::Red)),
            ChangeTag::Insert => ("+", Style::default().fg(Color::Green)),
            ChangeTag::Equal => (" ", Style::default().fg(Color::Gray)),
        };

        let line = Line::from(vec![
            Span::styled(prefix.to_string(), style.add_modifier(Modifier::BOLD)),
            Span::styled(change.value().trim_end().to_string(), style),
        ]);
        lines.push(line);
    }

    lines
}

pub fn compute_diff_stats(old_text: &str, new_text: &str) -> (usize, usize) {
    let diff = TextDiff::from_lines(old_text, new_text);
    let mut additions = 0;
    let mut deletions = 0;

    for change in diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Insert => additions += 1,
            ChangeTag::Delete => deletions += 1,
            ChangeTag::Equal => {}
        }
    }

    (additions, deletions)
}

pub fn has_meaningful_diff(old_text: &str, new_text: &str) -> bool {
    let diff = TextDiff::from_lines(old_text, new_text);
    for change in diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Insert | ChangeTag::Delete => return true,
            ChangeTag::Equal => {}
        }
    }
    false
}