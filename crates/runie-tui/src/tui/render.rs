use ratatui::{
    layout::Rect,
    buffer::Buffer,
    style::Style,
    text::{Line, Span},
};
use crate::theme::ThemeColors;
use crate::messages::MessageRegistry;
use crate::tui::state::TuiMode;
use crate::components::panel::Panel;
use crate::tui::view_models::{StatusBarViewModel, AgentListViewModel};

// ─── Status Bar ───────────────────────────────────────────────────────────────

const NAV_KEYS: &[(&str, &str)] = &[("Esc", "close"), ("j/k", "navigate"), ("Enter", "select")];
const ARROW_KEYS: &[(&str, &str)] = &[("Esc", "close"), ("↑↓", "navigate"), ("Enter", "jump")];

/// Format elapsed time as "1m 23s" or "1h 02m 30s"
fn format_elapsed(start: std::time::Instant) -> String {
    let elapsed = start.elapsed().as_secs();
    if elapsed < 60 {
        format!("{}s", elapsed)
    } else if elapsed < 3600 {
        format!("{}m {:02}s", elapsed / 60, elapsed % 60)
    } else {
        format!("{}h {:02}m {:02}s", elapsed / 3600, (elapsed % 3600) / 60, elapsed % 60)
    }
}

pub(crate) fn get_status_items(mode: &TuiMode) -> Vec<(&'static str, &'static str)> {
    match *mode {
        TuiMode::Chat => vec![("Enter", "send"), ("^b", "sidebar"), ("^k", "cmd"), ("^q", "quit")],
        TuiMode::Overlay | TuiMode::Select => NAV_KEYS.to_vec(),
        // P0-4 FIX: Updated to reflect actual key mappings + timeout indicator
        TuiMode::Permission => vec![("y/Enter", "confirm"), ("Esc/n", "cancel"), ("a", "always")],
        TuiMode::CommandPalette => vec![("Esc", "close"), ("Enter", "select"), ("↑↓", "navigate")],
        // P0-4 FIX: Added Ctrl+Q for consistent quit/close
        TuiMode::DiffViewer => vec![("q/Esc", "close"), ("j/k", "scroll")],
        TuiMode::SessionTree => ARROW_KEYS.to_vec(),
        TuiMode::Onboarding => vec![("Esc", "back"), ("↑↓", "navigate"), ("Enter", "next")],
        TuiMode::HomeScreen => vec![("↑↓", "navigate"), ("Enter", "select"), ("q", "quit")],
        TuiMode::Plan => vec![("Enter", "approve"), ("Esc", "close"), ("↑↓", "scroll")],
    }
}

fn build_center_line(vm: &StatusBarViewModel, text_tertiary: ratatui::style::Color, warning: ratatui::style::Color) -> (Line<'_>, usize) {
    use ratatui::text::Span;
    let mut parts = vec![];

    // P0-2 FIX: Show warning when no model is configured
    if vm.current_model.is_none() {
        parts.push(Span::styled("⚠ No model configured", Style::default().fg(warning)));
    } else if let Some(model) = vm.current_model.as_deref() {
        parts.push(Span::styled(model, Style::default().fg(text_tertiary)));
        parts.push(Span::styled(" · ", Style::default().fg(text_tertiary)));
    }

    if vm.session_token_usage.total_tokens > 0 {
        parts.push(Span::styled(format!("{} tokens", vm.session_token_usage.total_tokens), Style::default().fg(text_tertiary)));
        if vm.session_token_usage.estimated_cost > 0.0 {
            parts.push(Span::styled(format!(" · ${:.4}", vm.session_token_usage.estimated_cost), Style::default().fg(text_tertiary)));
        }
    }
    let line = Line::from(parts);
    let width = line.spans.iter().map(|s| s.width()).sum();
    (line, width)
}

fn render_left_keys(items: &[(&str, &str)], area: Rect, buf: &mut Buffer, text_tertiary: ratatui::style::Color) -> usize {
    use ratatui::text::{Line, Span};
    use ratatui::style::Modifier;
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
    x
}

pub fn render_status_bar(vm: &StatusBarViewModel, area: Rect, buf: &mut Buffer, colors: &ThemeColors) {
    let text_tertiary = colors.text_dim;
    let accent = colors.accent_primary;
    let warning = colors.error;
    let items = get_status_items(&vm.mode);

    let (center_line, center_width) = build_center_line(vm, text_tertiary, warning);
    let left_width: usize = items.iter().map(|(k, d)| k.len() + 1 + d.len()).sum::<usize>() + (items.len().saturating_sub(1) * 3);

    let mut x = render_left_keys(&items, area, buf, text_tertiary);
    let status_width = render_live_status(vm, &mut x, buf, accent, text_tertiary, area.y);
    render_status_center(area, buf, x, center_line, center_width, left_width + status_width);
}

/// Renders the live status indicator and returns its width
fn render_live_status(vm: &StatusBarViewModel, x: &mut usize, buf: &mut Buffer, accent: ratatui::style::Color, text_tertiary: ratatui::style::Color, y: u16) -> usize {
    use ratatui::text::{Line, Span};

    let (Some(header), Some(start_time)) = (&vm.status_header, vm.status_start_time) else {
        return 0;
    };

    let elapsed = format_elapsed(start_time);
    let status_text = if vm.status_details.is_some() {
        format!("● {} ({})", header, elapsed)
    } else {
        format!("● {}", header)
    };

    let details = vm.status_details.as_ref();
    let max_details_len = 40;
    let details_part = details.map(|d| {
        let truncated = if d.len() > max_details_len {
            format!("{}…", &d[..max_details_len.saturating_sub(1)])
        } else {
            d.clone()
        };
        format!("  └ {}", truncated)
    }).unwrap_or_default();

    let full_status = format!("{}{}", status_text, details_part);
    let line = Line::from(vec![
        Span::styled(&status_text, Style::default().fg(accent)),
        Span::styled(&details_part, Style::default().fg(text_tertiary)),
    ]);
    buf.set_line(*x as u16, y, &line, full_status.len() as u16);
    *x += full_status.len();
    *x += 3; // spacing before center
    full_status.len() + 3
}

/// Renders center text only if it fits without overlapping left or right sides
fn render_status_center(area: Rect, buf: &mut Buffer, left_end: usize, center_line: Line<'_>, center_width: usize, left_width: usize) {
    let right_width: u16 = 0;
    let min_padding = 2;
    let max_center_x = area.width as usize - right_width as usize - min_padding;
    let min_center_x = left_end + min_padding;

    if center_line.spans.is_empty() || center_width >= (area.width as usize).saturating_sub(left_width + right_width as usize + min_padding * 2) {
        return;
    }

    let ideal_center_x = (area.width as usize - center_width) / 2;
    let center_x = if ideal_center_x >= min_center_x && ideal_center_x + center_width <= max_center_x {
        ideal_center_x
    } else if ideal_center_x < min_center_x {
        return; // Not enough space on left, skip center
    } else {
        min_center_x // Would exceed right side, use min_center_x
    };

    if center_x < area.width as usize && center_x + center_width <= max_center_x {
        buf.set_line(center_x as u16, area.y, &center_line, center_width as u16);
    }
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

struct SidebarColors {
    text_secondary: ratatui::style::Color,
    text_dim: ratatui::style::Color,
    accent_primary: ratatui::style::Color,
    border_unfocused: ratatui::style::Color,
}

fn get_sidebar_colors(colors: &ThemeColors) -> SidebarColors {
    SidebarColors {
        text_secondary: colors.text_secondary,
        text_dim: colors.text_dim,
        accent_primary: colors.accent_primary,
        border_unfocused: colors.border_unfocused,
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

fn render_agent_list_full(buf: &mut Buffer, colors: &SidebarColors, vm: &AgentListViewModel, layout: &PanelLayout) {
    Panel::new()
        .title("Plan")
        .border_gradient(colors.border_unfocused, colors.accent_primary)
        .title_color(colors.accent_primary)
        .title_right()
        .render(layout.plan_rect, buf, |inner, buf| {
            render_plan_content(vm, inner, buf, colors);
        });

    Panel::new()
        .title("Agents")
        .border_gradient(colors.border_unfocused, colors.accent_primary)
        .title_color(colors.accent_primary)
        .title_right()
        .render(layout.agents_rect, buf, |inner, buf| {
            render_agents_content(vm, inner, buf, colors);
        });
}

pub fn render_agent_list(vm: &AgentListViewModel, area: Rect, buf: &mut Buffer, colors: &ThemeColors) {
    if area.width < 4 || area.height < 3 { return; }

    let colors = get_sidebar_colors(colors);
    let min_height = 9; // minimum for 2 panels with gap

    if area.height < min_height {
        return;
    }

    let layout = calculate_panel_layout(area);
    render_agent_list_full(buf, &colors, vm, &layout);
}

fn render_plan_content(
    vm: &AgentListViewModel,
    inner: Rect, buf: &mut Buffer,
    colors: &SidebarColors,
) {
    let inner_width = inner.width;
    let content_x = inner.x;
    let mut y = inner.y;
    let max_y = inner.y + inner.height - 1;

    if vm.plan_steps.is_empty() {
        render_no_plan(content_x, y, inner_width, buf, colors);
        return;
    }

    let spinner = BRAILLE_FRAMES[vm.braille_frame % 10];
    for (step, text, status) in &vm.plan_steps {
        if y >= max_y - 1 {
            break;
        }
        render_plan_step(content_x, y, inner_width, step, text, status, spinner, buf, colors);
        y += 1;
    }
}

fn render_no_plan(content_x: u16, y: u16, inner_width: u16, buf: &mut Buffer, colors: &SidebarColors) {
    let no_plan_line = Line::from(vec![
        Span::styled(" ", Style::default()),
        Span::styled("No plan steps", Style::default().fg(colors.text_dim)),
    ]);
    buf.set_line(content_x, y, &no_plan_line, inner_width);
}

fn render_plan_step(content_x: u16, y: u16, inner_width: u16, step: &usize, text: &str, status: &crate::components::message_list::PlanStatus, spinner: char, buf: &mut Buffer, colors: &SidebarColors) {
    let (glyph, color) = match status {
        crate::components::message_list::PlanStatus::Pending => ('○', colors.text_dim),
        crate::components::message_list::PlanStatus::Active => ('●', colors.accent_primary),
        crate::components::message_list::PlanStatus::Complete => ('✓', colors.text_secondary),
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
        text.to_string()
    };

    let plan_line = Line::from(vec![
        Span::styled(" ", Style::default()),
        Span::styled(format!("{}", glyph), Style::default().fg(color)),
        Span::styled(format!(" {}. {}", step, text_truncated), Style::default().fg(color)),
        Span::styled(&suffix, Style::default().fg(colors.text_dim)),
    ]);
    buf.set_line(content_x, y, &plan_line, inner_width);
}

fn render_agents_header(vm: &AgentListViewModel, inner: Rect, buf: &mut Buffer, colors: &SidebarColors) {
    let inner_width = inner.width;
    let content_x = inner.x;
    let status_text = if vm.agent_running {
        format!("● {}", MessageRegistry::status_running())
    } else {
        format!("○ {}", MessageRegistry::status_idle())
    };
    let agent_line = Line::from(vec![
        Span::styled(" ", Style::default()),
        Span::styled(&status_text, Style::default().fg(colors.text_dim)),
    ]);
    buf.set_line(content_x, inner.y, &agent_line, inner_width);
}

fn render_agents_jobs(vm: &AgentListViewModel, inner: Rect, buf: &mut Buffer, colors: &SidebarColors) -> usize {
    let inner_width = inner.width;
    let content_x = inner.x;
    let mut y = inner.y + 1;
    let max_y = inner.y + inner.height - 1;

    for job in &vm.running_jobs {
        if y >= max_y - 1 {
            break;
        }
        let job_line = Line::from(vec![
            Span::styled(" ", Style::default()),
            Span::styled("⬡ ", Style::default().fg(colors.text_dim)),
            Span::styled(&job.name, Style::default().fg(colors.text_secondary)),
        ]);
        buf.set_line(content_x, y, &job_line, inner_width);
        y += 1;
    }
    y as usize
}

fn render_agents_footer(vm: &AgentListViewModel, inner: Rect, buf: &mut Buffer, colors: &SidebarColors) {
    if vm.active_count > 0 {
        let inner_width = inner.width;
        let content_x = inner.x;
        let footer_text = format!("{} active · {}", vm.active_count, format_cost(vm.cost));
        let footer_y = inner.y + inner.height - 2;
        let footer_line = Line::from(vec![
            Span::styled(" ", Style::default()),
            Span::styled(&footer_text, Style::default().fg(colors.text_dim)),
        ]);
        buf.set_line(content_x, footer_y, &footer_line, inner_width);
    }
}

fn render_agents_content(
    vm: &AgentListViewModel,
    inner: Rect, buf: &mut Buffer,
    colors: &SidebarColors,
) {
    render_agents_header(vm, inner, buf, colors);
    render_agents_jobs(vm, inner, buf, colors);
    render_agents_footer(vm, inner, buf, colors);
}
