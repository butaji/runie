use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
};
use crate::theme::ThemeColors;
use crate::tui::state::TopBarState;

// ─── Top Bar ViewModel ───────────────────────────────────────────────────────
/// Exactly what the top bar should display. No logic, just data.
#[derive(Clone)]
pub struct TopBarViewModel {
    pub repo: String,
    pub branch: String,
    pub path: String,
    pub right_items: Vec<TopBarRightItem>,
}

#[derive(Clone)]
pub enum TopBarRightItem {
    Badge(String),
    PercentageText(f32),
    PercentageBar(f32),
    ChecksPassed(usize, usize),  // passed, total
    PercentageValue(f32),
    /// Combined checks + percentage with progress bar
    /// Format: {passed} ✓ {pct}% |{bar}|
    ProgressBar { passed: usize, total: usize, pct: f32 },
}

impl TopBarViewModel {
    /// Build from app state (real mode)
    pub fn from_state(state: &TopBarState) -> Self {
        let mut right_items = Vec::new();

        // Context badges
        for badge in &state.context_badges {
            right_items.push(TopBarRightItem::Badge(badge.clone()));
        }

        // Context % text
        if let Some(pct) = state.context_pct {
            right_items.push(TopBarRightItem::PercentageText(pct));
        }

        // Context % bar
        if let Some(pct) = state.context_bar_pct {
            right_items.push(TopBarRightItem::PercentageBar(pct));
        }

        // Fallback: checks + percentage combined
        if right_items.is_empty() {
            match (state.checks_passed, state.checks_total, state.percentage) {
                (Some(passed), Some(total), Some(pct)) => {
                    right_items.push(TopBarRightItem::ProgressBar { passed, total, pct });
                }
                (Some(passed), Some(total), None) => {
                    right_items.push(TopBarRightItem::ChecksPassed(passed, total));
                }
                (None, None, Some(pct)) => {
                    right_items.push(TopBarRightItem::PercentageValue(pct));
                }
                _ => {}
            }
        }

        Self {
            repo: state.repo.clone(),
            branch: state.branch.clone(),
            path: state.path.clone(),
            right_items,
        }
    }

}

// ─── Rendering ──────────────────────────────────────────────────────────────

fn build_context_bar_counts(pct: f32) -> (usize, usize) {
    let filled = if pct > 0.0 {
        ((pct / 100.0 * 10.0).round() as usize).max(1).min(10)
    } else {
        0
    };
    let empty = 10 - filled;
    (filled, empty)
}

fn build_context_bar(pct: f32) -> String {
    let (filled, empty) = build_context_bar_counts(pct);
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

fn build_left_spans(vm: &TopBarViewModel, bright: Color, dim: Color) -> Vec<Span<'_>> {
    let mut parts = Vec::new();

    if !vm.repo.is_empty() {
        parts.push(Span::styled(&vm.repo, Style::default().fg(bright)));
    }
    if !vm.branch.is_empty() {
        parts.push(Span::styled("/", Style::default().fg(dim)));
        parts.push(Span::styled(&vm.branch, Style::default().fg(dim)));
    }
    if !vm.path.is_empty() {
        parts.push(Span::styled(format!("  {}", vm.path), Style::default().fg(dim)));
    }

    parts
}

fn build_right_spans(vm: &TopBarViewModel, bright: Color, dim: Color) -> Vec<Span<'_>> {
    let mut parts = Vec::new();

    for (i, item) in vm.right_items.iter().enumerate() {
        if i > 0 {
            parts.push(Span::styled(" ", Style::default().fg(dim)));
        }

        match item {
            TopBarRightItem::Badge(text) => {
                parts.push(Span::styled(format!("[{}]", text), Style::default().fg(bright)));
            }
            TopBarRightItem::PercentageText(pct) => {
                parts.push(Span::styled(format!("{:.0}% of root agent", pct), Style::default().fg(bright)));
            }
            TopBarRightItem::PercentageBar(pct) => {
                let bar = build_context_bar(*pct);
                parts.push(Span::styled(bar, Style::default().fg(bright)));
            }
            TopBarRightItem::ChecksPassed(passed, total) => {
                let pct = *passed as f32 / *total as f32 * 100.0;
                let bar = build_context_bar(pct);
                parts.push(Span::styled(format!("{} ", passed), Style::default().fg(bright)));
                parts.push(Span::styled("✓ ", Style::default().fg(bright)));
                parts.push(Span::styled(bar, Style::default().fg(bright)));
                parts.push(Span::styled(" │", Style::default().fg(dim)));
            }
            TopBarRightItem::PercentageValue(pct) => {
                let bar = build_context_bar(*pct);
                parts.push(Span::styled(format!("{:.2}%", pct), Style::default().fg(bright)));
                parts.push(Span::styled(format!(" {}", bar), Style::default().fg(bright)));
                parts.push(Span::styled(" │", Style::default().fg(dim)));
            }
            TopBarRightItem::ProgressBar { passed, pct, .. } => {
                let (filled, empty) = build_context_bar_counts(*pct);
                parts.push(Span::styled(format!("{} ", passed), Style::default().fg(bright)));
                parts.push(Span::styled("✓ ", Style::default().fg(bright)));
                parts.push(Span::styled(format!("{:.1}%", pct), Style::default().fg(dim)));
                parts.push(Span::styled(" ", Style::default().fg(dim)));
                parts.push(Span::styled("█".repeat(filled), Style::default().fg(bright)));
                parts.push(Span::styled("░".repeat(empty), Style::default().fg(dim)));
            }
        }
    }

    parts
}

pub fn render_top_bar(vm: &TopBarViewModel, area: Rect, buf: &mut Buffer, colors: &ThemeColors) {
    let x = area.x + 1;
    let bright = colors.text_secondary;
    let dim = colors.text_dim;

    // Line 1: left parts
    let left_parts = build_left_spans(vm, bright, dim);
    if !left_parts.is_empty() {
        buf.set_line(x, area.y, &Line::from(left_parts), area.width - 2);
    }

    // Line 1: right parts
    let right_parts = build_right_spans(vm, bright, dim);
    if !right_parts.is_empty() {
        let right_line = Line::from(right_parts);
        let right_width: usize = right_line.spans.iter().map(|s| s.width()).sum();
        let right_x = area.x + area.width.saturating_sub(right_width as u16 + 1);
        if right_x > x {
            buf.set_line(right_x, area.y, &right_line, area.width);
        }
    }

    // Line 2: empty (padding)
}
