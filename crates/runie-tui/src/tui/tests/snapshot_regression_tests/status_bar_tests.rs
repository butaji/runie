use super::*;

#[test]
fn snapshot_model_overlay() {
    let vm = StatusBarViewModel {
        mode: TuiMode::Select,
        current_model: Some("openai/gpt-4o".to_string()),
        session_token_usage: TokenUsage { total_tokens: 5000, estimated_cost: 0.0234, ..Default::default() },
        agent_running: false,
        thinking_duration_secs: None,
        message_count: 5,
        max_messages: 50,
    };
    let colors = make_test_colors();
    let area = Rect::new(0, 23, 80, 1);
    let mut buf = Buffer::empty(area);
    render_status_bar(&vm, area, &mut buf, &colors);
    insta::assert_snapshot!("snapshot_model_overlay", buffer_to_string(&buf));
}

#[test]
fn snapshot_status_bar_with_model() {
    let vm = StatusBarViewModel {
        mode: TuiMode::Chat,
        current_model: Some("openai/gpt-4o".to_string()),
        session_token_usage: TokenUsage { total_tokens: 5000, estimated_cost: 0.0234, ..Default::default() },
        agent_running: false,
        thinking_duration_secs: None,
        message_count: 5,
        max_messages: 50,
    };
    let colors = make_test_colors();
    let area = Rect::new(0, 23, 80, 1);
    let mut buf = Buffer::empty(area);
    render_status_bar(&vm, area, &mut buf, &colors);
    insta::assert_snapshot!("snapshot_status_bar_with_model", buffer_to_string(&buf));
}

#[test]
fn snapshot_status_bar_no_model() {
    let vm = StatusBarViewModel {
        mode: TuiMode::Chat,
        current_model: None,
        session_token_usage: TokenUsage::default(),
        agent_running: false,
        thinking_duration_secs: None,
        message_count: 0,
        max_messages: 50,
    };
    let colors = make_test_colors();
    let area = Rect::new(0, 23, 80, 1);
    let mut buf = Buffer::empty(area);
    render_status_bar(&vm, area, &mut buf, &colors);
    insta::assert_snapshot!("snapshot_status_bar_no_model", buffer_to_string(&buf));
}

#[test]
fn snapshot_dark_theme_status_bar() {
    let vm = StatusBarViewModel {
        mode: TuiMode::Chat,
        current_model: Some("MiniMax-M2.7-highspeed".to_string()),
        session_token_usage: TokenUsage { total_tokens: 5000, estimated_cost: 0.0234, ..Default::default() },
        agent_running: false,
        thinking_duration_secs: None,
        message_count: 5,
        max_messages: 50,
    };
    let colors = make_test_colors();
    let area = Rect::new(0, 23, 80, 1);
    let mut buf = Buffer::empty(area);
    render_status_bar(&vm, area, &mut buf, &colors);
    insta::assert_snapshot!("snapshot_dark_theme_status_bar", buffer_to_string(&buf));
}
