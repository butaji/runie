use super::*;

/// Test that context window calculation uses chars / 4
#[test]
fn test_context_window_chars_div_4() {
    use crate::loop_engine::calculate_context_window_usage;

    let messages = vec![AgentMessage {
        role: "user".to_string(),
        content: vec![ContentPart::Text { text: "hello".to_string() }],
        timestamp: 0,
        usage: None,
        stop_reason: None,
        error_message: None,
        tool_calls: vec![],
    }];

    let context_window = 100_000;
    let usage = calculate_context_window_usage(&messages, context_window);

    assert!(usage < 0.01, "5 chars should be ~0 tokens percentage");
}
