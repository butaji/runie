//! Goal Mode side pane — shows goal progress, tasks, and phase.
//!
//! Renders alongside the chat messages when goal mode is active.

use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use runie_core::{goal::GoalState, GoalPhase, GoalStatus, Snapshot};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};
use super::utils::truncate_to_width;

/// Width of the goal pane.
#[allow(dead_code)]
const GOAL_PANE_WIDTH: u16 = 32;

/// Render the goal pane into the given area.
pub(crate) fn render_goal_pane(f: &mut Frame, snap: &Snapshot, area: Rect) {
    if area.width < 6 || area.height < 3 {
        return;
    }

    draw_pane_box(f, area);

    let inner = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };
    let content_width = inner.width.saturating_sub(2);

    if let Some(ref goal) = snap.goal_state {
        render_goal_content(f, goal, inner.x, inner.y, content_width);
    } else {
        render_no_goal(f, inner.x, inner.y, content_width);
    }
}

fn draw_pane_box(f: &mut Frame, area: Rect) {
    let border_style = crate::theme::style_border();
    let bg = crate::theme::color_bg();
    let base = border_style.bg(bg);

    let top = "┌".to_string() + &" ".repeat(area.width.saturating_sub(2) as usize) + "◎";
    let bottom = "└".to_string() + &" ".repeat(area.width.saturating_sub(2) as usize) + "┘";

    f.buffer_mut().set_string(area.x, area.y, top, base);

    for row in 1..area.height.saturating_sub(1) {
        let y = area.y + row;
        f.buffer_mut()
            .set_string(area.x, y, "│", border_style.bg(bg));
        f.buffer_mut()
            .set_string(area.x + area.width - 1, y, "│", border_style.bg(bg));
    }

    f.buffer_mut()
        .set_string(area.x + area.width - 1, area.y + area.height - 1, bottom.clone(), base);
    f.buffer_mut()
        .set_string(area.x, area.y + area.height - 1, bottom, base);
}

fn render_no_goal(f: &mut Frame, x: u16, y: u16, width: u16) {
    let style = crate::theme::style_hint();
    let text = Line::from(vec![Span::styled("No active goal", style)]);
    f.render_widget(Paragraph::new(text), Rect { x, y, width, height: 1 });
}

fn render_goal_content(
    f: &mut Frame,
    goal: &GoalState,
    x: u16,
    y: u16,
    width: u16,
) {
    // Header with phase and status
    render_header(f, goal, x, y, width);

    // Progress bar
    let progress_y = y + 2;
    render_progress_bar(f, goal, x, progress_y, width);

    // Objective
    let objective_y = progress_y + 2;
    render_objective(f, goal, x, objective_y, width);

    // Tasks
    let tasks_y = objective_y + 2;
    render_tasks(f, goal, x, tasks_y, width);
}

fn render_header(f: &mut Frame, goal: &GoalState, x: u16, y: u16, width: u16) {
    let header_style = crate::theme::style_tasks_pane_header();
    let phase = match goal.phase {
        GoalPhase::Idle => "Idle",
        GoalPhase::Planning => "Planning",
        GoalPhase::Executing => "Executing",
        GoalPhase::Paused => "Paused",
        GoalPhase::Completed => "Done",
        GoalPhase::Failed => "Failed",
    };

    let status_icon = match goal.status {
        GoalStatus::Active => "●",
        GoalStatus::UserPaused | GoalStatus::BackOffPaused | GoalStatus::NoProgressPaused => "⏸",
        GoalStatus::InfraPaused | GoalStatus::Blocked => "■",
        GoalStatus::BudgetLimited => "◐",
        GoalStatus::Complete => "✓",
    };

    let header_text = format!("▾ Goal {} {}", status_icon, phase);
    let header_width = header_text.width();
    let fill_len = (width as usize).saturating_sub(header_width);

    let spans = vec![
        Span::styled("   ", header_style),
        Span::styled(header_text, header_style),
        Span::styled(" ".repeat(fill_len), header_style),
    ];

    f.render_widget(
        Paragraph::new(Line::from(spans)),
        Rect { x, y, width, height: 1 },
    );
}

fn render_progress_bar(f: &mut Frame, goal: &GoalState, x: u16, y: u16, width: u16) {
    let total = goal.checkpoints.len().max(1);
    let completed = goal.checkpoints.iter().filter(|c| c.completed).count();
    let progress = completed as f32 / total as f32;

    let bar_width = (width as usize).saturating_sub(4);
    let filled = ((progress * bar_width as f32) as usize).min(bar_width);

    let completed_color = crate::theme::color_subagent_completed();
    let running_color = crate::theme::color_subagent_running();

    let bar_style = if progress >= 1.0 {
        completed_color
    } else {
        running_color
    };

    let bar = format!(
        " {} [{}{}]",
        completed,
        "█".repeat(filled),
        "░".repeat(bar_width - filled)
    );

    let span = Span::styled(bar, bar_style);
    f.render_widget(Paragraph::new(Line::from(span)), Rect { x, y, width, height: 1 });
}

fn render_objective(f: &mut Frame, goal: &GoalState, x: u16, y: u16, width: u16) {
    use ratatui::style::Style;
    let objective_style = Style::default().fg(crate::theme::color_fg()).bold();
    let truncated = truncate_to_width(&goal.objective, (width as usize).saturating_sub(4));

    let spans = vec![
        Span::styled("   ", objective_style),
        Span::styled(truncated, objective_style),
    ];

    f.render_widget(
        Paragraph::new(Line::from(spans)),
        Rect { x, y, width, height: 1 },
    );
}

fn render_tasks(f: &mut Frame, goal: &GoalState, x: u16, start_y: u16, width: u16) {
    use ratatui::style::Style;
    let dim_style = crate::theme::style_hint();
    let completed_style = Style::default().fg(crate::theme::color_subagent_completed());
    let running_style = Style::default().fg(crate::theme::color_subagent_running());

    for (idx, checkpoint) in goal.checkpoints.iter().enumerate() {
        let y = start_y + idx as u16;
        if y >= start_y + 10 {
            break; // Max 10 tasks visible
        }

        let (icon, style) = if checkpoint.completed {
            ("✓", completed_style)
        } else {
            ("○", running_style)
        };

        let desc = truncate_to_width(
            &checkpoint.description,
            (width as usize).saturating_sub(6),
        );
        let line = format!("   {} {}", icon, desc);

        let span = Span::styled(line, if checkpoint.completed { dim_style } else { style });
        f.render_widget(
            Paragraph::new(Line::from(span)),
            Rect { x, y, width, height: 1 },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};
    use runie_core::goal::GoalState;
    use super::super::utils::truncate_to_width;

    fn sample_goal() -> GoalState {
        let mut goal = GoalState::new(
            "test-goal-id".to_string(),
            "Test the feature implementation".to_string(),
            Some(10000),
        );
        goal.add_checkpoint("1-test", "Write tests");
        goal.add_checkpoint("2-review", "Review code");
        goal.checkpoints[0].completed = true;
        goal
    }

    fn make_snap_with_goal(goal: GoalState) -> Snapshot {
        Snapshot {
            goal_state: Some(goal),
            ..Default::default()
        }
    }

    #[test]
    fn goal_pane_renders_header() {
        let goal = sample_goal();
        let snap = make_snap_with_goal(goal);

        let backend = TestBackend::new(150, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let area = Rect { x: 100, y: 1, width: GOAL_PANE_WIDTH, height: 15 };

        terminal.draw(|f| render_goal_pane(f, &snap, area)).unwrap();

        let content: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();

        assert!(content.contains("Goal"), "should show Goal header");
        assert!(content.contains("Executing"), "should show phase");
    }

    #[test]
    fn goal_pane_renders_objective() {
        let goal = sample_goal();
        let snap = make_snap_with_goal(goal);

        let backend = TestBackend::new(150, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let area = Rect { x: 100, y: 1, width: GOAL_PANE_WIDTH, height: 15 };

        terminal.draw(|f| render_goal_pane(f, &snap, area)).unwrap();

        let content: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();

        assert!(content.contains("Test the feature"), "should show objective");
    }

    #[test]
    fn goal_pane_renders_progress() {
        let goal = sample_goal();
        let snap = make_snap_with_goal(goal);

        let backend = TestBackend::new(150, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let area = Rect { x: 100, y: 1, width: GOAL_PANE_WIDTH, height: 15 };

        terminal.draw(|f| render_goal_pane(f, &snap, area)).unwrap();

        let content: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();

        assert!(content.contains('1'), "should show completed count");
    }

    #[test]
    fn truncate_handles_short_text() {
        assert_eq!(truncate_to_width("short", 10), "short");
    }

    #[test]
    fn truncate_handles_empty() {
        assert_eq!(truncate_to_width("", 10), "");
        assert_eq!(truncate_to_width("text", 0), "");
    }

    #[test]
    fn truncate_adds_ellipsis() {
        let result = truncate_to_width("very long text", 8);
        assert!(result.ends_with('…'), "should end with ellipsis");
        assert!(result.width() <= 8);
    }
}
