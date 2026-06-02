//! RenderPipe mode-specific rendering.

use ratatui::{buffer::Buffer, layout::Rect, style::Style, widgets::Widget};

use crate::tui::view_models::ViewModels;
use crate::tui::AppState;
use crate::theme::ThemeWrapper;
use crate::theme::ThemeColors;

use super::helpers::clear_background;

pub fn render_onboarding_mode(
    buf: &mut Buffer,
    area: Rect,
    state: &AppState,
    vms: &ViewModels,
    main_areas: [Rect; 5],
    show_status_bar: bool,
    theme: &ThemeWrapper,
    theme_colors: &ThemeColors,
) {
    let bg_base: ratatui::style::Color = theme.color("bg.base").into();
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_char(' ');
                cell.set_style(ratatui::style::Style::default().bg(bg_base));
            }
        }
    }

    // Render top bar
    crate::components::top_bar::render_top_bar(&vms.top_bar, main_areas[0], buf, theme_colors);

    if let Some(ref onboarding) = state.onboarding {
        let onboarding_area = Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: area.height - if show_status_bar { 2 } else { 0 },
        };
        crate::components::onboarding::render::render_onboarding(onboarding, onboarding_area, buf, theme);
    }

    if show_status_bar {
        crate::components::status_bar::render_ref(&vms.status_bar, main_areas[4], buf, theme_colors);
    }
}

pub fn render_home_screen_mode(
    buf: &mut Buffer,
    area: Rect,
    state: &AppState,
    vms: &ViewModels,
    main_areas: [Rect; 5],
    theme: &ThemeWrapper,
    theme_colors: &ThemeColors,
) {
    clear_background(buf, area, theme_colors.bg_base);
    crate::components::top_bar::render_top_bar(&vms.top_bar, main_areas[0], buf, theme_colors);

    // Render home screen in the content area (between top bar and input)
    let home_area = Rect {
        x: main_areas[1].x,
        y: main_areas[1].y,
        width: main_areas[1].width,
        height: main_areas[1].height + main_areas[2].height, // content + global tags area
    };

    if state.home_screen.show_sessions {
        // Render session list view using SessionTreeNavigator
        // Create a temporary clone with visible=true to bypass the visibility check
        let mut session_tree = state.session_tree.clone();
        session_tree.visible = true;
        session_tree.render_ref(home_area, buf, theme);
    } else {
        // Render welcome menu
        crate::components::home_screen::render_home_screen(&state.home_screen, home_area, buf, theme);
    }

    // Clear input_right_info in home screen mode to show "runie" instead of "runie ─ mock"
    let clean_state = {
        let mut s = state.clone();
        s.input_right_info = String::new();
        s
    };
    super::render_input::render_input(buf, &clean_state, main_areas[3], theme, theme_colors);

    // Render version badge after input bar
    let version_badge = format!("{} Beta", env!("CARGO_PKG_VERSION"));
    let badge_y = main_areas[3].bottom();
    let badge_x = main_areas[3].right().saturating_sub(version_badge.len() as u16 + 2);
    buf.set_string(badge_x, badge_y, &version_badge, theme.version_style());
}

pub fn render_normal_mode(
    buf: &mut Buffer,
    area: Rect,
    state: &AppState,
    vms: &ViewModels,
    main_areas: [Rect; 5],
    show_sidebar: bool,
    show_status_bar: bool,
    palette: &crate::components::CommandPalette,
    padded: Rect,
    theme: &ThemeWrapper,
    theme_colors: &ThemeColors,
) {
    

    clear_background(buf, area, theme_colors.bg_base);
    // main_areas[0] = topbar, [1] = feed, [2] = global_tags, [3] = input, [4] = hotkeys
    crate::components::top_bar::render_top_bar(&vms.top_bar, main_areas[0], buf, theme_colors);
    super::render_content::render_content(buf, vms, state, show_sidebar, main_areas[1], theme, theme_colors);
    crate::components::global_tags::render_global_tags(&vms.global_tags, main_areas[2], buf, theme_colors);
    if state.slash_menu.is_open() {
        let menu_h = 12u16.min(main_areas[1].height.saturating_sub(2));
        let menu_area = Rect {
            x: main_areas[1].x,
            y: main_areas[2].y.saturating_sub(menu_h),
            width: main_areas[1].width,
            height: menu_h,
        };
        crate::components::slash_menu::render_slash_menu(&state.slash_menu, menu_area, buf, theme);
    }
    if state.file_picker.is_open() {
        let picker_h = 16u16.min(main_areas[1].height.saturating_sub(2));
        let picker_area = Rect {
            x: main_areas[1].x,
            y: main_areas[2].y.saturating_sub(picker_h),
            width: main_areas[1].width,
            height: picker_h,
        };
        (&state.file_picker).render(picker_area, buf);
    }
    super::render_input::render_input(buf, state, main_areas[3], theme, theme_colors);
    if show_status_bar {
        crate::components::status_bar::render_ref(&vms.status_bar, main_areas[4], buf, theme_colors);
    }
    super::overlays::render_overlays(buf, state, palette, padded, area, theme, theme_colors);
}
