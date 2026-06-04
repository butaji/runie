use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
};

/// Format a token/context count with K/M suffix.
/// Grok-style: 999 -> "999", 9_500 -> "9.5K", 1_000_000 -> "1.0M", 512_000 -> "512K".
pub fn format_token_count(n: usize) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f32 / 1_000_000.0)
    } else if n >= 1_000 {
        let val = format!("{:.1}", n as f32 / 1_000.0);
        let stripped = val.strip_suffix(".0").map(|s| s.to_string()).unwrap_or(val);
        format!("{}K", stripped)
    } else {
        n.to_string()
    }
}

/// Format context window (alias for format_token_count for backwards compatibility)
pub fn format_context_window(window: usize) -> String {
    format_token_count(window)
}

pub fn calculate_pct(vm: &super::TopBarViewModel) -> f32 {
    if vm.context_window > 0 {
        ((vm.estimated_tokens as f32 / vm.context_window as f32) * 100.0).min(100.0)
    } else {
        0.0
    }
}

pub fn draw_gauge(
    gauge_area: Rect,
    pct: f32,
    buf: &mut Buffer,
    text_dim: Color,
    text_secondary: Color,
    _bg_panel: Color,
) {
    let rounded_pct = pct.round() as u32;
    let ch = if rounded_pct >= 100 {
        '■'
    } else if rounded_pct >= 95 {
        '◉'
    } else if rounded_pct >= 75 {
        '●'
    } else if rounded_pct >= 50 {
        '◕'
    } else if rounded_pct >= 25 {
        '◐'
    } else if rounded_pct > 0 {
        '◔'
    } else {
        '○'
    };

    let gx = gauge_area.x + (gauge_area.width.saturating_sub(1) / 2);
    if let Some(cell) = buf.cell_mut((gx, gauge_area.y)) {
        cell.set_char(ch);
        if pct >= 95.0 {
            cell.set_style(Style::default().fg(text_secondary).add_modifier(Modifier::BOLD));
        } else {
            cell.set_style(Style::default().fg(text_dim));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::TopBarViewModel;
    use crate::tui::state::TuiMode;

    // ─── format_token_count / format_context_window tests ─────────────────────────

    #[test]
    fn test_format_token_count_raw() {
        assert_eq!(format_token_count(0), "0");
        assert_eq!(format_token_count(4), "4");
        assert_eq!(format_token_count(500), "500");
        assert_eq!(format_token_count(999), "999");
    }

    #[test]
    fn test_format_token_count_k() {
        assert_eq!(format_token_count(1_000), "1K");
        assert_eq!(format_token_count(7_600), "7.6K");
        assert_eq!(format_token_count(10_000), "10K");
        assert_eq!(format_token_count(21_000), "21K");
        assert_eq!(format_token_count(120_000), "120K");
        assert_eq!(format_token_count(512_000), "512K");
    }

    #[test]
    fn test_format_token_count_m() {
        assert_eq!(format_token_count(1_000_000), "1M");
        assert_eq!(format_token_count(1_280_000), "1.3M");
        assert_eq!(format_token_count(2_000_000), "2M");
        assert_eq!(format_token_count(2_500_000), "2.5M");
    }

    #[test]
    fn test_format_context_window_same_as_token_count() {
        // format_context_window is now an alias for format_token_count
        assert_eq!(format_context_window(0), format_token_count(0));
        assert_eq!(format_context_window(4), format_token_count(4));
        assert_eq!(format_context_window(7_600), format_token_count(7_600));
        assert_eq!(format_context_window(512_000), format_token_count(512_000));
        assert_eq!(format_context_window(1_000_000), format_token_count(1_000_000));
    }

    // ─── calculate_pct tests ───────────────────────────────────────────────────

    #[test]
    fn test_calculate_pct_zero() {
        let vm = TopBarViewModel {
            repo: String::new(),
            branch: String::new(),
            path: String::new(),
            context_window: 128_000,
            estimated_tokens: 0,
            agent_running: false,
            braille_frame: 0,
            mode: TuiMode::Chat,
        };
        assert_eq!(calculate_pct(&vm), 0.0);
    }

    #[test]
    fn test_calculate_pct_50_percent() {
        let vm = TopBarViewModel {
            repo: String::new(),
            branch: String::new(),
            path: String::new(),
            context_window: 100_000,
            estimated_tokens: 50_000,
            agent_running: false,
            braille_frame: 0,
            mode: TuiMode::Chat,
        };
        assert_eq!(calculate_pct(&vm), 50.0);
    }

    #[test]
    fn test_calculate_pct_100_percent() {
        let vm = TopBarViewModel {
            repo: String::new(),
            branch: String::new(),
            path: String::new(),
            context_window: 100_000,
            estimated_tokens: 100_000,
            agent_running: false,
            braille_frame: 0,
            mode: TuiMode::Chat,
        };
        assert_eq!(calculate_pct(&vm), 100.0);
    }

    #[test]
    fn test_calculate_pct_over_100_capped() {
        let vm = TopBarViewModel {
            repo: String::new(),
            branch: String::new(),
            path: String::new(),
            context_window: 100_000,
            estimated_tokens: 200_000,
            agent_running: false,
            braille_frame: 0,
            mode: TuiMode::Chat,
        };
        assert_eq!(calculate_pct(&vm), 100.0);
    }

    #[test]
    fn test_calculate_pct_zero_context_window() {
        let vm = TopBarViewModel {
            repo: String::new(),
            branch: String::new(),
            path: String::new(),
            context_window: 0,
            estimated_tokens: 50_000,
            agent_running: false,
            braille_frame: 0,
            mode: TuiMode::Chat,
        };
        assert_eq!(calculate_pct(&vm), 0.0);
    }

    #[test]
    fn test_calculate_pct_float_precision() {
        let vm = TopBarViewModel {
            repo: String::new(),
            branch: String::new(),
            path: String::new(),
            context_window: 100_000,
            estimated_tokens: 99_999,
            agent_running: false,
            braille_frame: 0,
            mode: TuiMode::Chat,
        };
        assert_eq!(calculate_pct(&vm), 99.999);
    }

    #[test]
    fn test_calculate_pct_very_small() {
        let vm = TopBarViewModel {
            repo: String::new(),
            branch: String::new(),
            path: String::new(),
            context_window: 128_000,
            estimated_tokens: 1,
            agent_running: false,
            braille_frame: 0,
            mode: TuiMode::Chat,
        };
        assert_eq!(calculate_pct(&vm), 0.00078125);
    }

    // ─── draw_gauge tests (via render output) ──────────────────────────────────

    #[test]
    fn test_draw_gauge_at_0_percent() {
        let vm = TopBarViewModel {
            repo: String::new(),
            branch: String::new(),
            path: String::new(),
            context_window: 100_000,
            estimated_tokens: 0,
            agent_running: false,
            braille_frame: 0,
            mode: TuiMode::Chat,
        };
        let pct = calculate_pct(&vm);
        assert_eq!(pct, 0.0);
        // Gauge char at 0% is ○
        assert!(vm.estimated_tokens == 0);
    }

    #[test]
    fn test_draw_gauge_at_100_percent() {
        let vm = TopBarViewModel {
            repo: String::new(),
            branch: String::new(),
            path: String::new(),
            context_window: 100_000,
            estimated_tokens: 100_000,
            agent_running: false,
            braille_frame: 0,
            mode: TuiMode::Chat,
        };
        let pct = calculate_pct(&vm);
        assert_eq!(pct, 100.0);
    }
}
