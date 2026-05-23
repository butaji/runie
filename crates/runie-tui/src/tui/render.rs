use ratatui::{
    layout::Rect,
    buffer::Buffer,
    style::Style,
    text::{Line, Span},
};
use crate::components::gradient_border::render_gradient_border;
use crate::theme::ThemeWrapper;
use crate::tui::state::{TuiMode, TopBarState, RenderState};

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
    let latest = running.last().unwrap();
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

/// Draw a section header: title + underline
fn draw_section_header(
    area: Rect, buf: &mut Buffer,
    title: &str,
    y: u16,
    accent_primary: ratatui::style::Color,
    border_unfocused: ratatui::style::Color,
) {
    let inner_width = area.width.saturating_sub(2);

    // Title in accent color
    let title_line = Line::from(Span::styled(
        format!(" {} ", title),
        Style::default().fg(accent_primary).add_modifier(ratatui::style::Modifier::BOLD),
    ));
    buf.set_line(area.x + 1, y, &title_line, inner_width);

    // Underline with ─
    let underline_y = y + 1;
    for x in area.x..(area.x + area.width) {
        if let Some(cell) = buf.cell_mut((x, underline_y)) {
            cell.set_char('─');
            cell.set_style(Style::default().fg(border_unfocused));
        }
    }
}

/// Draw a horizontal separator line
fn draw_separator(
    area: Rect, buf: &mut Buffer,
    y: u16,
    border_unfocused: ratatui::style::Color,
) {
    for x in (area.x + 1)..(area.x + area.width - 1) {
        if let Some(cell) = buf.cell_mut((x, y)) {
            cell.set_char('─');
            cell.set_style(Style::default().fg(border_unfocused));
        }
    }
}

/// Format token count with commas
fn format_tokens(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.insert(0, ',');
        }
        result.insert(0, ch);
    }
    result
}

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

fn render_sidebar_minimal(
    area: Rect, buf: &mut Buffer,
    bg_panel: ratatui::style::Color,
    accent_primary: ratatui::style::Color,
    text_dim: ratatui::style::Color,
    model: &Option<String>,
    agent_running: bool,
) {
    let inner_width = area.width.saturating_sub(2);
    let content_x = area.x + 1;
    let content_y = area.y + 1;

    let model_text = model.clone().unwrap_or_else(|| "No model".to_string());
    let status_text = if agent_running { "● running" } else { "○ idle" };

    let model_line = Line::from(vec![
        Span::styled("  ", Style::default().bg(bg_panel)),
        Span::styled(&model_text, Style::default().fg(accent_primary).bg(bg_panel)),
    ]);
    buf.set_line(content_x, content_y, &model_line, inner_width);

    let status_line = Line::from(vec![
        Span::styled("  ", Style::default().bg(bg_panel)),
        Span::styled(status_text, Style::default().fg(text_dim).bg(bg_panel)),
    ]);
    buf.set_line(content_x, content_y + 1, &status_line, inner_width);
}

fn render_sidebar_section_header(
    area: Rect, buf: &mut Buffer,
    title: &str,
    y: u16,
    accent_primary: ratatui::style::Color,
    border_color: ratatui::style::Color,
) {
    draw_section_header(area, buf, title, y, accent_primary, border_color);
}

fn render_sidebar_model_section(
    area: Rect, buf: &mut Buffer,
    content_x: u16,
    y: &mut u16,
    bg_panel: ratatui::style::Color,
    text_secondary: ratatui::style::Color,
    model: &Option<String>,
    accent_primary: ratatui::style::Color,
    border_color: ratatui::style::Color,
) {
    render_sidebar_section_header(area, buf, "Model", *y, accent_primary, border_color);
    *y += 2;

    let model_text = model.clone().unwrap_or_else(|| "No model".to_string());
    let model_line = Line::from(vec![
        Span::styled("  ", Style::default().bg(bg_panel)),
        Span::styled(&model_text, Style::default().fg(text_secondary).bg(bg_panel)),
    ]);
    buf.set_line(content_x, *y, &model_line, area.width.saturating_sub(2));
    *y += 1;
}

fn render_sidebar_context_section(
    area: Rect, buf: &mut Buffer,
    content_x: u16,
    y: &mut u16,
    bg_panel: ratatui::style::Color,
    text_secondary: ratatui::style::Color,
    tokens: u64,
    cost: f64,
    accent_primary: ratatui::style::Color,
    border_color: ratatui::style::Color,
) {
    render_sidebar_section_header(area, buf, "Context", *y, accent_primary, border_color);
    *y += 2;

    let context_text = if tokens > 0 {
        format!("{} tokens · {}", format_tokens(tokens), format_cost(cost))
    } else {
        "0 tokens".to_string()
    };
    let context_line = Line::from(vec![
        Span::styled("  ", Style::default().bg(bg_panel)),
        Span::styled(&context_text, Style::default().fg(text_secondary).bg(bg_panel)),
    ]);
    buf.set_line(content_x, *y, &context_line, area.width.saturating_sub(2));
    *y += 1;
}

fn render_sidebar_plan_section(
    area: Rect, buf: &mut Buffer,
    content_x: u16,
    y: &mut u16,
    max_y: u16,
    bg_panel: ratatui::style::Color,
    text_secondary: ratatui::style::Color,
    text_dim: ratatui::style::Color,
    accent_primary: ratatui::style::Color,
    border_color: ratatui::style::Color,
    plan_steps: &[(usize, String, crate::components::message_list::PlanStatus)],
    braille_frame: usize,
) {
    draw_separator(area, buf, *y, border_color);
    *y += 1;
    render_sidebar_section_header(area, buf, "Plan", *y, accent_primary, border_color);
    *y += 2;

    if plan_steps.is_empty() {
        let no_plan_line = Line::from(vec![
            Span::styled("  ", Style::default().bg(bg_panel)),
            Span::styled("No plan steps", Style::default().fg(text_dim).bg(bg_panel)),
        ]);
        buf.set_line(content_x, *y, &no_plan_line, area.width.saturating_sub(2));
        *y += 1;
        return;
    }

    let spinner = BRAILLE_FRAMES[braille_frame % 10];
    for (step, text, status) in plan_steps {
        if *y >= max_y - 1 {
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

        let inner_width = area.width.saturating_sub(2);
        let max_text_len = (inner_width as usize).saturating_sub(8);
        let text_truncated = if text.len() > max_text_len {
            format!("{}…", &text[..max_text_len.saturating_sub(1)])
        } else {
            text.clone()
        };

        let plan_line = Line::from(vec![
            Span::styled("  ", Style::default().bg(bg_panel)),
            Span::styled(format!("{}", glyph), Style::default().fg(color).bg(bg_panel)),
            Span::styled(
                format!(" {}. {}", step, text_truncated),
                Style::default().fg(color).bg(bg_panel),
            ),
            Span::styled(&suffix, Style::default().fg(text_dim).bg(bg_panel)),
        ]);
        buf.set_line(content_x, *y, &plan_line, inner_width);
        *y += 1;
    }
}

fn render_sidebar_agents_section(
    area: Rect, buf: &mut Buffer,
    content_x: u16,
    y: &mut u16,
    max_y: u16,
    bg_panel: ratatui::style::Color,
    text_secondary: ratatui::style::Color,
    text_dim: ratatui::style::Color,
    accent_primary: ratatui::style::Color,
    border_color: ratatui::style::Color,
    agent_running: bool,
    running_jobs: &[&crate::components::status_bar::BackgroundJob],
) {
    draw_separator(area, buf, *y, border_color);
    *y += 1;
    render_sidebar_section_header(area, buf, "Agents", *y, accent_primary, border_color);
    *y += 2;

    let agent_status = if agent_running { "● running" } else { "○ idle" };
    let agent_line = Line::from(vec![
        Span::styled("  ", Style::default().bg(bg_panel)),
        Span::styled(agent_status, Style::default().fg(text_dim).bg(bg_panel)),
    ]);
    buf.set_line(content_x, *y, &agent_line, area.width.saturating_sub(2));
    *y += 1;

    for job in running_jobs {
        if *y >= max_y - 1 {
            break;
        }
        let job_line = Line::from(vec![
            Span::styled("  ", Style::default().bg(bg_panel)),
            Span::styled("⬡ ", Style::default().fg(text_dim).bg(bg_panel)),
            Span::styled(&job.name, Style::default().fg(text_secondary).bg(bg_panel)),
        ]);
        buf.set_line(content_x, *y, &job_line, area.width.saturating_sub(2));
        *y += 1;
    }
}

fn render_sidebar_footer(
    area: Rect, buf: &mut Buffer,
    content_x: u16,
    max_y: u16,
    bg_panel: ratatui::style::Color,
    text_dim: ratatui::style::Color,
    cost: f64,
    active_count: usize,
) {
    if active_count > 0 {
        let footer_text = format!("{} active · {}", active_count, format_cost(cost));
        let footer_line = Line::from(vec![
            Span::styled("  ", Style::default().bg(bg_panel)),
            Span::styled(&footer_text, Style::default().fg(text_dim).bg(bg_panel)),
        ]);
        buf.set_line(content_x, max_y - 1, &footer_line, area.width.saturating_sub(2));
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
    border_color: ratatui::style::Color,
    text_secondary: ratatui::style::Color,
    text_dim: ratatui::style::Color,
    accent_primary: ratatui::style::Color,
}

fn get_sidebar_colors(theme: &ThemeWrapper) -> SidebarColors {
    SidebarColors {
        bg_panel: theme.color("bg.panel").into(),
        border_color: theme.color("border.unfocused").into(),
        text_secondary: theme.color("text.secondary").into(),
        text_dim: theme.color("text.dim").into(),
        accent_primary: theme.color("accent.primary").into(),
    }
}

fn render_sidebar_full(
    area: Rect, buf: &mut Buffer,
    colors: &SidebarColors,
    content_x: u16,
    content_y: u16,
    max_y: u16,
    state: &RenderState,
    plan_steps: &[(usize, String, crate::components::message_list::PlanStatus)],
    running_jobs: &[&crate::components::status_bar::BackgroundJob],
    active_count: usize,
    tokens: u64,
    cost: f64,
) {
    let mut y = content_y;

    render_sidebar_model_section(
        area, buf, content_x, &mut y, colors.bg_panel, colors.text_secondary,
        &state.current_model, colors.accent_primary, colors.border_color,
    );

    render_sidebar_context_section(
        area, buf, content_x, &mut y, colors.bg_panel, colors.text_secondary,
        tokens, cost, colors.accent_primary, colors.border_color,
    );

    render_sidebar_plan_section(
        area, buf, content_x, &mut y, max_y, colors.bg_panel,
        colors.text_secondary, colors.text_dim, colors.accent_primary, colors.border_color,
        plan_steps, state.animation.braille_frame,
    );

    render_sidebar_agents_section(
        area, buf, content_x, &mut y, max_y, colors.bg_panel,
        colors.text_secondary, colors.text_dim, colors.accent_primary, colors.border_color,
        state.agent_running, running_jobs,
    );

    render_sidebar_footer(
        area, buf, content_x, max_y, colors.bg_panel, colors.text_dim, cost, active_count,
    );
}

pub fn render_agent_list(area: Rect, buf: &mut Buffer, theme: &ThemeWrapper, state: &RenderState) {
    if area.width < 4 || area.height < 3 {
        return;
    }

    let colors = get_sidebar_colors(theme);
    let is_minimal = area.height < 10;

    // Clear interior with bg_panel color
    let bg_panel: ratatui::style::Color = colors.bg_panel;
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_style(Style::default().bg(bg_panel));
            }
        }
    }

    // Draw gradient border
    render_gradient_border(area, buf);

    let content_x = area.x + 1;
    let content_y = area.y + 1;
    let max_y = area.y + area.height - 1;

    if is_minimal {
        render_sidebar_minimal(
            area, buf, colors.bg_panel, colors.accent_primary, colors.text_dim,
            &state.current_model, state.agent_running,
        );
        return;
    }

    let (plan_steps, running_jobs, active_count, tokens, cost) = collect_sidebar_data(state);
    render_sidebar_full(
        area, buf, &colors, content_x, content_y, max_y, state,
        &plan_steps, &running_jobs, active_count, tokens, cost,
    );
}
