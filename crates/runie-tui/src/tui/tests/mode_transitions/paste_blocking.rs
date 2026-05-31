//! Tests for paste blocking in blocking modes.
//!
//! Verifies that paste events are:
//! - Blocked in Permission mode
//! - Blocked in Overlay mode
//! - Allowed in Chat mode
//! - Allowed in other non-blocking modes

use super::*;

/// Test: Paste blocked in Permission mode.
#[test]
fn test_paste_blocked_in_permission() {
    let msgs = simulate_paste("some text", TuiMode::Permission);
    assert!(msgs.is_empty(), "Paste should be blocked in Permission mode");
}

/// Test: Paste blocked in Overlay mode.
#[test]
fn test_paste_blocked_in_overlay() {
    let msgs = simulate_paste("some text", TuiMode::Overlay);
    assert!(msgs.is_empty(), "Paste should be blocked in Overlay mode");
}

/// Test: Paste allowed in Chat mode.
#[test]
fn test_paste_allowed_in_chat() {
    let msgs = simulate_paste("some text", TuiMode::Chat);
    assert!(!msgs.is_empty(), "Paste should be allowed in Chat mode");
    assert!(msgs.contains(&Msg::Paste("some text".to_string())));
}

/// Test: Paste allowed in CommandPalette mode.
#[test]
fn test_paste_allowed_in_palette() {
    let msgs = simulate_paste("some text", TuiMode::CommandPalette);
    assert!(!msgs.is_empty(), "Paste should be allowed in CommandPalette mode");
    assert!(msgs.contains(&Msg::Paste("some text".to_string())));
}

/// Test: Paste allowed in Onboarding mode.
#[test]
fn test_paste_allowed_in_onboarding() {
    let msgs = simulate_paste("some text", TuiMode::Onboarding);
    assert!(!msgs.is_empty(), "Paste should be allowed in Onboarding mode");
}

/// Test: Paste allowed in SessionTree mode.
#[test]
fn test_paste_allowed_in_session_tree() {
    let msgs = simulate_paste("some text", TuiMode::SessionTree);
    assert!(!msgs.is_empty(), "Paste should be allowed in SessionTree mode");
}

/// Test: Paste allowed in DiffViewer mode.
#[test]
fn test_paste_allowed_in_diff_viewer() {
    let msgs = simulate_paste("some text", TuiMode::DiffViewer);
    assert!(!msgs.is_empty(), "Paste should be allowed in DiffViewer mode");
}

/// Test: Paste content preserved when allowed.
#[test]
fn test_paste_content_preserved_when_allowed() {
    let test_text = "hello world with special chars: !@#$%^&*()";
    let msgs = simulate_paste(test_text, TuiMode::Chat);
    assert!(msgs.contains(&Msg::Paste(test_text.to_string())));
}

/// Test: Multi-line paste allowed in Chat.
#[test]
fn test_multiline_paste_allowed_in_chat() {
    let multiline = "line1\nline2\nline3";
    let msgs = simulate_paste(multiline, TuiMode::Chat);
    assert!(msgs.contains(&Msg::Paste(multiline.to_string())));
}

/// Test: Empty paste still produces message when allowed.
#[test]
fn test_empty_paste_produces_message_when_allowed() {
    let msgs = simulate_paste("", TuiMode::Chat);
    assert!(!msgs.is_empty(), "Empty paste should still produce message");
    assert!(msgs.contains(&Msg::Paste("".to_string())));
}

/// Test: Very long paste allowed in Chat.
#[test]
fn test_long_paste_allowed_in_chat() {
    let long_text = "x".repeat(10000);
    let msgs = simulate_paste(&long_text, TuiMode::Chat);
    assert!(msgs.contains(&Msg::Paste(long_text)));
}

/// Test: Paste blocked in Permission via event_to_msg.
#[test]
fn test_paste_blocked_in_permission_via_event_to_msg() {
    use crossterm::event::Event;

    let event = Event::Paste("test".to_string());
    let state = AppState {
        mode: TuiMode::Permission,
        ..Default::default()
    };
    let msgs = event_to_msg(event, &state);
    assert!(msgs.is_empty(), "Paste blocked in Permission via event_to_msg");
}

/// Test: Paste blocked in Overlay via event_to_msg.
#[test]
fn test_paste_blocked_in_overlay_via_event_to_msg() {
    use crossterm::event::Event;

    let event = Event::Paste("test".to_string());
    let state = AppState {
        mode: TuiMode::Overlay,
        ..Default::default()
    };
    let msgs = event_to_msg(event, &state);
    assert!(msgs.is_empty(), "Paste blocked in Overlay via event_to_msg");
}

/// Test: Paste with newlines blocked in Permission.
#[test]
fn test_paste_with_newlines_blocked_in_permission() {
    let msgs = simulate_paste("line1\nline2", TuiMode::Permission);
    assert!(msgs.is_empty(), "Paste with newlines should be blocked in Permission");
}

/// Test: Paste with newlines blocked in Overlay.
#[test]
fn test_paste_with_newlines_blocked_in_overlay() {
    let msgs = simulate_paste("line1\nline2", TuiMode::Overlay);
    assert!(msgs.is_empty(), "Paste with newlines should be blocked in Overlay");
}

/// Test: Paste returns empty vec in blocking modes (not None or error).
#[test]
fn test_paste_returns_empty_vec_not_error() {
    let permission_result = simulate_paste("test", TuiMode::Permission);
    let overlay_result = simulate_paste("test", TuiMode::Overlay);

    // Should be empty Vec, not None
    assert_eq!(permission_result, Vec::<Msg>::new());
    assert_eq!(overlay_result, Vec::<Msg>::new());
}
