use ratatui::{
    layout::Rect,
    style::Style,
    text::Line,
    widgets::{Clear, Paragraph},
    Frame,
};
use runie_core::Snapshot;

use crate::theme::{
    GLYPH_SELECTED, GLYPH_UNSELECTED, block_popup, style_popup_selected,
    style_popup_unselected, style_hint, style_thinking, style_user, style_tool_header,
    color_bg_panel,
};
use crate::ui::{parse_hint_spans, render_scrollbar};

pub mod panel;

/// Build a Paragraph with the panel background color baked in.
fn popup_p<'a>(lines: Vec<Line<'a>>) -> Paragraph<'a> {
    Paragraph::new(lines).style(Style::default().bg(color_bg_panel()))
}

/// Clear the given rect with the panel background color.
fn clear_panel_bg(f: &mut Frame, area: Rect) {
    f.render_widget(Clear, area);
    f.buffer_mut().set_style(area, Style::default().bg(color_bg_panel()));
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
    clear_panel_bg(f, popup_area);
    f.render_widget(
        Paragraph::new(lines)
            .style(Style::default().bg(color_bg_panel()))
            .block(block_popup(&format!(" paths ({}) ", items.len()))),
        popup_area,
    );
}

pub fn command_palette(f: &mut Frame, snap: &Snapshot) {
    let (filter, selected) = match &snap.dialog {
        Some(runie_core::commands::DialogState::CommandPalette { filter, selected }) => (filter.clone(), *selected),
        _ => return,
    };

    let popup_area = palette_popup_rect(f.area());
    clear_panel_bg(f, popup_area);
    let block = block_popup(" Commands ");
    let inner = block.inner(popup_area);
    f.render_widget(Paragraph::new("").block(block), popup_area);

    // Reserve 2 lines at bottom: 1 empty spacer + 1 hotkeys
    let content_height = inner.height.saturating_sub(2);
    let hotkeys_area = Rect {
        x: inner.x,
        y: inner.y + content_height,
        width: inner.width,
        height: 2,
    };
    let content_area = Rect {
        height: content_height,
        ..inner
    };

    // Header: filter + separator (always visible, not scrolled)
    let header_height = 2u16;
    let header_area = Rect {
        height: header_height,
        ..content_area
    };
    let items_area = Rect {
        x: content_area.x,
        y: content_area.y + header_height,
        height: content_height.saturating_sub(header_height),
        width: content_area.width,
    };

    let sep_width = inner.width as usize;
    let header_lines = vec![
        Line::from(format!("❯ {}", filter)).style(style_user()),
        Line::from("─".repeat(sep_width)).style(style_hint()),
    ];

    // Build item lines and track which line is selected
    let mut item_lines: Vec<Line> = Vec::new();
    let mut selected_line: Option<usize> = None;

    if snap.palette_items.is_empty() {
        item_lines.push(Line::from("No commands found").style(style_hint()));
    } else {
        let mut last_category = String::new();
        for (i, (name, desc, category)) in snap.palette_items.iter().enumerate() {
            if category != &last_category {
                if !last_category.is_empty() {
                    item_lines.push(Line::from(""));
                }
                item_lines.push(Line::from(format!("  {}", category)).style(style_thinking()));
                last_category = category.clone();
            }
            if i == selected {
                selected_line = Some(item_lines.len());
            }
            let prefix = if i == selected { GLYPH_SELECTED } else { GLYPH_UNSELECTED };
            let style = if i == selected { style_popup_selected() } else { style_popup_unselected() };
            item_lines.push(Line::from(format!("{}{:12} {}", prefix, name, desc)).style(style));
        }
    }

    let total_item_lines = item_lines.len();
    let visible_items_height = items_area.height as usize;

    let scroll_offset = if let Some(sel) = selected_line {
        if total_item_lines <= visible_items_height {
            0
        } else {
            sel.saturating_sub(visible_items_height / 2)
                .min(total_item_lines.saturating_sub(visible_items_height))
        }
    } else {
        0
    };

    let show_scrollbar = total_item_lines > visible_items_height;
    let items_width = if show_scrollbar {
        items_area.width.saturating_sub(1)
    } else {
        items_area.width
    };
    let scroll_area = Rect { width: items_width, ..items_area };
    let scrollbar_area = Rect {
        x: items_area.x + items_width,
        y: items_area.y,
        width: 1,
        height: items_area.height,
    };

    f.render_widget(popup_p(header_lines), header_area);
    f.render_widget(
        popup_p(item_lines).scroll((scroll_offset as u16, 0)),
        scroll_area,
    );

    if show_scrollbar {
        render_scrollbar(f, scrollbar_area, total_item_lines, scroll_offset as u16, visible_items_height);
    }

    // Hotkeys always pinned to bottom with shared parser (empty line + styled hints)
    let hint_lines = vec![
        Line::from(""),
        Line::from(parse_hint_spans("↑↓ navigate · enter select · esc close")),
    ];
    f.render_widget(popup_p(hint_lines), hotkeys_area);
}

pub fn palette_popup_rect(area: Rect) -> Rect {
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

    clear_panel_bg(f, popup_area);
    f.render_widget(
        Paragraph::new(lines)
            .style(Style::default().bg(color_bg_panel()))
            .block(block_popup(" Settings ")),
        popup_area,
    );
}

pub fn model_selector_dialog(f: &mut Frame, snap: &Snapshot) {
    let (filter, selected) = match &snap.dialog {
        Some(runie_core::commands::DialogState::ModelSelector { filter, selected }) => (filter.clone(), *selected),
        _ => return,
    };
    let popup_area = palette_popup_rect(f.area());
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(format!("❯ {}", filter)).style(style_user()));
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

    clear_panel_bg(f, popup_area);
    f.render_widget(
        Paragraph::new(lines)
            .style(Style::default().bg(color_bg_panel()))
            .block(block_popup(" Select Model ")),
        popup_area,
    );
}

pub fn scoped_models_dialog(f: &mut Frame, snap: &Snapshot) {
    let selected = match &snap.dialog {
        Some(runie_core::commands::DialogState::ScopedModels { selected }) => *selected,
        _ => return,
    };
    let popup_area = palette_popup_rect(f.area());
    clear_panel_bg(f, popup_area);
    let block = block_popup(" Scoped Models ");
    let inner = block.inner(popup_area);
    f.render_widget(Paragraph::new("").block(block), popup_area);

    // Reserve 2 lines at bottom: 1 empty spacer + 1 hotkeys (pinned, never scrolls)
    let content_height = inner.height.saturating_sub(2);
    let hotkeys_area = Rect {
        x: inner.x,
        y: inner.y + content_height,
        width: inner.width,
        height: 2,
    };
    let content_area = Rect {
        height: content_height,
        ..inner
    };

    // Header: sub-title (1 line) + separator (1 line)
    let header_height = 2u16;
    let header_area = Rect {
        height: header_height,
        ..content_area
    };
    let items_area = Rect {
        x: content_area.x,
        y: content_area.y + header_height,
        height: content_height.saturating_sub(header_height),
        width: content_area.width,
    };

    let sep_width = inner.width as usize;
    let header_lines = vec![
        Line::from("space toggle · a all · x none · p provider").style(style_hint()),
        Line::from("─".repeat(sep_width)).style(style_hint()),
    ];

    // Build item lines and track which line is selected
    let mut item_lines: Vec<Line> = Vec::new();
    let mut selected_line: Option<usize> = None;

    if snap.scoped_models.is_empty() {
        item_lines.push(Line::from("No models configured.").style(style_hint()));
    } else {
        let mut last_provider = String::new();
        for (i, model) in snap.scoped_models.iter().enumerate() {
            if model.provider != last_provider {
                if !last_provider.is_empty() {
                    item_lines.push(Line::from(""));
                }
                item_lines.push(Line::from(format!("  {}", model.provider)).style(style_thinking()));
                last_provider = model.provider.clone();
            }
            if i == selected {
                selected_line = Some(item_lines.len());
            }
            let checkbox = if model.enabled { "[x]" } else { "[ ]" };
            let style = if i == selected { style_popup_selected() } else { style_popup_unselected() };
            item_lines.push(Line::from(format!("    {} {}", checkbox, model.name)).style(style));
        }
    }

    let total_item_lines = item_lines.len();
    let visible_items_height = items_area.height as usize;

    let scroll_offset = if let Some(sel) = selected_line {
        if total_item_lines <= visible_items_height {
            0
        } else {
            sel.saturating_sub(visible_items_height / 2)
                .min(total_item_lines.saturating_sub(visible_items_height))
        }
    } else {
        0
    };

    let show_scrollbar = total_item_lines > visible_items_height;
    let items_width = if show_scrollbar {
        items_area.width.saturating_sub(1)
    } else {
        items_area.width
    };
    let scroll_area = Rect { width: items_width, ..items_area };
    let scrollbar_area = Rect {
        x: items_area.x + items_width,
        y: items_area.y,
        width: 1,
        height: items_area.height,
    };

    f.render_widget(popup_p(header_lines), header_area);
    f.render_widget(
        popup_p(item_lines).scroll((scroll_offset as u16, 0)),
        scroll_area,
    );

    if show_scrollbar {
        render_scrollbar(f, scrollbar_area, total_item_lines, scroll_offset as u16, visible_items_height);
    }

    // Hotkeys pinned to bottom with shared parser (empty line + styled hints)
    let hint_lines = vec![
        Line::from(""),
        Line::from(parse_hint_spans("↑↓ navigate · space toggle · esc close")),
    ];
    f.render_widget(popup_p(hint_lines), hotkeys_area);
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
            let prefix = if i == selected { "❯ " } else { "  " };
            let style = if i == selected { style_popup_selected() } else { style_popup_unselected() };
            let truncated: String = content.chars().take(50).collect();
            lines.push(Line::from(format!("{}{}{}{}", indent, prefix, truncated, if content.len() > 50 { "…" } else { "" })).style(style));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from("↑/↓=nav Enter=select f=cycle-filter Esc=close").style(style_hint()));

    clear_panel_bg(f, popup_area);
    f.render_widget(
        Paragraph::new(lines)
            .style(Style::default().bg(color_bg_panel()))
            .block(block_popup(" Session Tree ")),
        popup_area,
    );
}



