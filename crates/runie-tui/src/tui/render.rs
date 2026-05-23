use ratatui::{
    layout::Rect,
    buffer::Buffer,
    style::Style,
    text::{Line, Span},
};
use crate::theme::ThemeWrapper;
use crate::tui::state::{TuiMode, TopBarState, RenderState};
use crate::components::panel::Panel;

// ─── Top Bar ─────────────────────────────────────────────────────────────────

fn build_left_parts(top_bar: &TopBarState, text_primary: ratatui::style::Color, text_muted: ratatui::style::Color) -> Vec<Span<'_>> {
    let mut left_parts: Vec<Span> = Vec::new();

    if !top_bar.repo.is_empty() {
        left_parts.push(Span::styled(&top_bar.repo, Style::default().fg(text_primary)));
    }
    if !top_bar.branch.is_empty() {
        left_parts.push(Span::styled("/", Style::default().fg(text_muted)));
        left_parts.push(Span::styled(&top_bar.branch, Style::default().fg(text_muted)));
    }
    if !top_bar.path.is_empty() {
        left_parts.push(Span::styled(format!("  {}", top_bar.path),
            Style::default().fg(text_muted)));
    }

    left_parts
}

fn build_right_parts(top_bar: &TopBarState, text_secondary: ratatui::style::Color, text_muted: ratatui::style::Color) -> Vec<Span<'_>> {
    let mut right_parts: Vec<Span> = Vec::new();

    if let (Some(passed), Some(total)) = (top_bar.checks_passed, top_bar.checks_total) {
        right_parts.push(Span::styled(format!("{} ", passed), Style::default().fg(text_secondary)));
        right_parts.push(Span::styled("✓ ", Style::default().fg(text_muted)));

        let pct = passed as f32 / total.max(1) as f32;
        let filled = (pct * 10.0).round() as usize;
        let empty = 10 - filled;
        let bar = format!("{}{}", "█".repeat(filled), "░".repeat(empty));
        right_parts.push(Span::styled(bar, Style::default().fg(text_secondary)));
        right_parts.push(Span::styled(" │", Style::default().fg(text_muted)));
    } else if let Some(pct) = top_bar.percentage {
        right_parts.push(Span::styled(format!("{:.2}%", pct), Style::default().fg(text_secondary)));

        let filled = (pct / 100.0 * 10.0).round() as usize;
        let empty = 10 - filled;
        let bar = format!("{}{}", "█".repeat(filled), "░".repeat(empty));
        right_parts.push(Span::styled(format!(" {}", bar), Style::default().fg(text_secondary)));
        right_parts.push(Span::styled(" │", Style::default().fg(text_muted)));
    }

    right_parts
}

pub fn render_top_bar(state: &RenderState, area: Rect, buf: &mut Buffer, theme: &ThemeWrapper) {
    use ratatui::text::Line;

    let x = area.x + 1;
    let text_primary: ratatui::style::Color = theme.color("text.primary").into();
    let text_secondary: ratatui::style::Color = theme.color("text.secondary").into();
    let text_muted: ratatui::style::Color = theme.color("text.muted").into();

    let left_parts = build_left_parts(&state.top_bar, text_primary, text_muted);
    if !left_parts.is_empty() {
        let line = Line::from(left_parts);
        buf.set_line(x, area.y, &line, area.width - 2);
    }

    let right_parts = build_right_parts(&state.top_bar, text_secondary, text_muted);
    if !right_parts.is_empty() {
        let right_line = Line::from(right_parts);
        let right_width: usize = right_line.spans.iter().map(|s| s.width()).sum();
        let right_x = area.x + area.width.saturating_sub(right_width as u16 + 1);
        if right_x > x {
            buf.set_line(right_x, area.y, &right_line, area.width);
        }
    }
}

// ─── Status Bar ───────────────────────────────────────────────────────────────

const NAV_KEYS: &[(&str, &str)] = &[("Esc", "close"), ("j/k", "navigate"), ("Enter", "select")];
const ARROW_KEYS: &[(&str, &str)] = &[("Esc", "close"), ("↑↓", "navigate"), ("Enter", "jump")];

pub(crate) fn get_status_items(mode: &TuiMode) -> Vec<(&'static str, &'static str)> {
    match *mode {
        TuiMode::Chat => vec![("Enter", "send"), ("^b", "sidebar"), ("^k", "cmd"), ("^q", "quit")],
        TuiMode::Overlay | TuiMode::Select => NAV_KEYS.to_vec(),
        TuiMode::Permission => vec![("y", "confirm"), ("n", "cancel"), ("a", "always"), ("s", "skip")],
        TuiMode::CommandPalette => vec![("Esc", "close"), ("Enter", "select"), ("↑↓", "navigate")],
        TuiMode::DiffViewer => vec![("q", "close"), ("j/k", "scroll")],
        TuiMode::SessionTree => ARROW_KEYS.to_vec(),
        TuiMode::Onboarding => vec![("Esc", "back"), ("↑↓", "navigate"), ("Enter", "next")],
    }
}

fn build_center_line(state: &RenderState, text_tertiary: ratatui::style::Color) -> (Line<'_>, usize) {
    use ratatui::text::Span;
    let mut parts = vec![];
    if let Some(ref model) = state.current_model {
        parts.push(Span::styled(model.clone(), Style::default().fg(text_tertiary)));
        parts.push(Span::styled(" · ", Style::default().fg(text_tertiary)));
    }
    if state.session_token_usage.total_tokens > 0 {
        parts.push(Span::styled(format!("{} tokens", state.session_token_usage.total_tokens), Style::default().fg(text_tertiary)));
        if state.session_token_usage.estimated_cost > 0.0 {
            parts.push(Span::styled(format!(" · ${:.4}", state.session_token_usage.estimated_cost), Style::default().fg(text_tertiary)));
        }
    }
    let line = Line::from(parts.clone());
    let width = line.spans.iter().map(|s| s.width()).sum();
    (line, width)
}

fn render_bg_jobs(area: Rect, buf: &mut Buffer, text_secondary: ratatui::style::Color, jobs: &[crate::components::status_bar::BackgroundJob], braille_frame: usize) {
    use ratatui::text::Line;
    let running: Vec<_> = jobs.iter().filter(|j| j.status == crate::components::status_bar::JobStatus::Running).collect();
    if running.is_empty() { return; }
    let count = running.len();
    let latest = running.last().expect("checked non-empty above");
    let braille = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
    let spinner = braille[braille_frame % 10];
    let text = if count == 1 {
        format!("⬡ {} │ {} {}", latest.name, spinner, latest.name)
    } else {
        format!("⬡ {} jobs │ {} {}", count, spinner, latest.name)
    };
    let width = text.len() as u16;
    let x = area.x + area.width - width - 1;
    buf.set_line(x, area.y, &Line::raw(text).style(Style::default().fg(text_secondary)), width);
}

pub fn render_status_bar(state: &RenderState, area: Rect, buf: &mut Buffer, theme: &ThemeWrapper, braille_frame: usize) {
    use ratatui::style::Modifier;
    use ratatui::text::{Line, Span};

    let text_tertiary: ratatui::style::Color = theme.color("text.dim").into();
    let text_secondary: ratatui::style::Color = theme.color("text.secondary").into();
    let items = get_status_items(&state.mode);

    let (center_line, center_width) = build_center_line(state, text_tertiary);
    let left_width: usize = items.iter().map(|(k, d)| k.len() + 1 + d.len()).sum::<usize>() + (items.len().saturating_sub(1) * 3);
    let remaining = (area.width.saturating_sub(2) as usize).saturating_sub(left_width + center_width);

    let mut x = area.x as usize + 1;
    for (i, (key, desc)) in items.iter().enumerate() {
        if i > 0 {
            buf.set_line(x as u16, area.y, &Line::from(Span::styled(" | ", Style::default().fg(text_tertiary))), 3);
            x += 3;
        }
        let parts = vec![
            Span::styled(*key, Style::default().fg(text_tertiary)),
            Span::styled(format!(" {}", desc), Style::default().fg(text_tertiary).add_modifier(Modifier::DIM)),
        ];
        let width = (key.len() + 1 + desc.len()) as u16;
        buf.set_line(x as u16, area.y, &Line::from(parts), width);
        x += width as usize;
    }

    if !center_line.spans.is_empty() && remaining >= center_width {
        buf.set_line((area.x as usize + 1 + (remaining / 2)) as u16, area.y, &center_line, center_width as u16);
    }

    render_bg_jobs(area, buf, text_secondary, &state.background_jobs, braille_frame);
}

// ─── Agent List (Sidebar) ────────────────────────────────────────────────────

/// Braille spinner frames
const BRAILLE_FRAMES: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

/// Format cost as $X.XX (or $X.XX for larger)
fn format_cost(cost: f64) -> String {
    if cost >= 100.0 {
        format!("${:.0}", cost)
    } else if cost >= 10.0 {
        format!("${:.2}", cost)
    } else if cost >= 1.0 {
        format!("${:.2}", cost)
    } else {
        format!("${:.4}", cost)
    }
}

fn collect_sidebar_data(state: &RenderState) -> (Vec<(usize, String, crate::components::message_list::PlanStatus)>, Vec<&crate::components::status_bar::BackgroundJob>, usize, u64, f64) {
    let plan_steps: Vec<_> = state.messages.iter()
        .filter_map(|msg| {
            if let crate::components::MessageItem::PlanStep { step, text, status } = msg {
                Some((*step, text.clone(), status.clone()))
            } else {
                None
            }
        })
        .collect();

    let running_jobs: Vec<_> = state.background_jobs.iter()
        .filter(|j| j.status == crate::components::status_bar::JobStatus::Running)
        .collect();

    let active_count = running_jobs.len() + if state.agent_running { 1 } else { 0 };
    let tokens = state.session_token_usage.total_tokens as u64;
    let cost = state.session_token_usage.estimated_cost;

    (plan_steps, running_jobs, active_count, tokens, cost)
}

struct SidebarColors {
    bg_panel: ratatui::style::Color,
    text_secondary: ratatui::style::Color,
    text_dim: ratatui::style::Color,
    accent_primary: ratatui::style::Color,
    border_unfocused: ratatui::style::Color,
}

fn get_sidebar_colors(theme: &ThemeWrapper) -> SidebarColors {
    SidebarColors {
        bg_panel: theme.color("bg.panel").into(),
        text_secondary: theme.color("text.secondary").into(),
        text_dim: theme.color("text.dim").into(),
        accent_primary: theme.color("accent.primary").into(),
        border_unfocused: theme.color("border.unfocused").into(),
    }
}

struct PanelLayout {
    plan_rect: Rect,
    agents_rect: Rect,
}

fn calculate_panel_layout(area: Rect) -> PanelLayout {
    let total_height = area.height;
    let min_each = 4u16;
    let gap = 1u16;

    // Plan gets ~40%, Agents gets ~60%
    let plan_height = ((total_height as f32 * 0.4).round() as u16).max(min_each);
    let agents_height = (total_height.saturating_sub(plan_height + gap)).max(min_each);

    let plan_rect = Rect::new(area.x, area.y, area.width, plan_height);
    let agents_rect = Rect::new(area.x, area.y + plan_height + gap, area.width, agents_height);

    PanelLayout { plan_rect, agents_rect }
}

fn render_agent_list_full(buf: &mut Buffer, colors: &SidebarColors, state: &RenderState, layout: &PanelLayout, plan_steps: &[(usize, String, crate::components::message_list::PlanStatus)], running_jobs: &[&crate::components::status_bar::BackgroundJob], active_count: usize, _tokens: u64, cost: f64) {
    let bg_panel = colors.bg_panel;

    Panel::new()
        .title("Plan")
        .border_gradient(colors.border_unfocused, colors.accent_primary)
        .title_color(colors.accent_primary)
        .title_right()
        .bg(bg_panel)
        .render(layout.plan_rect, buf, |inner, buf| {
            render_plan_content(inner, buf, bg_panel, colors.text_secondary, colors.text_dim, colors.accent_primary, plan_steps, state.animation.braille_frame);
        });

    Panel::new()
        .title("Agents")
        .border_gradient(colors.border_unfocused, colors.accent_primary)
        .title_color(colors.accent_primary)
        .title_right()
        .bg(bg_panel)
        .render(layout.agents_rect, buf, |inner, buf| {
            render_agents_content(inner, buf, bg_panel, colors.text_secondary, colors.text_dim, state.agent_running, running_jobs, active_count, cost);
        });
}

pub fn render_agent_list(area: Rect, buf: &mut Buffer, theme: &ThemeWrapper, state: &RenderState) {
    if area.width < 4 || area.height < 3 { return; }

    let colors = get_sidebar_colors(theme);
    let min_height = 9; // minimum for 2 panels with gap

    if area.height < min_height {
        return;
    }

    let (plan_steps, running_jobs, active_count, _tokens, cost) = collect_sidebar_data(state);
    let layout = calculate_panel_layout(area);
    render_agent_list_full(buf, &colors, state, &layout, &plan_steps, &running_jobs, active_count, 0, cost);
}

fn render_plan_content(
    inner: Rect, buf: &mut Buffer,
    bg_panel: ratatui::style::Color,
    text_secondary: ratatui::style::Color,
    text_dim: ratatui::style::Color,
    accent_primary: ratatui::style::Color,
    plan_steps: &[(usize, String, crate::components::message_list::PlanStatus)],
    braille_frame: usize,
) {
    let inner_width = inner.width;
    let content_x = inner.x;
    let mut y = inner.y;
    let max_y = inner.y + inner.height - 1;

    if plan_steps.is_empty() {
        let no_plan_line = Line::from(vec![
            Span::styled(" ", Style::default().bg(bg_panel)),
            Span::styled("No plan steps", Style::default().fg(text_dim).bg(bg_panel)),
        ]);
        buf.set_line(content_x, y, &no_plan_line, inner_width);
        return;
    }

    let spinner = BRAILLE_FRAMES[braille_frame % 10];
    for (step, text, status) in plan_steps {
        if y >= max_y - 1 {
            break;
        }

        let (glyph, color) = match status {
            crate::components::message_list::PlanStatus::Pending => ('○', text_dim),
            crate::components::message_list::PlanStatus::Active => ('●', accent_primary),
            crate::components::message_list::PlanStatus::Complete => ('✓', text_secondary),
        };

        let suffix = if matches!(status, crate::components::message_list::PlanStatus::Active) {
            format!(" {}", spinner)
        } else {
            String::new()
        };

        let max_text_len = (inner_width as usize).saturating_sub(8);
        let text_truncated = if text.len() > max_text_len {
            format!("{}…", &text[..max_text_len.saturating_sub(1)])
        } else {
            text.clone()
        };

        let plan_line = Line::from(vec![
            Span::styled(" ", Style::default().bg(bg_panel)),
            Span::styled(format!("{}", glyph), Style::default().fg(color).bg(bg_panel)),
            Span::styled(
                format!(" {}. {}", step, text_truncated),
                Style::default().fg(color).bg(bg_panel),
            ),
            Span::styled(&suffix, Style::default().fg(text_dim).bg(bg_panel)),
        ]);
        buf.set_line(content_x, y, &plan_line, inner_width);
        y += 1;
    }
}

fn render_agents_content(
    inner: Rect, buf: &mut Buffer,
    bg_panel: ratatui::style::Color,
    text_secondary: ratatui::style::Color,
    text_dim: ratatui::style::Color,
    agent_running: bool,
    running_jobs: &[&crate::components::status_bar::BackgroundJob],
    active_count: usize,
    cost: f64,
) {
    let inner_width = inner.width;
    let content_x = inner.x;
    let mut y = inner.y;
    let max_y = inner.y + inner.height - 1;

    let agent_status = if agent_running { "● running" } else { "○ idle" };
    let agent_line = Line::from(vec![
        Span::styled(" ", Style::default().bg(bg_panel)),
        Span::styled(agent_status, Style::default().fg(text_dim).bg(bg_panel)),
    ]);
    buf.set_line(content_x, y, &agent_line, inner_width);
    y += 1;

    for job in running_jobs {
        if y >= max_y - 1 {
            break;
        }
        let job_line = Line::from(vec![
            Span::styled(" ", Style::default().bg(bg_panel)),
            Span::styled("⬡ ", Style::default().fg(text_dim).bg(bg_panel)),
            Span::styled(&job.name, Style::default().fg(text_secondary).bg(bg_panel)),
        ]);
        buf.set_line(content_x, y, &job_line, inner_width);
        y += 1;
    }

    if active_count > 0 {
        let footer_text = format!("{} active · {}", active_count, format_cost(cost));
        let footer_y = max_y - 1;
        let footer_line = Line::from(vec![
            Span::styled(" ", Style::default().bg(bg_panel)),
            Span::styled(&footer_text, Style::default().fg(text_dim).bg(bg_panel)),
        ]);
        buf.set_line(content_x, footer_y, &footer_line, inner_width);
    }
}

