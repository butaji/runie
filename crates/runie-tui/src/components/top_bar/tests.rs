//! Top bar render tests.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
};
use crate::components::top_bar::{TopBarViewModel, helpers::build_left_spans, render_top_bar};
use crate::theme::ThemeColors;

fn make_test_colors() -> ThemeColors {
    ThemeColors {
        bg_base: Color::Black,
        bg_panel: Color::Black,
        accent_primary: Color::White,
        accent_secondary: Color::Blue,
        text_primary: Color::White,
        text_secondary: Color::White,
        text_dim: Color::Gray,
        text_muted: Color::DarkGray,
        border_unfocused: Color::DarkGray,
        success: Color::Green,
        error: Color::Red,
        syntax_phase: Color::Yellow,
        text_plan: Color::Cyan,
        feed_tool_bar: Color::Magenta,
        accent_user: Color::Green,
        accent_assistant: Color::Blue,
        accent_thinking: Color::Yellow,
        accent_tool: Color::Cyan,
        accent_system: Color::DarkGray,
        accent_error: Color::Red,
        accent_success: Color::Green,
        accent_running: Color::Yellow,
        accent_skill: Color::Magenta,
        accent_plan: Color::Cyan,
        accent_feedback: Color::Red,
        accent_model: Color::Blue,
        accent_teal: Color::Cyan,
        accent_orange: Color::Red,
        accent_purple: Color::Magenta,
        accent_yellow: Color::Yellow,
        accent_blue_bright: Color::Blue,
        command: Color::Green,
        path: Color::Blue,
        running: Color::Yellow,
        fuzzy_accent: Color::Green,
        editor_bg: Color::Black,
        surface_bg: Color::Black,
        popover_bg: Color::DarkGray,
    }
}

// ─── build_left_spans tests ────────────────────────────────────────────────

#[test]
fn test_build_left_spans_repo_only() {
    let vm = TopBarViewModel {
        repo: "runie".to_string(),
        branch: String::new(),
        path: String::new(),
        context_window: 128_000,
        estimated_tokens: 0,
        agent_running: false,
        braille_frame: 0,
    };
    let dim_style = Style::default();
    let spans = build_left_spans(&vm, Color::White, Color::White, &dim_style, Color::Black);
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].content.as_ref(), "runie");
}

#[test]
fn test_build_left_spans_branch_only() {
    let vm = TopBarViewModel {
        repo: String::new(),
        branch: "main".to_string(),
        path: String::new(),
        context_window: 128_000,
        estimated_tokens: 0,
        agent_running: false,
        braille_frame: 0,
    };
    let dim_style = Style::default();
    let spans = build_left_spans(&vm, Color::White, Color::White, &dim_style, Color::Black);
    assert_eq!(spans.len(), 2);
    assert_eq!(spans[0].content.as_ref(), "\u{E0A0}");
    assert_eq!(spans[1].content.as_ref(), "main");
}

#[test]
fn test_build_left_spans_path_only() {
    let vm = TopBarViewModel {
        repo: String::new(),
        branch: String::new(),
        path: "src/lib.rs".to_string(),
        context_window: 128_000,
        estimated_tokens: 0,
        agent_running: false,
        braille_frame: 0,
    };
    let dim_style = Style::default();
    let spans = build_left_spans(&vm, Color::White, Color::White, &dim_style, Color::Black);
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].content.as_ref(), "  src/lib.rs");
}

// ─── render_top_bar tests ───────────────────────────────────────────────────

#[test]
fn test_render_top_bar_text_appears() {
    let vm = TopBarViewModel {
        repo: "myrepo".to_string(),
        branch: "feature".to_string(),
        path: "src".to_string(),
        context_window: 120_000,
        estimated_tokens: 40,
        agent_running: false,
        braille_frame: 0,
    };

    let colors = make_test_colors();
    let area = Rect::new(0, 0, 80, 1);
    let mut buf = Buffer::empty(area);

    render_top_bar(&vm, area, &mut buf, &colors);

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
        context_window: 128_000,
        estimated_tokens: 0,
        agent_running: false,
        braille_frame: 0,
    };

    let colors = make_test_colors();
    let area = Rect::new(0, 0, 80, 1);
    let mut buf = Buffer::empty(area);

    render_top_bar(&vm, area, &mut buf, &colors);

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
    let vm = TopBarViewModel {
        repo: String::new(),
        branch: String::new(),
        path: String::new(),
        context_window: 100_000,
        estimated_tokens: 50_000,
        agent_running: false,
        braille_frame: 0,
    };

    let colors = make_test_colors();
    let area = Rect::new(0, 0, 80, 1);
    let mut buf = Buffer::empty(area);

    render_top_bar(&vm, area, &mut buf, &colors);

    let content = buf.content();
    let has_percent = content.iter().any(|c| c.symbol() == "%");
    assert!(has_percent, "Expected '%' in buffer");
}

#[test]
fn test_render_top_bar_empty_vm() {
    let vm = TopBarViewModel {
        repo: String::new(),
        branch: String::new(),
        path: String::new(),
        context_window: 128_000,
        estimated_tokens: 0,
        agent_running: false,
        braille_frame: 0,
    };

    let colors = make_test_colors();
    let area = Rect::new(0, 0, 80, 1);
    let mut buf = Buffer::empty(area);

    render_top_bar(&vm, area, &mut buf, &colors);

    let content = buf.content();
    let has_0 = content.iter().any(|c| c.symbol().contains("0"));
    let has_bolt = content.iter().any(|c| c.symbol() == "○");
    assert!(has_0, "Expected '0' in buffer for zero tokens");
    assert!(has_bolt, "Expected gauge character in buffer");
}

#[test]
fn test_render_top_bar_narrow_terminal() {
    let vm = TopBarViewModel {
        repo: String::new(),
        branch: String::new(),
        path: String::new(),
        context_window: 128_000,
        estimated_tokens: 0,
        agent_running: false,
        braille_frame: 0,
    };

    let colors = make_test_colors();
    let area = Rect::new(0, 0, 18, 1);
    let mut buf = Buffer::empty(area);

    render_top_bar(&vm, area, &mut buf, &colors);

    let content = buf.content();
    let has_percent = content.iter().any(|c| c.symbol().contains("%"));
    assert!(has_percent, "Expected '%' in buffer");
}

#[test]
fn test_render_top_bar_gauge_at_0_percent() {
    let vm = TopBarViewModel {
        repo: String::new(),
        branch: String::new(),
        path: String::new(),
        context_window: 100_000,
        estimated_tokens: 0,
        agent_running: false,
        braille_frame: 0,
    };

    let colors = make_test_colors();
    let area = Rect::new(0, 0, 80, 1);
    let mut buf = Buffer::empty(area);

    render_top_bar(&vm, area, &mut buf, &colors);

    let content = buf.content();
    let has_gauge = content.iter().any(|c| c.symbol() == "○");
    assert!(has_gauge, "Expected gauge character in buffer for 0% gauge");
}

#[test]
fn test_render_top_bar_gauge_at_100_percent() {
    let vm = TopBarViewModel {
        repo: String::new(),
        branch: String::new(),
        path: String::new(),
        context_window: 100_000,
        estimated_tokens: 100_000,
        agent_running: false,
        braille_frame: 0,
    };

    let colors = make_test_colors();
    let area = Rect::new(0, 0, 80, 1);
    let mut buf = Buffer::empty(area);

    render_top_bar(&vm, area, &mut buf, &colors);

    let content = buf.content();
    let has_gauge = content.iter().any(|c| c.symbol() == "■");
    assert!(has_gauge, "Expected gauge character in buffer for 100% gauge");
}

#[test]
fn test_no_duplicate_percentage() {
    let vm = TopBarViewModel {
        repo: String::new(),
        branch: String::new(),
        path: String::new(),
        context_window: 100_000,
        estimated_tokens: 50_000,
        agent_running: false,
        braille_frame: 0,
    };

    let colors = make_test_colors();
    let area = Rect::new(0, 0, 80, 1);
    let mut buf = Buffer::empty(area);

    render_top_bar(&vm, area, &mut buf, &colors);

    let content = buf.content();
    let percent_count = content.iter().filter(|c| c.symbol().contains("%")).count();
    assert_eq!(percent_count, 1, "Expected exactly one '%' in buffer, found {}", percent_count);
}

#[test]
fn test_gauge_label_shows_pct() {
    let vm = TopBarViewModel {
        repo: String::new(),
        branch: String::new(),
        path: String::new(),
        context_window: 100_000,
        estimated_tokens: 50_000,
        agent_running: false,
        braille_frame: 0,
    };

    let colors = make_test_colors();
    let area = Rect::new(0, 0, 80, 1);
    let mut buf = Buffer::empty(area);

    render_top_bar(&vm, area, &mut buf, &colors);

    let content = buf.content();
    let has_5 = content.iter().any(|c| c.symbol() == "5");
    let has_percent = content.iter().any(|c| c.symbol() == "%");
    assert!(has_5, "Expected '5' in buffer");
    assert!(has_percent, "Expected '%' in buffer for 50%");
}

#[test]
fn test_gauge_width_sufficient() {
    let vm = TopBarViewModel {
        repo: String::new(),
        branch: String::new(),
        path: String::new(),
        context_window: 100_000,
        estimated_tokens: 50_000,
        agent_running: false,
        braille_frame: 0,
    };

    let colors = make_test_colors();
    let area = Rect::new(0, 0, 80, 1);
    let mut buf = Buffer::empty(area);

    render_top_bar(&vm, area, &mut buf, &colors);

    let content = buf.content();
    let has_percent = content.iter().any(|c| c.symbol() == "%");
    assert!(has_percent, "Text with percentage should appear");
}

#[test]
fn test_text_no_pct_suffix() {
    let vm = TopBarViewModel {
        repo: String::new(),
        branch: String::new(),
        path: String::new(),
        context_window: 100_000,
        estimated_tokens: 50_000,
        agent_running: false,
        braille_frame: 0,
    };

    let colors = make_test_colors();
    let area = Rect::new(0, 0, 80, 1);
    let mut buf = Buffer::empty(area);

    render_top_bar(&vm, area, &mut buf, &colors);

    let content = buf.content();
    let percent_count = content.iter().filter(|c| c.symbol().contains("%")).count();
    assert_eq!(percent_count, 1, "Text should contain percentage suffix");
}

#[test]
fn test_zero_percent_visible() {
    let vm = TopBarViewModel {
        repo: String::new(),
        branch: String::new(),
        path: String::new(),
        context_window: 100_000,
        estimated_tokens: 0,
        agent_running: false,
        braille_frame: 0,
    };

    let colors = make_test_colors();
    let area = Rect::new(0, 0, 80, 1);
    let mut buf = Buffer::empty(area);

    render_top_bar(&vm, area, &mut buf, &colors);

    let content = buf.content();
    let has_zero = content.iter().any(|c| c.symbol() == "0");
    let has_gauge = content.iter().any(|c| c.symbol() == "○");
    assert!(has_zero, "At 0%, should show '0' digit for tokens");
    assert!(has_gauge, "At 0%, gauge should show gauge character");
}

#[test]
fn test_gauge_visible_at_zero_percent() {
    let vm = TopBarViewModel {
        repo: String::new(),
        branch: String::new(),
        path: String::new(),
        context_window: 100_000,
        estimated_tokens: 0,
        agent_running: false,
        braille_frame: 0,
    };

    let colors = make_test_colors();
    let area = Rect::new(0, 0, 80, 1);
    let mut buf = Buffer::empty(area);

    render_top_bar(&vm, area, &mut buf, &colors);

    let content = buf.content();
    let has_gauge = content.iter().any(|c| c.symbol() == "○");
    assert!(has_gauge, "Gauge area should have gauge character at 0%");
}
