//! Feed styling contract — grok CLI-derived look (dark theme, SGR-verified).
//!
//! Pins the exact look of feed elements per the verified grok 0.2.87 dark-mode
//! captures: user card (chevron, band, margins, text), thought/tool posts,
//! assistant text, the turn-completed line, timestamps, 3-column content
//! indent, and the (unchanged) orange selection gutter. Deviations from grok
//! by explicit request: the feed chevron uses the accent color (matching the
//! input-box chevron) and content sits 2 columns further left.

use super::*;
use crate::terminal::caps::{MouseCapability, TermCaps};
use ratatui::backend::TestBackend;
use ratatui::style::{Color, Modifier};
use ratatui::Terminal;
use runie_core::labels::format_timestamp;

const FEED_DIM: Color = Color::Rgb(108, 108, 108);
const USER_BG: Color = Color::Rgb(36, 36, 36);
const USER_FG: Color = Color::Rgb(225, 225, 225);
const AGENT_FG: Color = Color::Rgb(200, 200, 200);
const ACCENT: Color = Color::Rgb(238, 105, 2);

/// Pin the dark runie theme at truecolor so Rgb assertions are exact.
/// Serializes with other theme-mutating tests via the global test lock.
fn dark_theme() -> std::sync::MutexGuard<'static, ()> {
    let guard = crate::theme::test_lock();
    crate::theme::set_current_theme_with_caps(
        "runie",
        TermCaps {
            truecolor: true,
            mouse: MouseCapability::Sgr,
            ..Default::default()
        },
    );
    guard
}

fn add_message(state: &mut AppState, role: Role, content: &str, timestamp: f64, id: &str) {
    state.session.messages.push(ChatMessage {
        role,
        parts: vec![Part::Text {
            content: content.to_string(),
        }],
        timestamp,
        id: id.to_string(),
        ..Default::default()
    });
    state.refresh_after_message_change();
}

fn draw(state: &mut AppState, width: u16, height: u16) -> ratatui::buffer::Buffer {
    state.set_last_content_width(width.saturating_sub(2));
    state.set_last_visible_height(height.saturating_sub(8).max(3));
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, state)).unwrap();
    terminal.backend().buffer().clone()
}

fn row_text(buf: &ratatui::buffer::Buffer, y: u16) -> String {
    (0..buf.area().width)
        .map(|x| buf[(x, y)].symbol())
        .collect()
}

fn find_row(buf: &ratatui::buffer::Buffer, needle: &str) -> Option<u16> {
    (0..buf.area().height).find(|&y| row_text(buf, y).contains(needle))
}

fn find_col(buf: &ratatui::buffer::Buffer, y: u16, symbol: &str) -> Option<u16> {
    (0..buf.area().width).find(|&x| buf[(x, y)].symbol() == symbol)
}

/// Char-cell column of `needle` in `text` (str::find returns bytes, which
/// misaligns after multibyte glyphs like ◆).
fn col_of(text: &str, needle: &str) -> Option<u16> {
    text.find(needle)
        .map(|byte| text[..byte].chars().count() as u16)
}

fn buffer_text(buf: &ratatui::buffer::Buffer) -> String {
    buf.content().iter().map(|c| c.symbol()).collect()
}

fn is_bold(buf: &ratatui::buffer::Buffer, x: u16, y: u16) -> bool {
    buf[(x, y)].style().add_modifier.contains(Modifier::BOLD)
}

// ── User card ────────────────────────────────────────────────────────────────

#[test]
fn user_card_chevron_matches_input_box_accent_inside_band() {
    let _guard = dark_theme();
    let mut state = AppState::default();
    add_message(&mut state, Role::User, "hello", 0.0, "req.0");

    let buf = draw(&mut state, 60, 20);
    let row = find_row(&buf, "hello").expect("user row");
    let x = find_col(&buf, row, "❯").expect("chevron cell");
    let cell = &buf[(x, row)];
    assert_eq!(
        cell.style().fg,
        Some(ACCENT),
        "chevron must use the accent color (same as the input-box chevron), got {:?}",
        cell.style().fg
    );
    assert!(
        !is_bold(&buf, x, row),
        "chevron must be normal weight, not bold"
    );
    assert_eq!(
        cell.style().bg,
        Some(USER_BG),
        "chevron must sit inside the card band"
    );
    // Geometry: 1-col terminal margin + 2-space feed indent → chevron at col 3.
    assert_eq!(x, 3, "chevron must sit at column 3");
}

#[test]
fn user_card_band_spans_full_app_width() {
    let _guard = dark_theme();
    let mut state = AppState::default();
    add_message(&mut state, Role::User, "hello", 0.0, "req.0");

    let buf = draw(&mut state, 60, 20);
    let w = buf.area().width;
    let content_row = find_row(&buf, "hello").expect("user row");

    for y in [content_row - 1, content_row, content_row + 1] {
        for x in 0..w {
            assert_eq!(
                buf[(x, y)].style().bg,
                Some(USER_BG),
                "band col {x} row {y} must be Rgb(36,36,36) (full app width)"
            );
        }
    }
    // The margin line after the card stays on the feed background.
    assert_ne!(
        buf[(2, content_row + 2)].style().bg,
        Some(USER_BG),
        "margin line after the card must not be banded"
    );
}

#[test]
fn user_text_is_light_gray_normal_weight() {
    let _guard = dark_theme();
    let mut state = AppState::default();
    add_message(&mut state, Role::User, "hello", 0.0, "req.0");

    let buf = draw(&mut state, 60, 20);
    let row = find_row(&buf, "hello").expect("user row");
    let x = find_col(&buf, row, "h").expect("text cell");
    let cell = &buf[(x, row)];
    assert_eq!(
        cell.style().fg,
        Some(USER_FG),
        "user text must be Rgb(225,225,225), got {:?}",
        cell.style().fg
    );
    assert!(!is_bold(&buf, x, row), "user text must be normal weight");
    assert_eq!(cell.style().bg, Some(USER_BG), "user text sits in the band");
    // 1-col margin + 2-space feed indent + "❯ " → text at col 5.
    assert_eq!(x, 5, "user text must start at column 5");
}

#[test]
fn user_card_timestamp_is_dim() {
    let _guard = dark_theme();
    let mut state = AppState::default();
    add_message(&mut state, Role::User, "hello", 12345.0, "req.0");

    let buf = draw(&mut state, 60, 20);
    let row = find_row(&buf, "hello").expect("user row");
    let ts = format_timestamp(12345.0);
    let ts_col = col_of(&row_text(&buf, row), &ts).expect("timestamp on user row");
    let cell = &buf[(ts_col, row)];
    assert_eq!(
        cell.style().fg,
        Some(FEED_DIM),
        "timestamp must be Rgb(108,108,108), got {:?}",
        cell.style().fg
    );
}

// ── Thought posts ────────────────────────────────────────────────────────────

#[test]
fn thought_summary_is_dim_bold_thought_without_affordance() {
    let _guard = dark_theme();
    let mut state = AppState::default();
    add_message(
        &mut state,
        Role::Thought,
        "◆ Thought for 1.2s\ndeep reasoning body",
        0.0,
        "t1",
    );

    let buf = draw(&mut state, 60, 20);
    assert!(
        !buffer_text(&buf).contains("[+]"),
        "collapsed thought must not render the [+] affordance"
    );
    assert!(
        !buffer_text(&buf).contains("deep reasoning"),
        "collapsed thought must hide the reasoning body"
    );

    let row = find_row(&buf, "Thought for").expect("thought summary row");
    let diamond = find_col(&buf, row, "◆").expect("thought diamond");
    assert_eq!(diamond, 3, "thought glyph must sit at column 3");
    assert_eq!(
        buf[(diamond, row)].style().fg,
        Some(FEED_DIM),
        "thought glyph must be dim"
    );

    let t_col = col_of(&row_text(&buf, row), "Thought").expect("Thought word");
    assert!(
        is_bold(&buf, t_col, row),
        "the word 'Thought' must be bold"
    );
    assert_eq!(
        buf[(t_col, row)].style().fg,
        Some(FEED_DIM),
        "'Thought' must be dim"
    );
    let for_col = col_of(&row_text(&buf, row), "for 1.2s").expect("duration");
    assert_eq!(
        buf[(for_col, row)].style().fg,
        Some(FEED_DIM),
        "duration must be dim"
    );
    assert!(
        !is_bold(&buf, for_col, row),
        "' for Xs' must be normal weight"
    );
}

#[test]
fn expanded_thought_body_is_dim() {
    let _guard = dark_theme();
    let mut state = AppState::default();
    add_message(
        &mut state,
        Role::Thought,
        "◆ Thought for 1.2s\ndeep reasoning body",
        0.0,
        "t1",
    );
    // Expand the thought post individually (Enter in feed nav equivalent).
    state.view.expanded_posts.insert(0);
    state.messages_changed();

    let buf = draw(&mut state, 60, 20);
    let row = find_row(&buf, "deep reasoning").expect("expanded reasoning row");
    let x = find_col(&buf, row, "d").expect("reasoning text");
    assert_eq!(
        buf[(x, row)].style().fg,
        Some(FEED_DIM),
        "expanded reasoning text must be dim Rgb(108,108,108)"
    );
}

// ── Tool posts ───────────────────────────────────────────────────────────────

#[test]
fn tool_done_post_is_dim_diamond_bold_name_no_duration() {
    let _guard = dark_theme();
    let mut state = AppState::default();
    add_message(
        &mut state,
        Role::Tool,
        "✓ list_files 0.5s\nfile1\nfile2",
        0.0,
        "x1",
    );

    let buf = draw(&mut state, 60, 20);
    let row = find_row(&buf, "Run list_files").expect("tool post row");
    let text = row_text(&buf, row);
    assert!(
        !text.contains('✓'),
        "tool post must not render the ✓ glyph: {text:?}"
    );
    assert!(
        !text.contains("0.5s"),
        "done tool post must not render a duration: {text:?}"
    );

    let diamond = find_col(&buf, row, "◆").expect("tool diamond");
    assert_eq!(diamond, 3, "tool glyph must sit at column 3");
    assert_eq!(
        buf[(diamond, row)].style().fg,
        Some(FEED_DIM),
        "tool glyph must be dim"
    );

    let name_col = col_of(&text, "Run list_files").expect("tool name");
    for x in name_col..name_col + "Run list_files".len() as u16 {
        assert!(
            is_bold(&buf, x, row),
            "tool verb/name must be bold at col {x}"
        );
        assert_eq!(
            buf[(x, row)].style().fg,
            Some(FEED_DIM),
            "tool name must be dim at col {x}"
        );
    }
}

#[test]
fn tool_output_lines_are_dim_and_indented() {
    let _guard = dark_theme();
    let mut state = AppState::default();
    add_message(
        &mut state,
        Role::Tool,
        "✓ list_files 0.5s\nfile1\nfile2",
        0.0,
        "x1",
    );

    let buf = draw(&mut state, 60, 20);
    let row = find_row(&buf, "file1").expect("tool output row");
    let x = find_col(&buf, row, "f").expect("output text");
    assert_eq!(x, 3, "tool output must align under the post at column 3");
    assert_eq!(
        buf[(x, row)].style().fg,
        Some(FEED_DIM),
        "tool output must be dim Rgb(108,108,108)"
    );
}

// ── Assistant text ───────────────────────────────────────────────────────────

#[test]
fn assistant_text_has_no_glyph_feed_indent_and_neutral_gray() {
    let _guard = dark_theme();
    let mut state = AppState::default();
    let mut msg = ChatMessage {
        role: Role::Assistant,
        parts: vec![Part::Text {
            content: "Hello **world**".into(),
        }],
        timestamp: 1.0,
        id: "resp.0".into(),
        ..Default::default()
    };
    msg.provider = "mock".into();
    state.session.messages.push(msg);
    state.refresh_after_message_change();

    let buf = draw(&mut state, 60, 20);
    let row = find_row(&buf, "Hello").expect("assistant row");
    let text = row_text(&buf, row);
    assert!(
        !text.contains('◆'),
        "plain answer lines must not render the ◆ prefix: {text:?}"
    );
    let x = find_col(&buf, row, "H").expect("answer text");
    assert_eq!(x, 3, "assistant text must start at column 3");
    assert_eq!(
        buf[(x, row)].style().fg,
        Some(AGENT_FG),
        "assistant text must be Rgb(200,200,200), got {:?}",
        buf[(x, row)].style().fg
    );
    // Markdown bold inline is preserved on top of the base color.
    let bold_col = col_of(&text, "world").expect("bold inline");
    assert!(
        is_bold(&buf, bold_col, row),
        "markdown bold inline must be preserved"
    );
    assert_eq!(
        buf[(bold_col, row)].style().fg,
        Some(AGENT_FG),
        "bold inline keeps the answer color"
    );
}

// ── Turn completed ───────────────────────────────────────────────────────────

#[test]
fn turn_completed_line_is_dim_with_trailing_period() {
    let _guard = dark_theme();
    let mut state = AppState::default();
    add_message(
        &mut state,
        Role::TurnComplete,
        "Turn completed in 1.0s",
        0.0,
        "tc1",
    );

    let buf = draw(&mut state, 60, 20);
    let row = find_row(&buf, "Turn completed").expect("turn completed row");
    let text = row_text(&buf, row);
    assert!(
        text.contains("Turn completed in 1.0s."),
        "turn line must end with a period: {text:?}"
    );
    let x = find_col(&buf, row, "T").expect("turn text");
    assert_eq!(x, 3, "turn line must start at column 3");
    assert_eq!(
        buf[(x, row)].style().fg,
        Some(FEED_DIM),
        "turn line must be dim Rgb(108,108,108)"
    );
}

// ── Light theme ──────────────────────────────────────────────────────────────

/// Pin a light builtin theme (catppuccin-latte) at truecolor so Rgb
/// assertions are exact. Serializes via the global theme test lock.
fn light_theme() -> std::sync::MutexGuard<'static, ()> {
    let guard = crate::theme::test_lock();
    crate::theme::set_current_theme_with_caps(
        "catppuccin-latte",
        TermCaps {
            truecolor: true,
            mouse: MouseCapability::Sgr,
            ..Default::default()
        },
    );
    guard
}

#[test]
fn light_theme_user_card_uses_derived_band_and_readable_text() {
    let _guard = light_theme();
    let mut state = AppState::default();
    add_message(&mut state, Role::User, "hello", 0.0, "req.0");

    let buf = draw(&mut state, 60, 20);
    let row = find_row(&buf, "hello").expect("user row");
    let x = find_col(&buf, row, "❯").expect("chevron cell");
    let cell = &buf[(x, row)];

    // Card band: derived from the light base (#eff1f5 darkened 0.06) — never
    // the opaline missing-token fallback gray (128,128,128) and never Reset.
    assert_eq!(
        cell.style().bg,
        Some(Color::Rgb(225, 227, 230)),
        "light-theme card band must be the derived shade Rgb(225,227,230), got {:?}",
        cell.style().bg
    );
    // Chevron matches the input-box chevron: the theme accent (latte mauve).
    assert_eq!(
        cell.style().fg,
        Some(Color::Rgb(136, 57, 239)),
        "light-theme chevron must be the latte accent #8839ef, got {:?}",
        cell.style().fg
    );
    // User text keeps full contrast on the light card (unlightened primary).
    let tx = find_col(&buf, row, "h").expect("text cell");
    assert_eq!(
        buf[(tx, row)].style().fg,
        Some(Color::Rgb(76, 79, 105)),
        "light-theme user text must be latte text.primary #4c4f69, got {:?}",
        buf[(tx, row)].style().fg
    );
}

// ── Selection gutter (unchanged) ─────────────────────────────────────────────

#[test]
fn selection_gutter_stays_orange() {
    let _guard = dark_theme();
    let mut state = AppState::default();
    state.config.vim_mode = true;
    add_message(&mut state, Role::User, "hello", 0.0, "req.0");
    state.update(Event::DialogBack); // enter feed navigation

    let buf = draw(&mut state, 60, 20);
    let row = find_row(&buf, "hello").expect("user row");
    let cell = &buf[(0, row)];
    assert_eq!(cell.symbol(), "▎", "selection gutter glyph unchanged");
    assert_eq!(
        cell.style().fg,
        Some(ACCENT),
        "selection gutter stays orange Rgb(238,105,2)"
    );
}
