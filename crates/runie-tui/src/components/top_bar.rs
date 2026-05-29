use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
};
use crate::theme::ThemeColors;
use crate::tui::state::TopBarState;

pub mod builder;
pub use builder::*;

#[derive(Clone)]
pub struct TopBarViewModel {
    pub repo: String,
    pub branch: String,
    pub path: String,
    pub model: String,
    pub context_window: usize,
    pub estimated_tokens: usize,
}

impl TopBarViewModel {
    pub fn from_state(state: &TopBarState) -> Self {
        Self {
            repo: state.repo.clone(),
            branch: state.branch.clone(),
            path: state.path.clone(),
            model: state.model.clone(),
            context_window: state.context_window.unwrap_or(128_000),
            estimated_tokens: state.estimated_tokens.unwrap_or(0),
        }
    }
}

fn format_context_window(window: usize) -> String {
    if window >= 1_000_000 {
        format!("{:.0}m", window as f32 / 1_000_000.0)
    } else if window >= 1_000 {
        format!("{:.0}k", window as f32 / 1_000.0)
    } else {
        window.to_string()
    }
}

fn build_left_spans<'a>(vm: &'a TopBarViewModel, bright: Color, _dim: Color, dim_style: &'a Style) -> Vec<Span<'a>> {
    let mut parts = Vec::new();
    if !vm.repo.is_empty() {
        parts.push(Span::styled(&vm.repo, Style::default().fg(bright)));
    }
    if !vm.branch.is_empty() {
        parts.push(Span::styled("/", *dim_style));
        parts.push(Span::styled(&vm.branch, *dim_style));
    }
    if !vm.path.is_empty() {
        parts.push(Span::styled(format!("  {}", vm.path), *dim_style));
    }
    parts
}

fn calculate_pct(vm: &TopBarViewModel) -> f32 {
    if vm.context_window > 0 {
        ((vm.estimated_tokens as f32 / vm.context_window as f32) * 100.0).min(100.0)
    } else {
        0.0
    }
}

fn draw_gauge(gauge_area: Rect, pct: f32, buf: &mut Buffer, text_dim: ratatui::style::Color, text_secondary: ratatui::style::Color, _bg_panel: ratatui::style::Color) {
    let ch = if pct >= 100.0 {
        '■'  // solid square = full/overflow
    } else if pct >= 95.0 {
        '◉'  // full circle with dot = almost full
    } else if pct >= 75.0 {
        '●'  // solid circle
    } else if pct >= 50.0 {
        '◕'  // half-full
    } else if pct >= 25.0 {
        '◐'  // quarter-full
    } else if pct > 0.0 {
        '◔'  // slight fill
    } else {
        '○'  // empty circle
    };

    // Center the gauge char in the gauge area
    let gx = gauge_area.x + (gauge_area.width.saturating_sub(1) / 2);
    if let Some(cell) = buf.cell_mut((gx, gauge_area.y)) {
        cell.set_char(ch);
        if pct >= 95.0 {
            cell.set_style(Style::default().fg(text_secondary).add_modifier(ratatui::style::Modifier::BOLD));
        } else {
            cell.set_style(Style::default().fg(text_dim));
        }
    }
}

pub fn render_top_bar(vm: &TopBarViewModel, area: Rect, buf: &mut Buffer, colors: &ThemeColors) {
    let x = area.x + 1;
    let bright = colors.text_dim;
    let dim = colors.text_dim;
    let dim_style = Style::default().fg(dim).add_modifier(ratatui::style::Modifier::DIM);

    let mut left_parts = vec![Span::styled(" ", Style::default())];
    left_parts.extend(build_left_spans(vm, bright, dim, &dim_style));
    if left_parts.len() > 1 {
        buf.set_line(x, area.y, &Line::from(left_parts), area.width - 2);
    }

    let pct = calculate_pct(vm);
    let window_str = format_context_window(vm.context_window);
    let text = format!("{}/{} {:.0}%", vm.estimated_tokens, window_str, pct);
    let text_len = text.len() as u16;
    let gauge_width = 3u16; // single char gauge centered
    let right_x = area.x + area.width.saturating_sub(text_len + gauge_width + 1);

    if right_x > x {
        buf.set_line(right_x, area.y, &Line::from(vec![Span::styled(text, Style::default().fg(bright))]), text_len);
        draw_gauge(Rect::new(right_x + text_len, area.y, gauge_width, 1), pct, buf, colors.text_dim, colors.text_secondary, colors.bg_panel);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── format_context_window tests ───────────────────────────────────────────

    #[test]
    fn test_format_context_window_raw() {
        assert_eq!(format_context_window(500), "500");
        assert_eq!(format_context_window(999), "999");
    }

    #[test]
    fn test_format_context_window_k() {
        assert_eq!(format_context_window(1_000), "1k");
        assert_eq!(format_context_window(10_000), "10k");
        assert_eq!(format_context_window(120_000), "120k");
    }

    #[test]
    fn test_format_context_window_m() {
        assert_eq!(format_context_window(1_000_000), "1m");
        assert_eq!(format_context_window(1_280_000), "1m");
        assert_eq!(format_context_window(2_000_000), "2m");
    }

    // ─── calculate_pct tests ───────────────────────────────────────────────────

    #[test]
    fn test_calculate_pct_zero() {
        let vm = TopBarViewModel {
            repo: String::new(),
            branch: String::new(),
            path: String::new(),
            model: String::new(),
            context_window: 128_000,
            estimated_tokens: 0,
        };
        assert_eq!(calculate_pct(&vm), 0.0);
    }

    #[test]
    fn test_calculate_pct_50_percent() {
        let vm = TopBarViewModel {
            repo: String::new(),
            branch: String::new(),
            path: String::new(),
            model: String::new(),
            context_window: 100_000,
            estimated_tokens: 50_000,
        };
        assert_eq!(calculate_pct(&vm), 50.0);
    }

    #[test]
    fn test_calculate_pct_100_percent() {
        let vm = TopBarViewModel {
            repo: String::new(),
            branch: String::new(),
            path: String::new(),
            model: String::new(),
            context_window: 100_000,
            estimated_tokens: 100_000,
        };
        assert_eq!(calculate_pct(&vm), 100.0);
    }

    #[test]
    fn test_calculate_pct_over_100_capped() {
        let vm = TopBarViewModel {
            repo: String::new(),
            branch: String::new(),
            path: String::new(),
            model: String::new(),
            context_window: 100_000,
            estimated_tokens: 200_000,
        };
        assert_eq!(calculate_pct(&vm), 100.0);
    }

    #[test]
    fn test_calculate_pct_zero_context_window() {
        let vm = TopBarViewModel {
            repo: String::new(),
            branch: String::new(),
            path: String::new(),
            model: String::new(),
            context_window: 0,
            estimated_tokens: 50_000,
        };
        assert_eq!(calculate_pct(&vm), 0.0);
    }

    // ─── TopBarViewModel::from_state tests ──────────────────────────────────────

    #[test]
    fn test_from_state_with_defaults() {
        let state = TopBarState::default();
        let vm = TopBarViewModel::from_state(&state);
        assert_eq!(vm.repo, "");
        assert_eq!(vm.branch, "");
        assert_eq!(vm.path, "");
        assert_eq!(vm.model, "");
        assert_eq!(vm.context_window, 128_000); // default
        assert_eq!(vm.estimated_tokens, 0); // default
    }

    #[test]
    fn test_from_state_with_values() {
        let mut state = TopBarState::default();
        state.repo = "runie".to_string();
        state.branch = "main".to_string();
        state.path = "src/main.rs".to_string();
        state.model = "claude-3-5-sonnet".to_string();
        state.context_window = Some(200_000);
        state.estimated_tokens = Some(40_000);

        let vm = TopBarViewModel::from_state(&state);
        assert_eq!(vm.repo, "runie");
        assert_eq!(vm.branch, "main");
        assert_eq!(vm.path, "src/main.rs");
        assert_eq!(vm.model, "claude-3-5-sonnet");
        assert_eq!(vm.context_window, 200_000);
        assert_eq!(vm.estimated_tokens, 40_000);
    }

    // ─── render_top_bar tests ───────────────────────────────────────────────────

    fn make_test_colors() -> ThemeColors {
        ThemeColors {
            bg_base: Color::Black,
            bg_panel: Color::Black,
            accent_primary: Color::White,
            text_primary: Color::White,
            text_secondary: Color::White,
            text_dim: Color::Gray,
            text_muted: Color::DarkGray,
            border_unfocused: Color::DarkGray,
            success: Color::Green,
            error: Color::Red,
            syntax_phase: Color::Yellow,
        }
    }

    #[test]
    fn test_render_top_bar_text_appears() {
        // 40/120k case: estimated_tokens=40, context_window=120000
        // Text: "40/120k 0% " + gauge character
        let vm = TopBarViewModel {
            repo: "myrepo".to_string(),
            branch: "feature".to_string(),
            path: "src".to_string(),
            model: "claude".to_string(),
            context_window: 120_000,
            estimated_tokens: 40,
        };

        let colors = make_test_colors();
        let area = Rect::new(0, 0, 80, 1);
        let mut buf = Buffer::empty(area);

        render_top_bar(&vm, area, &mut buf, &colors);

        // Check that text appears in buffer (digits, k, %)
        let content = buf.content();
        let has_4 = content.iter().any(|c| c.symbol() == "4");
        let has_0 = content.iter().any(|c| c.symbol() == "0");
        let has_k = content.iter().any(|c| c.symbol() == "k");
        let has_percent = content.iter().any(|c| c.symbol() == "%");
        assert!(has_4, "Expected '4' in buffer");
        assert!(has_0, "Expected '0' in buffer");
        assert!(has_k, "Expected 'k' in buffer");
        assert!(has_percent, "Expected '%' in buffer");
    }

    #[test]
    fn test_render_top_bar_left_side_text() {
        let vm = TopBarViewModel {
            repo: "runie".to_string(),
            branch: "main".to_string(),
            path: String::new(),
            model: String::new(),
            context_window: 128_000,
            estimated_tokens: 0,
        };

        let colors = make_test_colors();
        let area = Rect::new(0, 0, 80, 1);
        let mut buf = Buffer::empty(area);

        render_top_bar(&vm, area, &mut buf, &colors);

        // Check that repo name characters appear in buffer
        let content = buf.content();
        let has_r = content.iter().any(|c| c.symbol() == "r");
        let has_u = content.iter().any(|c| c.symbol() == "u");
        let has_n = content.iter().any(|c| c.symbol() == "n");
        let has_i = content.iter().any(|c| c.symbol() == "i");
        let has_e = content.iter().any(|c| c.symbol() == "e");
        assert!(has_r, "Expected 'r' in buffer");
        assert!(has_u, "Expected 'u' in buffer");
        assert!(has_n, "Expected 'n' in buffer");
        assert!(has_i, "Expected 'i' in buffer");
        assert!(has_e, "Expected 'e' in buffer");
    }

    #[test]
    fn test_render_top_bar_gauge_rendered() {
        // 50000 / 100000 = 50%
        let vm = TopBarViewModel {
            repo: String::new(),
            branch: String::new(),
            path: String::new(),
            model: String::new(),
            context_window: 100_000,
            estimated_tokens: 50_000,
        };

        let colors = make_test_colors();
        let area = Rect::new(0, 0, 80, 1);
        let mut buf = Buffer::empty(area);

        render_top_bar(&vm, area, &mut buf, &colors);

        let content = buf.content();
        // Text with percentage should appear
        let has_percent = content.iter().any(|c| c.symbol() == "%");
        assert!(has_percent, "Expected '%' in buffer");
    }

    #[test]
    fn test_render_top_bar_empty_vm() {
        let vm = TopBarViewModel {
            repo: String::new(),
            branch: String::new(),
            path: String::new(),
            model: String::new(),
            context_window: 128_000,
            estimated_tokens: 0,
        };

        let colors = make_test_colors();
        let area = Rect::new(0, 0, 80, 1);
        let mut buf = Buffer::empty(area);

        render_top_bar(&vm, area, &mut buf, &colors);

        // With empty repo/branch/path, only the right side text should appear
        // which would be "0/128k " + gauge (battery cells + lightning bolt)
        let content = buf.content();
        let has_0 = content.iter().any(|c| c.symbol().contains("0"));
        let has_128k = content.iter().any(|c| c.symbol().contains("128k") || c.symbol().contains("k"));
        let has_bolt = content.iter().any(|c| c.symbol() == "○");
        assert!(has_0, "Expected '0' in buffer for zero tokens");
        assert!(has_bolt, "Expected gauge character in buffer");
    }

    // ─── format_context_window boundary tests ───────────────────────────────────

    #[test]
    fn test_format_context_window_boundary_999_999() {
        // 999_999 is below 1_000_000, so K path not M
        // 999_999 / 1000 = 999.999 → rounds to "1000k"
        assert_eq!(format_context_window(999_999), "1000k");
    }

    #[test]
    fn test_format_context_window_boundary_1_000_001() {
        // 1_000_001 >= 1_000_000, so M path
        // 1_000_001 / 1_000_000 = 1.000001 → rounds to "1m"
        assert_eq!(format_context_window(1_000_001), "1m");
    }

    // ─── calculate_pct edge case tests ────────────────────────────────────────

    #[test]
    fn test_calculate_pct_float_precision() {
        // 99_999/100_000 = 0.99999 → 99.999% (not rounded to 100%)
        let vm = TopBarViewModel {
            repo: String::new(),
            branch: String::new(),
            path: String::new(),
            model: String::new(),
            context_window: 100_000,
            estimated_tokens: 99_999,
        };
        assert_eq!(calculate_pct(&vm), 99.999);
    }

    #[test]
    fn test_calculate_pct_very_small() {
        // 1/128_000 = 0.0000078125 → 0.00078125% (not rounded to 0)
        let vm = TopBarViewModel {
            repo: String::new(),
            branch: String::new(),
            path: String::new(),
            model: String::new(),
            context_window: 128_000,
            estimated_tokens: 1,
        };
        assert_eq!(calculate_pct(&vm), 0.00078125);
    }

    // ─── TopBarViewModel::from_state partial tests ─────────────────────────────

    #[test]
    fn test_from_state_partial() {
        // Only context_window set, estimated_tokens is None → defaults to 0
        let mut state = TopBarState::default();
        state.context_window = Some(200_000);
        state.estimated_tokens = None;

        let vm = TopBarViewModel::from_state(&state);
        assert_eq!(vm.context_window, 200_000);
        assert_eq!(vm.estimated_tokens, 0); // defaulted
    }

    // ─── build_left_spans partial组合 tests ────────────────────────────────────

    #[test]
    fn test_build_left_spans_repo_only() {
        let vm = TopBarViewModel {
            repo: "runie".to_string(),
            branch: String::new(),
            path: String::new(),
            model: String::new(),
            context_window: 128_000,
            estimated_tokens: 0,
        };
        let dim_style = Style::default();
        let spans = build_left_spans(&vm, Color::White, Color::White, &dim_style);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content.as_ref(), "runie");
    }

    #[test]
    fn test_build_left_spans_branch_only() {
        let vm = TopBarViewModel {
            repo: String::new(),
            branch: "main".to_string(),
            path: String::new(),
            model: String::new(),
            context_window: 128_000,
            estimated_tokens: 0,
        };
        let dim_style = Style::default();
        let spans = build_left_spans(&vm, Color::White, Color::White, &dim_style);
        // No repo, so no leading span - but branch adds "/" + "main"
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].content.as_ref(), "/");
        assert_eq!(spans[1].content.as_ref(), "main");
    }

    #[test]
    fn test_build_left_spans_path_only() {
        let vm = TopBarViewModel {
            repo: String::new(),
            branch: String::new(),
            path: "src/lib.rs".to_string(),
            model: String::new(),
            context_window: 128_000,
            estimated_tokens: 0,
        };
        let dim_style = Style::default();
        let spans = build_left_spans(&vm, Color::White, Color::White, &dim_style);
        // Only path: "  src/lib.rs"
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content.as_ref(), "  src/lib.rs");
    }

    // ─── render_top_bar narrow terminal test ───────────────────────────────────

    #[test]
    fn test_render_top_bar_narrow_terminal() {
        // Terminal width = 18, text = "0/128k 0% " (11 chars) + gauge (3 chars) = 15 total
        // Since 15 < 18, right side renders with percentage
        let vm = TopBarViewModel {
            repo: String::new(),
            branch: String::new(),
            path: String::new(),
            model: String::new(),
            context_window: 128_000,
            estimated_tokens: 0,
        };

        let colors = make_test_colors();
        let area = Rect::new(0, 0, 18, 1);
        let mut buf = Buffer::empty(area);

        render_top_bar(&vm, area, &mut buf, &colors);

        // Right side renders with percentage in narrow terminal
        let content = buf.content();
        let has_percent = content.iter().any(|c| c.symbol().contains("%"));
        assert!(has_percent, "Expected '%' in buffer - right side renders in narrow terminal");
    }

    // ─── render_top_bar gauge boundary tests ───────────────────────────────────

    #[test]
    fn test_render_top_bar_gauge_at_0_percent() {
        // 0% gauge - gauge character ○
        let vm = TopBarViewModel {
            repo: String::new(),
            branch: String::new(),
            path: String::new(),
            model: String::new(),
            context_window: 100_000,
            estimated_tokens: 0,
        };

        let colors = make_test_colors();
        let area = Rect::new(0, 0, 80, 1);
        let mut buf = Buffer::empty(area);

        render_top_bar(&vm, area, &mut buf, &colors);

        let content = buf.content();
        // Gauge character at 0% is ○
        let has_gauge = content.iter().any(|c| c.symbol() == "○");
        assert!(has_gauge, "Expected gauge character in buffer for 0% gauge");
    }

    #[test]
    fn test_render_top_bar_gauge_at_100_percent() {
        // 100% gauge - gauge character ■
        let vm = TopBarViewModel {
            repo: String::new(),
            branch: String::new(),
            path: String::new(),
            model: String::new(),
            context_window: 100_000,
            estimated_tokens: 100_000,
        };

        let colors = make_test_colors();
        let area = Rect::new(0, 0, 80, 1);
        let mut buf = Buffer::empty(area);

        render_top_bar(&vm, area, &mut buf, &colors);

        let content = buf.content();
        // Gauge character at 100% is ■
        let has_gauge = content.iter().any(|c| c.symbol() == "■");
        assert!(has_gauge, "Expected gauge character in buffer for 100% gauge");
    }

    // ─── duplicate percentage + gauge visibility bug regression tests ─────────

    #[test]
    fn test_no_duplicate_percentage() {
        // Text format includes percentage: "{}/{} {:.0}% "
        // "%" should appear exactly once in the buffer
        let vm = TopBarViewModel {
            repo: String::new(),
            branch: String::new(),
            path: String::new(),
            model: String::new(),
            context_window: 100_000,
            estimated_tokens: 50_000, // 50%
        };

        let colors = make_test_colors();
        let area = Rect::new(0, 0, 80, 1);
        let mut buf = Buffer::empty(area);

        render_top_bar(&vm, area, &mut buf, &colors);

        let content = buf.content();
        let percent_count = content.iter().filter(|c| c.symbol().contains("%")).count();
        assert_eq!(
            percent_count, 1,
            "Expected exactly one '%' in buffer, found {}",
            percent_count
        );
    }

    #[test]
    fn test_gauge_label_shows_pct() {
        // Verify text with percentage renders at 50%
        let vm = TopBarViewModel {
            repo: String::new(),
            branch: String::new(),
            path: String::new(),
            model: String::new(),
            context_window: 100_000,
            estimated_tokens: 50_000, // 50%
        };

        let colors = make_test_colors();
        let area = Rect::new(0, 0, 80, 1);
        let mut buf = Buffer::empty(area);

        render_top_bar(&vm, area, &mut buf, &colors);

        let content = buf.content();
        // Check text with percentage appears
        let has_5 = content.iter().any(|c| c.symbol() == "5");
        let has_percent = content.iter().any(|c| c.symbol() == "%");
        assert!(has_5, "Expected '5' in buffer");
        assert!(has_percent, "Expected '%' in buffer for 50%");
    }

    #[test]
    fn test_gauge_width_sufficient() {
        // Verify render doesn't panic at 50%
        let vm = TopBarViewModel {
            repo: String::new(),
            branch: String::new(),
            path: String::new(),
            model: String::new(),
            context_window: 100_000,
            estimated_tokens: 50_000,
        };

        let colors = make_test_colors();
        let area = Rect::new(0, 0, 80, 1);
        let mut buf = Buffer::empty(area);

        // Should not panic
        render_top_bar(&vm, area, &mut buf, &colors);

        // Check text appears
        let content = buf.content();
        let has_percent = content.iter().any(|c| c.symbol() == "%");
        assert!(has_percent, "Text with percentage should appear");
    }

    #[test]
    fn test_text_no_pct_suffix() {
        // Text format: "{estimated_tokens}/{window_str} {:.0}% "
        // The text now includes percentage suffix
        let vm = TopBarViewModel {
            repo: String::new(),
            branch: String::new(),
            path: String::new(),
            model: String::new(),
            context_window: 100_000,
            estimated_tokens: 50_000,
        };

        let colors = make_test_colors();
        let area = Rect::new(0, 0, 80, 1);
        let mut buf = Buffer::empty(area);

        render_top_bar(&vm, area, &mut buf, &colors);

        // The text "50000/100k 50% " includes percentage suffix
        let content = buf.content();
        let percent_count = content.iter().filter(|c| c.symbol().contains("%")).count();
        assert_eq!(
            percent_count, 1,
            "Text should contain percentage suffix"
        );
    }

    #[test]
    fn test_zero_percent_visible() {
        // At 0%, gauge should show gauge character ○
        let vm = TopBarViewModel {
            repo: String::new(),
            branch: String::new(),
            path: String::new(),
            model: String::new(),
            context_window: 100_000,
            estimated_tokens: 0, // 0%
        };

        let colors = make_test_colors();
        let area = Rect::new(0, 0, 80, 1);
        let mut buf = Buffer::empty(area);

        render_top_bar(&vm, area, &mut buf, &colors);

        let content = buf.content();
        // Gauge character at 0% is ○
        let has_zero = content.iter().any(|c| c.symbol() == "0");
        let has_gauge = content.iter().any(|c| c.symbol() == "○");
        assert!(has_zero, "At 0%, should show '0' digit for tokens");
        assert!(has_gauge, "At 0%, gauge should show gauge character");
    }

    #[test]
    fn test_gauge_visible_at_zero_percent() {
        // At 0% ratio, gauge should show gauge character ○
        let vm = TopBarViewModel {
            repo: String::new(),
            branch: String::new(),
            path: String::new(),
            model: String::new(),
            context_window: 100_000,
            estimated_tokens: 0, // 0% ratio
        };

        let colors = make_test_colors();
        let area = Rect::new(0, 0, 80, 1);
        let mut buf = Buffer::empty(area);

        render_top_bar(&vm, area, &mut buf, &colors);

        let content = buf.content();
        // Gauge character at 0% is ○
        let has_gauge = content.iter().any(|c| c.symbol() == "○");
        assert!(has_gauge, "Gauge area should have gauge character at 0%");
    }
}
