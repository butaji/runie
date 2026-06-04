use std::fmt::Write;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::Line,
};

use crate::components::message_list::feed::FeedItem;
use crate::components::message_list::MessageItem;
use crate::components::message_list::PlanStatus;

use super::messages::{
    format_transfer_bytes, render_edit_msg, render_empty_state, render_error_msg,
    render_interrupt_msg, render_plan_step_msg, render_rewind_msg,
    render_separator, render_system_msg, render_thought_msg, render_tool_complete_msg,
    render_tool_running_msg,
};

// ============================================================================
// render_tool_running_msg tests
// ============================================================================

#[test]
fn test_tool_running_shows_left_content() {
    let area = Rect::new(0, 0, 80, 10);
    let mut buf = Buffer::empty(area);

    render_tool_running_msg(
        "Read",
        "{}",
        100,
        200,
        0,
        area,
        0,
        0,
        0,
        &mut buf,
        Color::Gray,
        '⠴',
        true,
    );

    // Check that left content contains "Read" with spinner and duration
    // The left content format is: "{spinner} {name}… {duration}s"
    let content = buf
        .content
        .iter()
        .map(|c| c.symbol())
        .collect::<String>();
    assert!(
        content.contains("Read"),
        "Expected 'Read' in content, got: {}",
        content
    );
    // Should have spinner char
    assert!(
        content.contains("⠴"),
        "Expected spinner in content"
    );
    // Should have duration
    assert!(
        content.contains("0.1s"),
        "Expected '0.1s' duration in content"
    );
}

#[test]
fn test_tool_running_shows_right_content() {
    let area = Rect::new(0, 0, 80, 10);
    let mut buf = Buffer::empty(area);

    render_tool_running_msg(
        "List",
        ".",
        100,
        5700, // 5.7s total elapsed
        21200, // 21.2k bytes
        area,
        0,
        0,
        0,
        &mut buf,
        Color::Gray,
        '⠴',
        true,
    );

    let content = buf
        .content
        .iter()
        .map(|c| c.symbol())
        .collect::<String>();

    // Right side format: " {total_elapsed}s ⇣{bytes} [ ]"
    assert!(
        content.contains("5.7s"),
        "Expected total elapsed '5.7s' in content"
    );
    assert!(
        content.contains("21.2k"),
        "Expected bytes '21.2k' in content"
    );
    assert!(
        content.contains("[ ]"),
        "Expected empty status brackets in content"
    );
}

#[test]
fn test_tool_args_cleaned_of_json_quotes() {
    let area = Rect::new(0, 0, 80, 10);
    let mut buf = Buffer::empty(area);

    // Pass JSON-encoded string with quotes: "\".\""
    render_tool_running_msg(
        "List",
        "\".\"",
        100,
        200,
        0,
        area,
        0,
        0,
        0,
        &mut buf,
        Color::Gray,
        '⠴',
        true,
    );

    let content = buf
        .content
        .iter()
        .map(|c| c.symbol())
        .collect::<String>();

    // The args are cleaned: args[1..len-1] removes outer quotes
    // So "\".\"" becomes "." not "\"\""
    // We should NOT see triple quotes in the output
    assert!(
        !content.contains("\"\"\""),
        "Should not have triple quotes in cleaned output, got: {}",
        content
    );
    // Should show clean "."
    assert!(
        content.contains("."),
        "Should contain cleaned args '.', got: {}",
        content
    );
}

#[test]
fn test_tool_args_not_cleaned_when_not_json() {
    let area = Rect::new(0, 0, 80, 10);
    let mut buf = Buffer::empty(area);

    // Plain args (not JSON quoted)
    render_tool_running_msg(
        "Read",
        "/path/to/file.txt",
        100,
        200,
        0,
        area,
        0,
        0,
        0,
        &mut buf,
        Color::Gray,
        '⠴',
        true,
    );

    let content = buf
        .content
        .iter()
        .map(|c| c.symbol())
        .collect::<String>();

    // Should preserve the path as-is
    assert!(
        content.contains("/path/to/file.txt"),
        "Should preserve plain args, got: {}",
        content
    );
}

// ============================================================================
// render_tool_complete_msg tests
// ============================================================================

#[test]
fn test_tool_complete_shows_checkmark() {
    let area = Rect::new(0, 0, 80, 10);
    let mut buf = Buffer::empty(area);

    render_tool_complete_msg(
        "Read",
        "file content here",
        None,
        area,
        0,
        0,
        0,
        &mut buf,
        Color::Green,
        Color::Gray,
    );

    let content = buf
        .content
        .iter()
        .map(|c| c.symbol())
        .collect::<String>();

    // Should show checkmark (✓) for success
    assert!(
        content.contains('✓'),
        "Expected checkmark in success output, got: {}",
        content
    );
    // Should show name
    assert!(
        content.contains("Read"),
        "Expected tool name 'Read' in output, got: {}",
        content
    );
}

#[test]
fn test_tool_complete_error_shows_x() {
    let area = Rect::new(0, 0, 80, 10);
    let mut buf = Buffer::empty(area);

    render_tool_complete_msg(
        "Read",
        "Error: file not found",
        None,
        area,
        0,
        0,
        0,
        &mut buf,
        Color::Green,
        Color::Gray,
    );

    let content = buf
        .content
        .iter()
        .map(|c| c.symbol())
        .collect::<String>();

    // Should show ✗ for error
    assert!(
        content.contains('✗'),
        "Expected ✗ for error output, got: {}",
        content
    );
}

#[test]
fn test_tool_complete_shows_result_preview() {
    let area = Rect::new(0, 0, 80, 10);
    let mut buf = Buffer::empty(area);

    render_tool_complete_msg(
        "List",
        "item1\nitem2\nitem3",
        Some(&3),
        area,
        0,
        0,
        0,
        &mut buf,
        Color::Green,
        Color::Gray,
    );

    let content = buf
        .content
        .iter()
        .map(|c| c.symbol())
        .collect::<String>();

    // Should show checkmark and name
    assert!(
        content.contains('✓'),
        "Expected checkmark in output"
    );
    assert!(
        content.contains("List"),
        "Expected tool name in output"
    );
    // Should show line count on second line
    assert!(
        content.contains("(3 lines)"),
        "Expected line count in output, got: {}",
        content
    );
}

// ============================================================================
// FeedItem::ToolRunning conversion tests
// ============================================================================

#[test]
fn test_feed_item_tool_running_conversion() {
    let msg = MessageItem::ToolRunning {
        name: "List".to_string(),
        args: ".".to_string(),
        duration_ms: 100,
        total_elapsed_ms: 200,
        download_bytes: 0,
    };

    let feed_item: FeedItem = msg.try_into().unwrap();

    match feed_item {
        FeedItem::ToolRunning {
            name,
            args,
            duration_ms,
            total_elapsed_ms,
            download_bytes,
        } => {
            assert_eq!(name, "List");
            assert_eq!(args, ".");
            assert_eq!(duration_ms, 100);
            assert_eq!(total_elapsed_ms, 200);
            assert_eq!(download_bytes, 0);
        }
        _ => panic!("Expected FeedItem::ToolRunning, got something else"),
    }
}

#[test]
fn test_feed_item_tool_complete_conversion() {
    let msg = MessageItem::ToolComplete {
        name: "Read".to_string(),
        result: "file contents".to_string(),
        lines: Some(10),
    };

    let feed_item: FeedItem = msg.try_into().unwrap();

    match feed_item {
        FeedItem::ToolComplete {
            name,
            result,
            lines,
        } => {
            assert_eq!(name, "Read");
            assert_eq!(result, "file contents");
            assert_eq!(lines, Some(10));
        }
        _ => panic!("Expected FeedItem::ToolComplete, got something else"),
    }
}

// ============================================================================
// format_transfer_bytes tests
// ============================================================================

#[test]
fn test_format_transfer_bytes_bytes() {
    assert_eq!(format_transfer_bytes(0), "0");
    assert_eq!(format_transfer_bytes(500), "500");
    assert_eq!(format_transfer_bytes(999), "999");
}

#[test]
fn test_format_transfer_bytes_kilobytes() {
    assert_eq!(format_transfer_bytes(1_000), "1.0k");
    assert_eq!(format_transfer_bytes(1_500), "1.5k");
    assert_eq!(format_transfer_bytes(10_000), "10.0k");
    assert_eq!(format_transfer_bytes(999_999), "1000.0k");
}

#[test]
fn test_format_transfer_bytes_megabytes() {
    assert_eq!(format_transfer_bytes(1_000_000), "1.0M");
    assert_eq!(format_transfer_bytes(2_500_000), "2.5M");
    assert_eq!(format_transfer_bytes(10_000_000), "10.0M");
}

// ============================================================================
// Other render function tests (basic sanity)
// ============================================================================

#[test]
fn test_render_thought_msg() {
    let area = Rect::new(0, 0, 80, 10);
    let mut buf = Buffer::empty(area);

    let rows = render_thought_msg(
        1.5,
        area,
        0,
        0,
        0,
        &mut buf,
        Color::Gray,
        '⠴',
        true,
    );

    assert_eq!(rows, 1);
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();
    assert!(content.contains("◆"));
    assert!(content.contains("1.5s"));
}

#[test]
fn test_render_system_msg() {
    let area = Rect::new(0, 0, 80, 10);
    let mut buf = Buffer::empty(area);
    let mut wrap_cache = crate::components::message_list::WrapCache::default();

    let rows = render_system_msg(
        "System message",
        area,
        0,
        0,
        0,
        &mut buf,
        Color::Gray,
        Color::Red,
        &mut wrap_cache,
    );

    assert_eq!(rows, 1);
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();
    assert!(content.contains("◆"));
    assert!(content.contains("System message"));
}

#[test]
fn test_render_system_msg_error() {
    let area = Rect::new(0, 0, 80, 10);
    let mut buf = Buffer::empty(area);
    let mut wrap_cache = crate::components::message_list::WrapCache::default();

    let rows = render_system_msg(
        "Error: something went wrong",
        area,
        0,
        0,
        0,
        &mut buf,
        Color::Gray,
        Color::Red,
        &mut wrap_cache,
    );

    assert_eq!(rows, 1);
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();
    assert!(content.contains("!"));
}

#[test]
fn test_render_error_msg() {
    let area = Rect::new(0, 0, 80, 10);
    let mut buf = Buffer::empty(area);
    let mut wrap_cache = crate::components::message_list::WrapCache::default();

    let rows = render_error_msg(
        "Something went wrong",
        false,
        area,
        0,
        0,
        0,
        &mut buf,
        Color::Red,
        Color::Gray,
        &mut wrap_cache,
    );

    assert_eq!(rows, 1);
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();
    assert!(content.contains("!"));
    assert!(content.contains("Something went wrong"));
}

#[test]
fn test_render_edit_msg() {
    let area = Rect::new(0, 0, 80, 10);
    let mut buf = Buffer::empty(area);

    let rows = render_edit_msg(
        "src/main.rs",
        "",
        area,
        0,
        0,
        0,
        &mut buf,
        Color::Blue,
        Color::Cyan,
    );

    assert_eq!(rows, 1);
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();
    assert!(content.contains("◆"));
    assert!(content.contains("Edit"));
    assert!(content.contains("src/main.rs"));
}

#[test]
fn test_render_plan_step_pending() {
    let area = Rect::new(0, 0, 80, 10);
    let mut buf = Buffer::empty(area);

    let rows = render_plan_step_msg(
        1,
        "Do something",
        &PlanStatus::Pending,
        area,
        0,
        0,
        0,
        &mut buf,
        Color::Gray,
        Color::Gray,
        '⠴',
        true,
    );

    assert_eq!(rows, 1);
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();
    assert!(content.contains("Do something"));
}

#[test]
fn test_render_plan_step_complete() {
    let area = Rect::new(0, 0, 80, 10);
    let mut buf = Buffer::empty(area);

    let rows = render_plan_step_msg(
        1,
        "Do something",
        &PlanStatus::Complete,
        area,
        0,
        0,
        0,
        &mut buf,
        Color::Gray,
        Color::Green,
        '⠴',
        true,
    );

    assert_eq!(rows, 1);
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();
    assert!(content.contains('✓'));
}

#[test]
fn test_render_separator() {
    let area = Rect::new(0, 0, 80, 10);
    let mut buf = Buffer::empty(area);

    let rows = render_separator(
        65, // 1m 5s
        3,
        Some(1500),
        true,
        area,
        0,
        0,
        &mut buf,
        Color::Gray,
    );

    assert_eq!(rows, 1);
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();
    assert!(content.contains("1m 05s") || content.contains("65s"));
    assert!(content.contains("[✓]"));
}

#[test]
fn test_render_interrupt_msg() {
    let area = Rect::new(0, 0, 80, 10);
    let mut buf = Buffer::empty(area);
    let animation = crate::tui::state::AnimationState::default();

    let rows = render_interrupt_msg(
        area,
        0,
        0,
        0,
        &mut buf,
        Color::Red,
        Color::Gray,
        &animation,
    );

    assert_eq!(rows, 1);
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();
    assert!(content.contains("Interrupted"));
}

#[test]
fn test_render_rewind_msg() {
    let area = Rect::new(0, 0, 80, 10);
    let mut buf = Buffer::empty(area);

    let rows = render_rewind_msg(
        3,
        area,
        0,
        0,
        0,
        &mut buf,
        Color::Gray,
        '⠴',
        true,
    );

    assert_eq!(rows, 1);
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();
    assert!(content.contains("Rewinding"));
    assert!(content.contains("3 steps"));
}
