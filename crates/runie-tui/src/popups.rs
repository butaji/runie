use ratatui::{
    layout::Rect,
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use runie_core::Snapshot;

use crate::theme::{
    GLYPH_SELECTED, GLYPH_UNSELECTED, style_popup_border, style_popup_selected,
    style_popup_unselected, style_hint, style_thinking, style_user, style_tool_header,
};

pub fn at_suggestions(f: &mut Frame, snap: &Snapshot) {
    let suggestions = match &snap.at_suggestions {
        Some(s) if !s.is_empty() => s,
        _ => return,
    };
    let selected = snap.at_selected.unwrap_or(0).min(suggestions.len().saturating_sub(1));
    let area = f.area();
    let display_count = suggestions.len().min(8) as u16;
    let max_height = display_count + 4;
    let popup_area = Rect {
        x: area.x + 1,
        y: area.y + area.height.saturating_sub(4 + max_height),
        width: area.width.saturating_sub(2).max(20),
        height: max_height,
    };
    let mut lines: Vec<Line> = suggestions
        .iter()
        .take(8)
        .enumerate()
        .map(|(i, s)| {
            let prefix = if i == selected { GLYPH_SELECTED } else { GLYPH_UNSELECTED };
            let style = if i == selected { style_popup_selected() } else { style_popup_unselected() };
            Line::from(format!("{}{}", prefix, s)).style(style)
        })
        .collect();
    lines.push(Line::from(""));
    lines.push(Line::from("Tab=cycle Enter=insert Esc=close").style(style_hint()));
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" @ files ({}) ", suggestions.len()))
        .border_style(style_popup_border());
    f.render_widget(Paragraph::new(lines).block(block), popup_area);
}

pub fn path_suggestions(f: &mut Frame, snap: &Snapshot) {
    let items = match &snap.path_suggestions {
        Some(s) if !s.is_empty() => s,
        _ => return,
    };
    let selected = snap.path_selected.unwrap_or(0).min(items.len().saturating_sub(1));
    let area = f.area();
    let display_count = items.len().min(8) as u16;
    let max_height = display_count + 4;
    let popup_area = Rect {
        x: area.x + 1,
        y: area.y + area.height.saturating_sub(4 + max_height),
        width: area.width.saturating_sub(2).max(20),
        height: max_height,
    };
    let mut lines: Vec<Line> = items
        .iter()
        .take(8)
        .enumerate()
        .map(|(i, item)| {
            let prefix = if i == selected { GLYPH_SELECTED } else { GLYPH_UNSELECTED };
            let style = if i == selected { style_popup_selected() } else { style_popup_unselected() };
            let suffix = if item.is_dir { "/" } else { "" };
            Line::from(format!("{}{}{}", prefix, item.path, suffix)).style(style)
        })
        .collect();
    lines.push(Line::from(""));
    lines.push(Line::from("↑/↓=nav Enter=select Esc=close").style(style_hint()));
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" paths ({}) ", items.len()))
        .border_style(style_popup_border());
    f.render_widget(Paragraph::new(lines).block(block), popup_area);
}

pub fn command_palette(f: &mut Frame, snap: &Snapshot) {
    let (filter, selected) = match &snap.dialog {
        Some(runie_core::commands::DialogState::CommandPalette { filter, selected }) => (filter.clone(), *selected),
        _ => return,
    };
    let popup_area = palette_popup_rect(f.area());
    let lines = build_palette_lines(snap, &filter, selected);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Commands ")
        .border_style(style_popup_border());
    f.render_widget(Paragraph::new(lines).block(block), popup_area);
}

fn palette_popup_rect(area: Rect) -> Rect {
    let popup_width = 60u16.min(area.width.saturating_sub(4)).max(20);
    let popup_height = 18u16.min(area.height.saturating_sub(4)).max(6);
    Rect {
        x: area.x + (area.width.saturating_sub(popup_width)) / 2,
        y: area.y + (area.height.saturating_sub(popup_height)) / 2,
        width: popup_width,
        height: popup_height,
    }
}

pub fn settings_dialog(f: &mut Frame, snap: &Snapshot) {
    let (category, selected) = match &snap.dialog {
        Some(runie_core::commands::DialogState::Settings { category, selected }) => (*category, *selected),
        _ => return,
    };
    let popup_area = palette_popup_rect(f.area());
    let mut lines: Vec<Line> = Vec::new();
    let cats = runie_core::settings::SettingsCategory::all();
    let cat_labels: Vec<String> = cats
        .iter()
        .map(|c| {
            let label = c.as_str();
            if *c == category { format!("[{}]", label) } else { label.to_string() }
        })
        .collect();
    lines.push(Line::from(cat_labels.join(" | ")).style(style_thinking()));
    lines.push(Line::from(""));

    let category_items: Vec<_> = snap.settings_items.iter().filter(|i| i.category == category).collect();
    if category_items.is_empty() {
        lines.push(Line::from("No settings in this category.").style(style_hint()));
    } else {
        for (i, item) in category_items.iter().enumerate() {
            let value_str = match &item.value {
                runie_core::settings::SettingValue::Bool(v) => if *v { "on".to_string() } else { "off".to_string() },
                runie_core::settings::SettingValue::Enum { current, .. } => current.clone(),
            };
            let style = if i == selected { style_popup_selected() } else { style_popup_unselected() };
            lines.push(Line::from(format!("    {:20} {}", item.label, value_str)).style(style));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from("←/→=tab ↑/↓=nav Enter=toggle Esc=close").style(style_hint()));

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Settings ")
        .border_style(style_popup_border());
    f.render_widget(Paragraph::new(lines).block(block), popup_area);
}

pub fn model_selector_dialog(f: &mut Frame, snap: &Snapshot) {
    let (filter, selected) = match &snap.dialog {
        Some(runie_core::commands::DialogState::ModelSelector { filter, selected }) => (filter.clone(), *selected),
        _ => return,
    };
    let popup_area = palette_popup_rect(f.area());
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(format!("> {}", filter)).style(style_user()));
    lines.push(Line::from(""));

    if snap.model_selector_items.is_empty() {
        lines.push(Line::from("No models found").style(style_hint()));
    } else {
        let mut last_header = String::new();
        for (i, (header, name, cost, _is_selected, is_current)) in snap.model_selector_items.iter().enumerate() {
            if !header.is_empty() && header != &last_header {
                if !last_header.is_empty() {
                    lines.push(Line::from(""));
                }
                lines.push(Line::from(format!("  {}", header)).style(style_thinking()));
                last_header = header.clone();
            }
            let star = if *is_current { "★ " } else { "  " };
            let cost_part = if cost.is_empty() { String::new() } else { format!("  {}", cost) };
            let style = if i == selected { style_popup_selected() } else { style_popup_unselected() };
            lines.push(Line::from(format!("{}{}{}", star, name, cost_part)).style(style));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from("↑/↓=nav Enter=select Esc=close").style(style_hint()));

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Select Model ")
        .border_style(style_popup_border());
    f.render_widget(Paragraph::new(lines).block(block), popup_area);
}

pub fn scoped_models_dialog(f: &mut Frame, snap: &Snapshot) {
    let selected = match &snap.dialog {
        Some(runie_core::commands::DialogState::ScopedModels { selected }) => *selected,
        _ => return,
    };
    let popup_area = palette_popup_rect(f.area());
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from("Scoped Models — Space=toggle a/x=all/none p=provider").style(style_tool_header()));
    lines.push(Line::from(""));

    if snap.scoped_models.is_empty() {
        lines.push(Line::from("No models configured.").style(style_hint()));
    } else {
        let mut last_provider = String::new();
        for (i, model) in snap.scoped_models.iter().enumerate() {
            if model.provider != last_provider {
                if !last_provider.is_empty() {
                    lines.push(Line::from(""));
                }
                lines.push(Line::from(format!("  {}", model.provider)).style(style_thinking()));
                last_provider = model.provider.clone();
            }
            let checkbox = if model.enabled { "[x]" } else { "[ ]" };
            let style = if i == selected { style_popup_selected() } else { style_popup_unselected() };
            lines.push(Line::from(format!("    {} {}", checkbox, model.name)).style(style));
        }
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Scoped Models ")
        .border_style(style_popup_border());
    f.render_widget(Paragraph::new(lines).block(block), popup_area);
}

pub fn session_tree_dialog(f: &mut Frame, snap: &Snapshot) {
    let (filter, selected) = match &snap.dialog {
        Some(runie_core::commands::DialogState::SessionTree { filter, selected }) => (*filter, *selected),
        _ => return,
    };
    let popup_area = palette_popup_rect(f.area());
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(format!("Session Tree — filter: {}", filter.as_str())).style(style_tool_header()));
    lines.push(Line::from(""));

    let tree_items: Vec<(usize, String)> = snap.session_tree_items.clone();
    if tree_items.is_empty() {
        lines.push(Line::from("No session tree. Use /fork or /clone to create branches.").style(style_hint()));
    } else {
        for (i, (depth, content)) in tree_items.iter().enumerate() {
            let indent = "  ".repeat(*depth);
            let prefix = if i == selected { "> " } else { "  " };
            let style = if i == selected { style_popup_selected() } else { style_popup_unselected() };
            let truncated: String = content.chars().take(50).collect();
            lines.push(Line::from(format!("{}{}{}{}", indent, prefix, truncated, if content.len() > 50 { "…" } else { "" })).style(style));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from("↑/↓=nav Enter=select f=cycle-filter Esc=close").style(style_hint()));

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Session Tree ")
        .border_style(style_popup_border());
    f.render_widget(Paragraph::new(lines).block(block), popup_area);
}

fn build_palette_lines<'a>(snap: &'a Snapshot, filter: &str, selected: usize) -> Vec<Line<'a>> {
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(format!("> {}", filter)).style(style_user()));
    // Separator
    let sep_width = 56usize;
    lines.push(Line::from("─".repeat(sep_width)).style(style_hint()));

    if snap.palette_items.is_empty() {
        lines.push(Line::from("No commands found").style(style_hint()));
        lines.push(Line::from(""));
        lines.push(Line::from("↑↓ navigate · enter select · esc close").style(style_hint()));
        return lines;
    }

    let mut last_category = String::new();
    for (i, (name, desc, category)) in snap.palette_items.iter().enumerate() {
        if category != &last_category {
            if !last_category.is_empty() {
                lines.push(Line::from(""));
            }
            lines.push(Line::from(format!("  {}", category)).style(style_thinking()));
            last_category = category.clone();
        }
        let prefix = if i == selected { GLYPH_SELECTED } else { GLYPH_UNSELECTED };
        let style = if i == selected { style_popup_selected() } else { style_popup_unselected() };
        lines.push(Line::from(format!("{}{:12} {}", prefix, name, desc)).style(style));
    }
    lines.push(Line::from(""));
    lines.push(Line::from("↑↓ navigate · enter select · esc close").style(style_hint()));
    lines
}
