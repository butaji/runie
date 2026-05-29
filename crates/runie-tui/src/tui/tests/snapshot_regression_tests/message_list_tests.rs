use super::*;

#[test]
fn snapshot_main_ui_empty_state() {
    let vm = MessageListViewModel {
        messages: vec![],
        scroll_offset: 0,
        agent_running: false,
        animation: crate::tui::state::AnimationState::default(),
        wrap_cache: WrapCache::new(),
    };
    let theme = ThemeWrapper::default();
    let area = Rect::new(0, 2, 80, 18);
    let mut buf = Buffer::empty(area);
    MessageList::render_ref(&vm, area, &mut buf, &theme);
    insta::assert_snapshot!("snapshot_main_ui_empty_state", buffer_to_string(&buf));
}

#[test]
fn snapshot_main_ui_with_messages() {
    let vm = MessageListViewModel {
        messages: vec![
            user_message("Hello AI"),
            assistant_message("Hello! How can I help you today?"),
        ],
        scroll_offset: 0,
        agent_running: false,
        animation: crate::tui::state::AnimationState::default(),
        wrap_cache: WrapCache::new(),
    };
    let theme = ThemeWrapper::default();
    let area = Rect::new(0, 2, 80, 18);
    let mut buf = Buffer::empty(area);
    MessageList::render_ref(&vm, area, &mut buf, &theme);
    insta::assert_snapshot!("snapshot_main_ui_with_messages", buffer_to_string(&buf));
}

#[test]
fn snapshot_error_message() {
    let vm = MessageListViewModel {
        messages: vec![error_message("Network timeout after 30 seconds", true)],
        scroll_offset: 0,
        agent_running: false,
        animation: crate::tui::state::AnimationState::default(),
        wrap_cache: WrapCache::new(),
    };
    let theme = ThemeWrapper::default();
    let area = Rect::new(0, 2, 80, 18);
    let mut buf = Buffer::empty(area);
    MessageList::render_ref(&vm, area, &mut buf, &theme);
    insta::assert_snapshot!("snapshot_error_message", buffer_to_string(&buf));
}

#[test]
fn snapshot_system_message() {
    let vm = MessageListViewModel {
        messages: vec![system_message("Using model: gpt-4o-mini")],
        scroll_offset: 0,
        agent_running: false,
        animation: crate::tui::state::AnimationState::default(),
        wrap_cache: WrapCache::new(),
    };
    let theme = ThemeWrapper::default();
    let area = Rect::new(0, 2, 80, 18);
    let mut buf = Buffer::empty(area);
    MessageList::render_ref(&vm, area, &mut buf, &theme);
    insta::assert_snapshot!("snapshot_system_message", buffer_to_string(&buf));
}

#[test]
fn snapshot_tool_result_message() {
    let vm = MessageListViewModel {
        messages: vec![
            tool_call("bash", r#"{"command": "ls"}"#, None, false),
            MessageItem::ToolComplete { name: "bash".to_string(), result: "README.md\nsrc\ntests".to_string(), lines: Some(3) },
        ],
        scroll_offset: 0,
        agent_running: false,
        animation: crate::tui::state::AnimationState::default(),
        wrap_cache: WrapCache::new(),
    };
    let theme = ThemeWrapper::default();
    let area = Rect::new(0, 2, 80, 18);
    let mut buf = Buffer::empty(area);
    MessageList::render_ref(&vm, area, &mut buf, &theme);
    insta::assert_snapshot!("snapshot_tool_result_message", buffer_to_string(&buf));
}

#[test]
fn snapshot_code_block_message() {
    let code = r#"fn main() {
    println!("Hello, world!");
}"#;
    let vm = MessageListViewModel {
        messages: vec![assistant_message(&format!("Here is some code:\n```rust\n{}\n```", code))],
        scroll_offset: 0,
        agent_running: false,
        animation: crate::tui::state::AnimationState::default(),
        wrap_cache: WrapCache::new(),
    };
    let theme = ThemeWrapper::default();
    let area = Rect::new(0, 2, 80, 18);
    let mut buf = Buffer::empty(area);
    MessageList::render_ref(&vm, area, &mut buf, &theme);
    insta::assert_snapshot!("snapshot_code_block_message", buffer_to_string(&buf));
}

#[test]
fn snapshot_narrow_terminal_40cols() {
    let vm = MessageListViewModel {
        messages: vec![user_message("Hello"), assistant_message("Hi there! How can I help?")],
        scroll_offset: 0,
        agent_running: false,
        animation: crate::tui::state::AnimationState::default(),
        wrap_cache: WrapCache::new(),
    };
    let theme = ThemeWrapper::default();
    let area = Rect::new(0, 2, 40, 18);
    let mut buf = Buffer::empty(area);
    MessageList::render_ref(&vm, area, &mut buf, &theme);
    insta::assert_snapshot!("snapshot_narrow_terminal_40cols", buffer_to_string(&buf));
}

#[test]
fn snapshot_wide_terminal_120cols() {
    let vm = MessageListViewModel {
        messages: vec![user_message("Hello"), assistant_message("Hi there! How can I help you today?")],
        scroll_offset: 0,
        agent_running: false,
        animation: crate::tui::state::AnimationState::default(),
        wrap_cache: WrapCache::new(),
    };
    let theme = ThemeWrapper::default();
    let area = Rect::new(0, 2, 120, 18);
    let mut buf = Buffer::empty(area);
    MessageList::render_ref(&vm, area, &mut buf, &theme);
    insta::assert_snapshot!("snapshot_wide_terminal_120cols", buffer_to_string(&buf));
}

#[test]
fn snapshot_short_terminal_12rows() {
    let vm = MessageListViewModel {
        messages: vec![user_message("Hello"), assistant_message("Hi!")],
        scroll_offset: 0,
        agent_running: false,
        animation: crate::tui::state::AnimationState::default(),
        wrap_cache: WrapCache::new(),
    };
    let theme = ThemeWrapper::default();
    let area = Rect::new(0, 2, 80, 6);
    let mut buf = Buffer::empty(area);
    MessageList::render_ref(&vm, area, &mut buf, &theme);
    insta::assert_snapshot!("snapshot_short_terminal_12rows", buffer_to_string(&buf));
}

#[test]
fn snapshot_plan_steps() {
    let vm = MessageListViewModel {
        messages: vec![
            MessageItem::PlanStep { step: 1, text: "Analyze the codebase".to_string(), status: PlanStatus::Complete },
            MessageItem::PlanStep { step: 2, text: "Write tests".to_string(), status: PlanStatus::Active },
            MessageItem::PlanStep { step: 3, text: "Implement feature".to_string(), status: PlanStatus::Pending },
        ],
        scroll_offset: 0,
        agent_running: true,
        animation: crate::tui::state::AnimationState::default(),
        wrap_cache: WrapCache::new(),
    };
    let theme = ThemeWrapper::default();
    let area = Rect::new(0, 2, 80, 18);
    let mut buf = Buffer::empty(area);
    MessageList::render_ref(&vm, area, &mut buf, &theme);
    insta::assert_snapshot!("snapshot_plan_steps", buffer_to_string(&buf));
}

#[test]
fn snapshot_tool_call_error() {
    let vm = MessageListViewModel {
        messages: vec![
            tool_call("bash", r#"{"command": "ls"}"#, None, false),
            tool_call("bash", r#"{"command": "rm file"}"#, Some("rm: cannot remove 'file': Permission denied"), true),
        ],
        scroll_offset: 0,
        agent_running: false,
        animation: crate::tui::state::AnimationState::default(),
        wrap_cache: WrapCache::new(),
    };
    let theme = ThemeWrapper::default();
    let area = Rect::new(0, 2, 80, 18);
    let mut buf = Buffer::empty(area);
    MessageList::render_ref(&vm, area, &mut buf, &theme);
    insta::assert_snapshot!("snapshot_tool_call_error", buffer_to_string(&buf));
}

#[test]
fn snapshot_long_message_wrap() {
    let long_text = "A".repeat(200);
    let vm = MessageListViewModel {
        messages: vec![assistant_message(&long_text)],
        scroll_offset: 0,
        agent_running: false,
        animation: crate::tui::state::AnimationState::default(),
        wrap_cache: WrapCache::new(),
    };
    let theme = ThemeWrapper::default();
    let area = Rect::new(0, 2, 60, 18);
    let mut buf = Buffer::empty(area);
    MessageList::render_ref(&vm, area, &mut buf, &theme);
    insta::assert_snapshot!("snapshot_long_message_wrap", buffer_to_string(&buf));
}
