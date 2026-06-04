//! Snapshot tests for Feed rendering via MessageList component.

use ratatui::{buffer::Buffer, layout::Rect};
use insta::assert_snapshot;

use crate::components::message_list::feed::Feed;
use crate::components::message_list::render::WrapCache;
use crate::components::message_list::MessageListViewModel;
use crate::components::message_list::MessageList;
use crate::tui::state::AnimationState;
use crate::theme::ThemeWrapper;

fn make_test_theme() -> ThemeWrapper {
    ThemeWrapper::default_for_test()
}

fn buffer_to_string(buf: &Buffer) -> String {
    let mut result = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            if let Some(cell) = buf.cell((x, y)) {
                result.push_str(cell.symbol());
            }
        }
        if y < buf.area.height - 1 {
            result.push('\n');
        }
    }
    result
}

fn render_feed(feed: Feed, width: u16, height: u16) -> String {
    render_feed_with_agent(feed, width, height, false)
}

fn render_feed_with_agent(feed: Feed, width: u16, height: u16, agent_running: bool) -> String {
    let area = Rect::new(0, 0, width, height);
    let buf = Buffer::empty(area);
    let theme = make_test_theme();
    let vm = MessageListViewModel::new(
        feed,
        0,
        agent_running,
        AnimationState::default(),
        WrapCache::new(),
        None,
        None,
    );
    let mut buf = buf;
    MessageList::render_ref(&vm, area, &mut buf, &theme);
    buffer_to_string(&buf)
}

// ============================================================================
// Basic Message Snapshots
// ============================================================================

#[test]
fn test_user_message_snapshot() {
    let feed = Feed::builder()
        .user_message("Hey!")
        .build();
    let rendered = render_feed(feed, 80, 24);
    assert_snapshot!("feed_user_message", rendered);
}

#[test]
fn test_user_and_assistant_simple() {
    let feed = Feed::builder()
        .user_message("Hello AI")
        .assistant()
            .say("Hello! How can I help you today?")
        .build();
    let rendered = render_feed(feed, 80, 24);
    assert_snapshot!("feed_user_and_assistant_simple", rendered);
}

// ============================================================================
// Thoughts Snapshots
// ============================================================================

#[test]
fn test_assistant_with_thinking() {
    let feed = Feed::builder()
        .user_message("Hello")
        .assistant()
            .thinking_for(std::time::Duration::from_millis(200))
            .say("Hey you too!")
        .build();
    let rendered = render_feed(feed, 80, 24);
    assert_snapshot!("feed_assistant_with_thinking", rendered);
}

#[test]
fn test_assistant_with_thoughts_and_turn_timing() {
    let feed = Feed::builder()
        .user_message("Hey")
        .assistant()
            .thinking_for(std::time::Duration::from_secs_f32(0.2))
            .say("Hey you too!")
            .turn_completed_in(std::time::Duration::from_secs_f32(1.5))
        .build();
    let rendered = render_feed(feed, 80, 24);
    assert_snapshot!("feed_thoughts_and_turn_timing", rendered);
}

// ============================================================================
// Tool Call Snapshots
// ============================================================================

#[test]
fn test_assistant_with_tool_call() {
    let feed = Feed::builder()
        .user_message("what time is it?")
        .assistant()
            .thinking_for(std::time::Duration::from_millis(500))
            .tool_call("date", serde_json::json!({}))
            .thinking_for(std::time::Duration::from_millis(100))
            .say("Sat May 30 09:30:16 -05 2026")
            .turn_completed_in(std::time::Duration::from_secs_f32(4.0))
        .build();
    let rendered = render_feed(feed, 80, 24);
    assert_snapshot!("feed_assistant_with_tool_call", rendered);
}

#[test]
fn test_assistant_with_multiple_tool_calls() {
    let feed = Feed::builder()
        .user_message("Check my system")
        .assistant()
            .thinking_for(std::time::Duration::from_millis(100))
            .tool_call("bash", serde_json::json!({"command": "uname -a"}))
            .tool_call("bash", serde_json::json!({"command": "uptime"}))
            .say("Your system is running well!")
        .build();
    let rendered = render_feed(feed, 80, 24);
    assert_snapshot!("feed_multiple_tool_calls", rendered);
}

// ============================================================================
// Multi-turn Conversation Snapshots
// ============================================================================

#[test]
fn test_multi_turn_conversation() {
    let feed = Feed::builder()
        .user_message("Hello")
        .assistant()
            .say("Hi there!")
        .user_message("How are you?")
        .assistant()
            .thinking_for(std::time::Duration::from_millis(50))
            .say("I'm doing great, thanks for asking!")
        .build();
    let rendered = render_feed(feed, 80, 24);
    assert_snapshot!("feed_multi_turn", rendered);
}

// ============================================================================
// Code Block Snapshots
// ============================================================================

#[test]
fn test_assistant_with_code_block() {
    let feed = Feed::builder()
        .user_message("Show me a hello world")
        .assistant()
            .say("Here is a simple Hello World in Rust:\n```rust\nfn main() {\n    println!(\"Hello, world!\");\n}\n```")
        .build();
    let rendered = render_feed(feed, 80, 24);
    assert_snapshot!("feed_code_block", rendered);
}

// ============================================================================
// System Notice Snapshot
// ============================================================================

#[test]
fn test_system_notice() {
    let feed = Feed::builder()
        .user_message("Hello")
        .assistant()
            .say("Hi!")
        .build();
    let mut feed = feed;
    feed.add_system_notice("Using model: gpt-4o-mini".to_string());
    let rendered = render_feed(feed, 80, 24);
    assert_snapshot!("feed_system_notice", rendered);
}

// ============================================================================
// Different Terminal Sizes
// ============================================================================

#[test]
fn test_narrow_terminal() {
    let feed = Feed::builder()
        .user_message("Hello")
        .assistant()
            .say("Hi there!")
        .build();
    let rendered = render_feed(feed, 40, 20);
    assert_snapshot!("feed_narrow_terminal", rendered);
}

#[test]
fn test_wide_terminal() {
    let feed = Feed::builder()
        .user_message("Hello")
        .assistant()
            .say("Hi there! How can I help you today?")
        .build();
    let rendered = render_feed(feed, 120, 24);
    assert_snapshot!("feed_wide_terminal", rendered);
}

#[test]
fn test_short_terminal() {
    let feed = Feed::builder()
        .user_message("Hello")
        .assistant()
            .say("Hi!")
        .build();
    let rendered = render_feed(feed, 80, 10);
    assert_snapshot!("feed_short_terminal", rendered);
}

// ============================================================================
// Long Content Snapshots
// ============================================================================

#[test]
fn test_long_user_message() {
    let long_text = "A".repeat(200);
    let feed = Feed::builder()
        .user_message(long_text)
        .build();
    let rendered = render_feed(feed, 60, 24);
    assert_snapshot!("feed_long_user_message", rendered);
}

#[test]
fn test_long_assistant_message() {
    let feed = Feed::builder()
        .user_message("Tell me a story")
        .assistant()
            .say("Once upon a time, in a land far, far away, there lived a young developer who loved writing code. The end.")
        .build();
    let rendered = render_feed(feed, 60, 24);
    assert_snapshot!("feed_long_assistant_message", rendered);
}

#[test]
fn test_exact_conversation_snapshot() {
    let feed = Feed::builder()
        .user_message("Hey!")
        .assistant()
            .thinking_for(std::time::Duration::from_millis(200))
            .say("Hey! 👋\n\nHow can I help you with your\ncode or project today?")
            .turn_completed_in(std::time::Duration::from_secs_f32(1.5))
        .build();
    let rendered = render_feed(feed, 80, 24);
    assert_snapshot!("feed_exact_conversation", rendered);
}

// ============================================================================
// Requested Test Cases
// ============================================================================

#[test]
fn test_empty_state() {
    let feed = Feed::builder().build();
    let rendered = render_feed(feed, 80, 24);
    assert_snapshot!("feed_empty_state", rendered);
}

#[test]
fn test_single_user_hello() {
    let feed = Feed::builder().user_message("Hello").build();
    let rendered = render_feed(feed, 80, 24);
    assert_snapshot!("feed_single_user_hello", rendered);
}

#[test]
fn test_user_hi_assistant_hello() {
    let feed = Feed::builder()
        .user_message("Hi")
        .assistant().say("Hello!").build();
    let rendered = render_feed(feed, 80, 24);
    assert_snapshot!("feed_user_hi_assistant_hello", rendered);
}

#[test]
fn test_user_assistant_with_thoughts() {
    let feed = Feed::builder()
        .user_message("Hello")
        .assistant()
            .thinking_for(std::time::Duration::from_millis(200))
            .say("Hey you too!")
        .build();
    let rendered = render_feed(feed, 80, 24);
    assert_snapshot!("feed_user_assistant_thoughts", rendered);
}

#[test]
fn test_multi_turn_three_exchanges() {
    let feed = Feed::builder()
        .user_message("Hello")
        .assistant().say("Hi there!").done()
        .user_message("How are you?")
        .assistant().say("I'm good, thanks!").done()
        .user_message("What's for lunch?")
        .assistant().say("How about pizza?").done()
        .build();
    let rendered = render_feed(feed, 80, 24);
    assert_snapshot!("feed_multi_turn_3", rendered);
}

#[test]
fn test_tool_call_bash_files() {
    let feed = Feed::builder()
        .user_message("Show me the files")
        .assistant()
            .tool_call("bash", serde_json::json!({"command": "ls -la"}))
            .say("Here are the files in your directory:")
        .build();
    let rendered = render_feed(feed, 80, 24);
    assert_snapshot!("feed_tool_call_bash", rendered);
}

#[test]
fn test_error_state_tool_failure() {
    let feed = Feed::builder()
        .user_message("Delete the file")
        .assistant()
            .tool_call("bash", serde_json::json!({"command": "rm important.txt"}))
            .say("I couldn't delete the file.")
        .build();
    let mut feed = feed;
    feed.add_system_notice("Tool execution failed: permission denied".to_string());
    let rendered = render_feed(feed, 80, 24);
    assert_snapshot!("feed_error_state", rendered);
}

#[test]
fn test_long_message_wraps() {
    let feed = Feed::builder()
        .user_message("Tell me everything about something very long that takes up multiple lines when rendered")
        .assistant()
            .say("This is a very long response that should wrap correctly when the terminal is narrow. It contains multiple sentences and should demonstrate proper text wrapping behavior at different terminal widths.")
        .build();
    let rendered = render_feed(feed, 50, 30);
    assert_snapshot!("feed_long_message_wrap", rendered);
}

#[test]
fn test_top_bar_model() {
    let feed = Feed::builder()
        .user_message("Hello")
        .assistant().say("Hi!").build();
    let mut feed = feed;
    feed.add_system_notice("openai/gpt-4o".to_string());
    let rendered = render_feed(feed, 80, 24);
    assert_snapshot!("feed_top_bar_model", rendered);
}

#[test]
fn test_global_tags_running() {
    let feed = Feed::builder()
        .user_message("Hello")
        .assistant()
            .thinking_for(std::time::Duration::from_millis(500))
            .say("Thinking...")
        .build();
    let rendered = render_feed_with_agent(feed, 80, 24, true);
    assert_snapshot!("feed_global_tags_running", rendered);
}
