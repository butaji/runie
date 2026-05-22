use ratatui::{
    layout::{Constraint, Layout, Rect},
    buffer::Buffer,
    style::Style,
    text::{Line, Span},
};
use crate::theme::ThemeWrapper;
use crate::tui::state::{TuiMode, TopBarState, RenderState};
use crate::components::{
    AgentStatus, AgentList, AgentItem, ContextPanel,
    GitChange, GitStatus,
};

// ─── Top Bar ─────────────────────────────────────────────────────────────────

fn build_left_parts(top_bar: &TopBarState, text_primary: ratatui::style::Color, text_muted: ratatui::style::Color) -> Vec<Span> {
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

fn build_right_parts(top_bar: &TopBarState, text_secondary: ratatui::style::Color, text_muted: ratatui::style::Color) -> Vec<Span> {
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
    use ratatui::text::{Line, Span};

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

fn get_status_items(mode: &TuiMode) -> Vec<(&'static str, &'static str)> {
    match *mode {
        TuiMode::Chat => vec![
            ("Enter", "send"),
            ("^b", "sidebar"),
            ("^k", "cmd"),
            ("^q", "quit"),
        ],
        TuiMode::Overlay => vec![
            ("Esc", "close"),
            ("j/k", "navigate"),
            ("Enter", "select"),
        ],
        TuiMode::Select => vec![
            ("Esc", "close"),
            ("j/k", "navigate"),
            ("Enter", "select"),
        ],
        TuiMode::Permission => vec![
            ("y", "confirm"),
            ("n", "cancel"),
            ("a", "always"),
            ("s", "skip"),
        ],
        TuiMode::CommandPalette => vec![
            ("Esc", "close"),
            ("Enter", "select"),
            ("↑↓", "navigate"),
        ],
        TuiMode::DiffViewer => vec![
            ("q", "close"),
            ("j/k", "scroll"),
        ],
        TuiMode::SessionTree => vec![
            ("Esc", "close"),
            ("↑↓", "navigate"),
            ("Enter", "jump"),
        ],
    }
}

pub fn render_status_bar(state: &RenderState, area: Rect, buf: &mut Buffer, theme: &ThemeWrapper) {
    use ratatui::style::Modifier;
    use ratatui::text::{Line, Span};

    let text_tertiary: ratatui::style::Color = theme.color("text.dim").into();
    let items = get_status_items(&state.mode);

    // Build center parts
    let mut center_parts = vec![];
    if let Some(ref model) = state.current_model {
        center_parts.push(Span::styled(model.clone(), Style::default().fg(text_tertiary)));
        center_parts.push(Span::styled(" · ", Style::default().fg(text_tertiary)));
    }
    if state.session_token_usage.total_tokens > 0 {
        center_parts.push(Span::styled(format!("{} tokens", state.session_token_usage.total_tokens), Style::default().fg(text_tertiary)));
        if state.session_token_usage.estimated_cost > 0.0 {
            center_parts.push(Span::styled(format!(" · ${:.4}", state.session_token_usage.estimated_cost), Style::default().fg(text_tertiary)));
        }
    }
    let center_line = Line::from(center_parts.clone());
    let center_width: usize = center_line.spans.iter().map(|s| s.width()).sum();

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

    if !center_parts.is_empty() && remaining >= center_width {
        buf.set_line((area.x as usize + 1 + (remaining / 2)) as u16, area.y, &center_line, center_width as u16);
    }
}

// ─── Agent List ───────────────────────────────────────────────────────────────

fn render_agent_header(area: Rect, buf: &mut Buffer, border_color: ratatui::style::Color, accent_primary: ratatui::style::Color) {
    let header = " AGENTS ";
    let header_style = Style::default().fg(accent_primary).add_modifier(ratatui::style::Modifier::BOLD);
    let inner_width = area.width.saturating_sub(2);
    let header_len = header.len() as u16;
    let dashes = inner_width.saturating_sub(header_len);

    let mut x = area.x + 1;
    for ch in header.chars() {
        buf.get_mut(x, area.y).set_char(ch);
        buf.get_mut(x, area.y).set_style(header_style);
        x += 1;
    }
    for _ in 0..dashes {
        buf.get_mut(x, area.y).set_char('─');
        buf.get_mut(x, area.y).set_style(Style::default().fg(border_color));
        x += 1;
    }
}

fn get_agent_status_style(status: &AgentStatus, accent_primary: ratatui::style::Color, success: ratatui::style::Color, error: ratatui::style::Color, text_dim: ratatui::style::Color) -> (char, ratatui::style::Color) {
    match *status {
        AgentStatus::Running => ('●', accent_primary),
        AgentStatus::Completed => ('✓', success),
        AgentStatus::Failed => ('✗', error),
        AgentStatus::Waiting => ('○', text_dim),
    }
}

fn render_agent_item_row(
    area: Rect, buf: &mut Buffer,
    bg_panel: ratatui::style::Color,
    text_secondary: ratatui::style::Color,
    text_dim: ratatui::style::Color,
    accent_primary: ratatui::style::Color,
    success: ratatui::style::Color,
    error: ratatui::style::Color,
    agent: (&str, &str, &str, &str, &str, i32, AgentStatus),
    current_y: u16,
    max_y: u16,
) {
    let (status_char, status_fg) = get_agent_status_style(&agent.6, accent_primary, success, error, text_dim);

    let tag_color = match agent.2 {
        "user" | "assistant" => accent_primary,
        "system" => accent_primary,
        _ => text_dim,
    };

    let inner_width = area.width.saturating_sub(2);

    // Line 1: icon + tag
    let y1 = current_y;
    buf.get_mut(area.x + 2, y1).set_char(' ');
    buf.get_mut(area.x + 2, y1).set_style(Style::default().bg(bg_panel));
    buf.get_mut(area.x + 3, y1).set_char(status_char);
    buf.get_mut(area.x + 3, y1).set_style(Style::default().fg(status_fg).bg(bg_panel));
    buf.get_mut(area.x + 4, y1).set_char(' ');
    buf.get_mut(area.x + 4, y1).set_style(Style::default().bg(bg_panel));

    let tag_span = Span::styled(
        agent.1.to_string(),
        Style::default().fg(tag_color).add_modifier(ratatui::style::Modifier::BOLD).bg(bg_panel),
    );
    let tag_line = Line::from(vec![tag_span]);
    buf.set_line(area.x + 5, y1, &tag_line, inner_width.saturating_sub(5));

    // Line 2: description
    let y2 = current_y + 1;
    let desc_span = Span::styled(format!("  {}", agent.3), Style::default().fg(text_secondary).bg(bg_panel));
    let desc_line = Line::from(vec![desc_span]);
    buf.set_line(area.x + 2, y2, &desc_line, inner_width.saturating_sub(2));

    // Line 3: model + duration
    let y3 = current_y + 2;
    let duration_secs = agent.5;
    let duration_str = if duration_secs >= 60 {
        format!("{}m", duration_secs / 60)
    } else {
        format!("{}s", duration_secs)
    };
    let meta_span = Span::styled(format!("  {} · {}", agent.4, duration_str), Style::default().fg(text_dim).bg(bg_panel));
    let meta_line = Line::from(vec![meta_span]);
    buf.set_line(area.x + 2, y3, &meta_line, inner_width.saturating_sub(2));

    // Separator
    if current_y + 4 < max_y - 1 {
        let sep_y = current_y + 3;
        for sx in (area.x + 2)..(area.x + area.width - 2) {
            buf.get_mut(sx, sep_y).set_char('·');
            buf.get_mut(sx, sep_y).set_style(Style::default().fg(text_dim).bg(bg_panel));
        }
    }
}

fn fill_agent_list_interior(area: Rect, buf: &mut Buffer, bg_panel: ratatui::style::Color, border_color: ratatui::style::Color) {
    // Interior fill
    for y in (area.y + 1)..(area.y + area.height - 1) {
        for x in (area.x + 1)..(area.x + area.width - 1) {
            buf.get_mut(x, y).set_style(Style::default().bg(bg_panel));
        }
    }

    // Left and right borders
    for y in (area.y + 1)..(area.y + area.height - 1) {
        buf.get_mut(area.x, y).set_char('│');
        buf.get_mut(area.x, y).set_style(Style::default().fg(border_color));
        buf.get_mut(area.x + area.width - 1, y).set_char('│');
        buf.get_mut(area.x + area.width - 1, y).set_style(Style::default().fg(border_color));
    }
}

fn render_agent_list_bottom_border(area: Rect, buf: &mut Buffer, border_color: ratatui::style::Color) {
    let bottom_y = area.y + area.height - 1;
    buf.get_mut(area.x, bottom_y).set_char('╰');
    buf.get_mut(area.x, bottom_y).set_style(Style::default().fg(border_color));

    for x in (area.x + 1)..(area.x + area.width - 1) {
        buf.get_mut(x, bottom_y).set_char('─');
        buf.get_mut(x, bottom_y).set_style(Style::default().fg(border_color));
    }

    buf.get_mut(area.x + area.width - 1, bottom_y).set_char('╯');
    buf.get_mut(area.x + area.width - 1, bottom_y).set_style(Style::default().fg(border_color));
}

pub fn render_agent_list(area: Rect, buf: &mut Buffer, theme: &ThemeWrapper) {
    let bg_panel: ratatui::style::Color = theme.color("bg.panel").into();
    let border_color: ratatui::style::Color = theme.color("border.unfocused").into();
    let text_secondary: ratatui::style::Color = theme.color("text.secondary").into();
    let text_dim: ratatui::style::Color = theme.color("text.dim").into();
    let accent_primary: ratatui::style::Color = theme.color("accent.primary").into();
    let success: ratatui::style::Color = theme.color("success").into();
    let error: ratatui::style::Color = theme.color("error").into();

    if area.width < 4 || area.height < 3 {
        return;
    }

    // Top border
    buf.get_mut(area.x, area.y).set_char('╭');
    buf.get_mut(area.x, area.y).set_style(Style::default().fg(border_color));
    buf.get_mut(area.x + area.width - 1, area.y).set_char('╮');
    buf.get_mut(area.x + area.width - 1, area.y).set_style(Style::default().fg(border_color));

    render_agent_header(area, buf, border_color, accent_primary);
    fill_agent_list_interior(area, buf, bg_panel, border_color);

    // Demo agents
    let demo_agents = vec![
        ("coder", "coder", "assistant", "editing files", "claude-4", 45, AgentStatus::Running),
        ("test", "test", "system", "running tests", "gpt-4", 12, AgentStatus::Completed),
    ];

    let mut current_y = area.y + 1;
    let max_y = area.y + area.height - 1;

    for agent in demo_agents {
        if current_y + 3 >= max_y {
            break;
        }
        render_agent_item_row(
            area, buf, bg_panel, text_secondary, text_dim,
            accent_primary, success, error, agent, current_y, max_y,
        );
        current_y += 4;
    }

    render_agent_list_bottom_border(area, buf, border_color);
}

// ─── Context Panel ────────────────────────────────────────────────────────────

fn render_recent_files(
    area: Rect, buf: &mut Buffer,
    text_secondary: ratatui::style::Color,
    accent_primary: ratatui::style::Color,
    left_margin: u16,
    max_width: u16,
    y: &mut u16,
) {
    use ratatui::text::{Line, Span};

    if *y < area.y + area.height {
        let header = Span::styled("RECENT", Style::default().fg(accent_primary).add_modifier(ratatui::style::Modifier::BOLD));
        let line = Line::from(vec![header]);
        buf.set_line(area.x + left_margin, *y, &line, max_width);
        *y += 1;

        for file in &["src/main.rs", "Cargo.toml", "README.md"] {
            if *y >= area.y + area.height {
                break;
            }
            let file_span = Span::styled(format!("▸ {}", file), Style::default().fg(text_secondary));
            let line = Line::from(vec![file_span]);
            buf.set_line(area.x + left_margin, *y, &line, max_width);
            *y += 1;
        }
    }
}

fn render_context_model_session(
    area: Rect, buf: &mut Buffer,
    text_secondary: ratatui::style::Color,
    text_muted: ratatui::style::Color,
    accent_secondary: ratatui::style::Color,
    left_margin: u16,
    max_width: u16,
    y: &mut u16,
) {
    use ratatui::text::{Line, Span};

    // Model
    if *y < area.y + area.height {
        let model_label = Span::styled("Model: ", Style::default().fg(text_muted));
        let model_name = Span::styled("claude-4".to_string(), Style::default().fg(accent_secondary));
        let line = Line::from(vec![model_label, model_name]);
        buf.set_line(area.x + left_margin, *y, &line, max_width);
        *y += 1;
    }

    // Session
    if *y < area.y + area.height {
        let session_label = Span::styled("Session: ", Style::default().fg(text_muted));
        let session_info = Span::styled("new session".to_string(), Style::default().fg(text_secondary));
        let line = Line::from(vec![session_label, session_info]);
        buf.set_line(area.x + left_margin, *y, &line, max_width);
        *y += 1;
    }
}

fn render_context_panel(_state: &RenderState, area: Rect, buf: &mut Buffer, theme: &ThemeWrapper) {
    use ratatui::style::Modifier;
    use ratatui::text::{Line, Span};

    let bg_panel: ratatui::style::Color = theme.color("bg.panel").into();
    for py in area.y..(area.y + area.height) {
        for px in area.x..(area.x + area.width) {
            if let Some(cell) = buf.cell_mut((px as u16, py as u16)) {
                cell.set_style(Style::default().bg(bg_panel));
            }
        }
    }

    let text_secondary: ratatui::style::Color = theme.color("text.secondary").into();
    let text_muted: ratatui::style::Color = theme.color("text.muted").into();
    let accent_secondary: ratatui::style::Color = theme.color("accent.secondary").into();
    let accent_primary: ratatui::style::Color = theme.color("accent.primary").into();
    let border_unfocused: ratatui::style::Color = theme.color("border.unfocused").into();

    let left_margin = 1u16;
    let max_width = area.width.saturating_sub(left_margin + 1);
    let mut y = area.y;

    render_context_model_session(area, buf, text_secondary, text_muted, accent_secondary, left_margin, max_width, &mut y);

    // Separator
    if y < area.y + area.height {
        let sep = Span::styled("─".repeat(max_width as usize), Style::default().fg(border_unfocused));
        let line = Line::from(vec![sep]);
        buf.set_line(area.x + left_margin, y, &line, max_width);
        y += 1;
    }

    render_recent_files(area, buf, text_secondary, accent_primary, left_margin, max_width, &mut y);
}

// ─── Shadow ───────────────────────────────────────────────────────────────────

fn draw_vertical_shadow(modal_area: Rect, buf: &mut ratatui::buffer::Buffer, shadow_bg: ratatui::style::Color, shadow_fg: ratatui::style::Color) {
    let shadow_x = modal_area.x + modal_area.width;
    if shadow_x < buf.area.width {
        for y in modal_area.y + 1..modal_area.y + modal_area.height + 1 {
            if y < buf.area.height {
                if let Some(cell) = buf.cell_mut((shadow_x, y)) {
                    cell.set_char('░');
                    cell.set_style(Style::default().fg(shadow_fg).bg(shadow_bg));
                }
            }
        }
    }
}

fn draw_horizontal_shadow(modal_area: Rect, buf: &mut ratatui::buffer::Buffer, shadow_bg: ratatui::style::Color, shadow_fg: ratatui::style::Color) {
    let shadow_y = modal_area.y + modal_area.height;
    if shadow_y < buf.area.height {
        for x in modal_area.x + 1..modal_area.x + modal_area.width + 1 {
            if x < buf.area.width {
                if let Some(cell) = buf.cell_mut((x, shadow_y)) {
                    cell.set_char('░');
                    cell.set_style(Style::default().fg(shadow_fg).bg(shadow_bg));
                }
            }
        }
    }
}

fn draw_corner_shadow(modal_area: Rect, buf: &mut ratatui::buffer::Buffer, shadow_bg: ratatui::style::Color, shadow_fg: ratatui::style::Color) {
    let corner_x = modal_area.x + modal_area.width;
    let corner_y = modal_area.y + modal_area.height;
    if corner_x < buf.area.width && corner_y < buf.area.height {
        if let Some(cell) = buf.cell_mut((corner_x, corner_y)) {
            cell.set_char('▒');
            cell.set_style(Style::default().fg(shadow_fg).bg(shadow_bg));
        }
    }
}

pub fn render_shadow(modal_area: Rect, buf: &mut ratatui::buffer::Buffer, theme: &ThemeWrapper) {
    let shadow_bg: ratatui::style::Color = theme.color("bg.base").into();
    let shadow_fg: ratatui::style::Color = theme.color("text.dim").into();

    draw_vertical_shadow(modal_area, buf, shadow_bg, shadow_fg);
    draw_horizontal_shadow(modal_area, buf, shadow_bg, shadow_fg);
    draw_corner_shadow(modal_area, buf, shadow_bg, shadow_fg);
}
