//! Pipe render_content module.

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
};
use crate::style::layout::SIDEBAR_WIDTH;
use crate::tui::view_models::ViewModels;
use crate::theme::ThemeWrapper;
use crate::theme::ThemeColors;
use crate::components::MessageList;

pub fn render_content(
    buf: &mut Buffer,
    vms: &ViewModels,
    state: &crate::tui::AppState,
    show_sidebar: bool,
    area: Rect,
    theme: &ThemeWrapper,
    theme_colors: &ThemeColors,
) {
    use crate::components::activity_panel::{ActivityPanel, ACTIVITY_PANEL_WIDTH, should_show_activity_panel, render_activity_panel};

    // Show activity panel only when: width >= 100, agent is running, and has background jobs
    let show_activity = should_show_activity_panel(area.width)
        && state.agent_running
        && !state.background_jobs.is_empty();
    let activity_width = if show_activity { ACTIVITY_PANEL_WIDTH } else { 0 };

    let mut h_constraints = vec![Constraint::Min(20)];
    if show_sidebar && area.width >= SIDEBAR_WIDTH + 20 {
        h_constraints.push(Constraint::Length(SIDEBAR_WIDTH));
    }
    if activity_width > 0 {
        h_constraints.push(Constraint::Length(activity_width));
    }

    let h_areas = Layout::horizontal(h_constraints.as_slice()).split(area);
    let feed_area = h_areas[0];
    if let Some(ref msg_list) = vms.message_list {
        MessageList::render_ref(msg_list, feed_area, buf, theme);
    }

    if show_sidebar && area.width >= SIDEBAR_WIDTH + 20 {
        if let Some(ref agent_list) = vms.agent_list {
            crate::tui::render::render_agent_list(agent_list, h_areas[1], buf, theme_colors);
        }
    }

    // Render activity panel on the right
    if show_activity {
        let activity_area_idx = if show_sidebar && area.width >= SIDEBAR_WIDTH + 20 {
            2
        } else {
            1
        };
        if activity_area_idx < h_areas.len() {
            let activity_panel = ActivityPanel::with_jobs(state.background_jobs.clone());
            render_activity_panel(&activity_panel, h_areas[activity_area_idx], buf, theme_colors);
        }
    }
}
