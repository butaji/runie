use ratatui::{buffer::Buffer, layout::Rect, style::Color};
use insta::assert_debug_snapshot;

use crate::components::top_bar::{render_top_bar, TopBarViewModel};
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
    }
}

fn render_to_string(vm: &TopBarViewModel, width: u16) -> String {
    let colors = make_test_colors();
    let area = Rect::new(0, 0, width, 1);
    let mut buf = Buffer::empty(area);
    render_top_bar(vm, area, &mut buf, &colors);

    // Extract symbols from buffer as a simple string representation
    buf.content()
        .iter()
        .map(|c| c.symbol().to_string())
        .collect::<String>()
        .trim_end()
        .to_string()
}

#[test]
fn test_top_bar_snapshot_empty() {
    let vm = TopBarViewModel {
        repo: String::new(),
        branch: String::new(),
        path: String::new(),
        context_window: 128_000,
        estimated_tokens: 0,
        agent_running: false,
        braille_frame: 0,
    };

    let rendered = render_to_string(&vm, 80);
    assert_debug_snapshot!("top_bar_empty", rendered);
}

#[test]
fn test_top_bar_snapshot_with_repo() {
    let vm = TopBarViewModel {
        repo: "runie".to_string(),
        branch: "main".to_string(),
        path: String::new(),
        context_window: 128_000,
        estimated_tokens: 0,
        agent_running: false,
        braille_frame: 0,
    };

    let rendered = render_to_string(&vm, 80);
    assert_debug_snapshot!("top_bar_with_repo", rendered);
}

#[test]
fn test_top_bar_snapshot_full() {
    let vm = TopBarViewModel {
        repo: "runie".to_string(),
        branch: "feature/new-ui".to_string(),
        path: "src/main.rs".to_string(),

        context_window: 200_000,
        estimated_tokens: 50_000,
        agent_running: false,
        braille_frame: 0,
    };

    let rendered = render_to_string(&vm, 80);
    assert_debug_snapshot!("top_bar_full", rendered);
}

#[test]
fn test_top_bar_snapshot_narrow() {
    let vm = TopBarViewModel {
        repo: "runie".to_string(),
        branch: "main".to_string(),
        path: "src".to_string(),
        context_window: 128_000,
        estimated_tokens: 40,
        agent_running: false,
        braille_frame: 0,
    };

    // Narrow terminal width - right side may be omitted
    let rendered = render_to_string(&vm, 20);
    assert_debug_snapshot!("top_bar_narrow", rendered);
}

#[test]
fn test_top_bar_snapshot_high_tokens() {
    let vm = TopBarViewModel {
        repo: "myproject".to_string(),
        branch: "develop".to_string(),
        path: "crates/core/src/lib.rs".to_string(),

        context_window: 100_000,
        estimated_tokens: 95_000, // Near 100%
        agent_running: false,
        braille_frame: 0,
    };

    let rendered = render_to_string(&vm, 80);
    assert_debug_snapshot!("top_bar_high_tokens", rendered);
}
