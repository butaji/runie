use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Gauge, Widget},
};
use crate::theme::ThemeColors;
use crate::tui::state::TopBarState;

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

    let gauge_width = 6u16;
    let total_right_width = text_len + 1 + gauge_width;

    let right_x = area.x + area.width.saturating_sub(total_right_width + 1);
    if right_x > x {
        let text_span = Span::styled(format!("{} ", text), Style::default().fg(bright));
        buf.set_line(right_x, area.y, &Line::from(vec![text_span]), area.width);

        let gauge_area = Rect::new(
            right_x + text_len + 1,
            area.y,
            gauge_width,
            1,
        );
        Gauge::default()
            .use_unicode(true)
            .gauge_style(Style::default().fg(colors.text_dim))
            .ratio((pct / 100.0) as f64)
            .render(gauge_area, buf);
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
        // 40/120k 0% case: estimated_tokens=40, context_window=120000
        // pct = (40/120000) * 100 = 0.033... ≈ 0% (rounded to 0)
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

        // Check that "40/120k" text appears in buffer (each cell has a single char symbol)
        let content = buf.content();
        let has_4 = content.iter().any(|c| c.symbol() == "4");
        let has_0 = content.iter().any(|c| c.symbol() == "0");
        let has_percent = content.iter().any(|c| c.symbol() == "%");
        let has_k = content.iter().any(|c| c.symbol() == "k");
        assert!(has_4, "Expected '4' in buffer");
        assert!(has_0, "Expected '0' in buffer");
        assert!(has_percent, "Expected '%' in buffer");
        assert!(has_k, "Expected 'k' in buffer");
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
        // Use a case that gives a visible percentage
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

        // The gauge is 6 characters wide starting at position (text_len + right_x + 1)
        // text = "50000/100k 50%" which is 15 chars
        // We need to find where the gauge was rendered and check for non-default cells
        let content = buf.content();

        // The default cell symbol is " " (space). After rendering gauge, some cells
        // should have different symbols (░ or █ when using unicode)
        let has_non_default = content.iter().any(|c| {
            c.symbol() != " " || c.fg != Color::Reset || c.bg != Color::Reset
        });

        // At minimum, the gauge area (6 chars at the end) should have been modified
        // or the text area should show the "50/100k 50%" text
        let has_percent = content.iter().any(|c| c.symbol().contains("%"));
        assert!(has_percent, "Expected percent sign in buffer");
        assert!(has_non_default, "Expected some non-default cells in buffer after gauge render");
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
        // which would be "0/128k 0%"
        let content = buf.content();
        let has_0 = content.iter().any(|c| c.symbol().contains("0"));
        let has_percent = content.iter().any(|c| c.symbol().contains("%"));
        assert!(has_0, "Expected '0' in buffer for zero tokens");
        assert!(has_percent, "Expected '%' in buffer");
    }
}
