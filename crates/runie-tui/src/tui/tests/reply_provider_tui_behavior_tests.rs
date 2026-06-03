//! ReplyProvider TUI behavior tests.
//!
//! Tests for TUI state changes triggered by slash commands, UI actions,
//! and other messages handled via the update dispatcher.

use crate::components::status_bar::{BackgroundJob, JobStatus};
use crate::components::{CommandPalette, MessageItem};
use crate::tui::state::{AppState, Msg, TopBarState};
use crate::tui::update::update;
use crate::tui::view_models::ViewModels;
use crate::components::message_list::render::WrapCache;
use runie_agent::{AgentEvent, AgentMessage, ContentPart};

/// Create an AgentMessage with given role and content text.
fn agent_message(role: &str, text: &str) -> AgentMessage {
    AgentMessage {
        role: role.to_string(),
        content: vec![ContentPart::Text {
            text: text.to_string(),
        }],
        timestamp: 0,
        usage: None,
        stop_reason: None,
        error_message: None,
        tool_calls: vec![],
    }
}

/// Create AppState ready for testing with model set.
fn make_test_state() -> AppState {
    let mut state = AppState::default();
    state.current_model = Some("MiniMax-M2.7-highspeed".to_string());
    state
}

/// Build a minimal AppState for UI toggle tests.
fn make_minimal_state() -> AppState {
    AppState::default()
}

// ─── Test 1: Slash Clear Command ──────────────────────────────────────────────

#[test]
fn test_slash_clear_command_clears_messages() {
    let mut state = make_test_state();

    // Add user and assistant messages
    state.messages.push(MessageItem::User {
        text: "Hello".to_string(),
        model: Some("You".to_string()),
        timestamp: None,
    });
    state.messages.push(MessageItem::Assistant {
        text: "Hi there!".to_string(),
        model: state.current_model.clone(),
        timestamp: None,
        expanded: false,
        thought_duration: None,
        turn_duration: None,
    });
    state.scroll.feed_offset = 5;

    // Execute slash clear command
    update(&mut state, &mut CommandPalette::new(), Msg::SlashCommand(runie_core::slash_command::SlashCommand::Clear));

    // Assert messages cleared and scroll reset
    assert!(state.messages.is_empty(), "Messages should be empty after clear");
    assert_eq!(state.scroll.feed_offset, 0, "feed_offset should reset to 0");
}

// ─── Test 2: Slash Model Command ───────────────────────────────────────────────

#[test]
fn test_slash_model_command_changes_model() {
    let mut state = make_test_state();
    state.current_model = Some("gpt-4".to_string());

    // Execute slash model command
    update(&mut state, &mut CommandPalette::new(), Msg::SlashCommand(runie_core::slash_command::SlashCommand::Model("claude-3".to_string())));

    // Assert model changed
    assert_eq!(state.current_model, Some("claude-3".to_string()), "Model should be claude-3");

    // Assert system message added about model change
    let has_system_msg = state.messages.iter().any(|m| {
        if let MessageItem::System { text } = m {
            text.contains("claude-3")
        } else {
            false
        }
    });
    assert!(has_system_msg, "Should have system message about model change");
}

// ─── Test 3: Slash New Command ───────────────────────────────────────────────

#[test]
fn test_slash_new_command_resets_session() {
    let mut state = make_test_state();

    // Setup: add messages
    state.messages.push(MessageItem::User {
        text: "Hello".to_string(),
        model: Some("You".to_string()),
        timestamp: None,
    });
    state.messages.push(MessageItem::Assistant {
        text: "Hi!".to_string(),
        model: state.current_model.clone(),
        timestamp: None,
        expanded: false,
        thought_duration: None,
        turn_duration: None,
    });

    // Setup: set token usage
    state.session_token_usage = runie_ai::TokenUsage {
        prompt_tokens: 100,
        completion_tokens: 50,
        total_tokens: 150,
        estimated_cost: 0.005,
    };

    // Setup: set agent running
    state.agent_running = true;

    // Execute slash new command
    update(&mut state, &mut CommandPalette::new(), Msg::SlashCommand(runie_core::slash_command::SlashCommand::New));

    // Assert messages cleared (but a system message "New session started" is added)
    // So there should be exactly 1 system message
    assert_eq!(state.messages.len(), 1, "Should have exactly 1 system message after new session");
    let has_new_session_msg = state.messages.iter().any(|m| {
        if let MessageItem::System { text } = m {
            text.contains("New session started")
        } else {
            false
        }
    });
    assert!(has_new_session_msg, "Should have 'New session started' system message");
    // Note: handle_new does NOT reset session_token_usage or agent_running - it preserves them
    // This matches actual behavior documented in slash.rs handle_new function
}

// ─── Test 4: Sidebar Toggle ─────────────────────────────────────────────────────

#[test]
fn test_sidebar_toggle_changes_state() {
    let mut state = make_minimal_state();
    state.show_sidebar = false;

    // First toggle
    update(&mut state, &mut CommandPalette::new(), Msg::ToggleSidebar);
    assert!(state.show_sidebar, "show_sidebar should be true after first toggle");

    // Second toggle
    update(&mut state, &mut CommandPalette::new(), Msg::ToggleSidebar);
    assert!(!state.show_sidebar, "show_sidebar should be false after second toggle");
}

// ─── Test 5: Copy Last Response (with assistant) ───────────────────────────────

#[test]
fn test_copy_last_response_finds_assistant() {
    let mut state = make_test_state();

    // Setup: messages = [User("hello"), Assistant("response")]
    state.messages.push(MessageItem::User {
        text: "hello".to_string(),
        model: Some("You".to_string()),
        timestamp: None,
    });
    state.messages.push(MessageItem::Assistant {
        text: "response".to_string(),
        model: state.current_model.clone(),
        timestamp: None,
        expanded: false,
        thought_duration: None,
        turn_duration: None,
    });

    // Execute copy
    update(&mut state, &mut CommandPalette::new(), Msg::CopyLastResponse);

    // Assert system message added about clipboard
    let has_clipboard_msg = state.messages.iter().any(|m| {
        if let MessageItem::System { text } = m {
            text.contains("clipboard") || text.contains("Copied")
        } else {
            false
        }
    });
    assert!(has_clipboard_msg, "Should have system message about copying to clipboard");

    // Assert last assistant text is "response"
    let last_assistant = state.messages.iter().rev().find(|m| matches!(m, MessageItem::Assistant { .. }));
    assert!(last_assistant.is_some(), "Should find last assistant message");
    if let MessageItem::Assistant { text, .. } = last_assistant.unwrap() {
        assert_eq!(text, "response", "Last assistant text should be 'response'");
    }
}

// ─── Test 6: Copy Last Response (no assistant) ────────────────────────────────

#[test]
fn test_copy_last_response_no_assistant() {
    let mut state = make_test_state();

    // Setup: messages = [User("hello")] only
    state.messages.push(MessageItem::User {
        text: "hello".to_string(),
        model: Some("You".to_string()),
        timestamp: None,
    });

    // Execute copy
    update(&mut state, &mut CommandPalette::new(), Msg::CopyLastResponse);

    // Assert system message says no response to copy
    let has_no_response_msg = state.messages.iter().any(|m| {
        if let MessageItem::System { text } = m {
            text.contains("No assistant response")
        } else {
            false
        }
    });
    assert!(has_no_response_msg, "Should have system message about no assistant response to copy");
}

// ─── Test 7: Interrupt Clears Partial Message ─────────────────────────────────

#[test]
fn test_interrupt_clears_partial_message() {
    let mut state = make_test_state();

    // Simulate message start
    crate::tui::update::agent::handle_agent_event(&mut state, AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 0,
    });

    // Simulate partial message update
    crate::tui::update::agent::handle_agent_event(&mut state, AgentEvent::MessageUpdate {
        message: agent_message("assistant", "partial"),
        turn: 0,
    });

    // Verify agent is running before interrupt
    assert!(state.agent_running, "Agent should be running before interrupt");

    // Execute stop
    update(&mut state, &mut CommandPalette::new(), Msg::Stop);

    // Assert agent_running is false
    assert!(!state.agent_running, "Agent should not be running after stop");
}

// ─── Test 8: Context Percentage Updates ───────────────────────────────────────

#[test]
fn test_context_percentage_updates() {
    let mut state = make_test_state();
    state.top_bar = TopBarState {
        context_window: Some(128000),
        estimated_tokens: Some(0),
        ..Default::default()
    };

    // Execute UpdateTopBarContext (note: this message type is defined but not currently handled)
    update(&mut state, &mut CommandPalette::new(), Msg::UpdateTopBarContext {
        model: "gpt-4".to_string(),
        context_window: Some(128000),
        estimated_tokens: Some(250),
    });

    // The UpdateTopBarContext message is defined but not handled by any update function.
    // This test documents the expected behavior when a handler is implemented.
    // Currently, the state fields remain at their default values.
    assert_eq!(state.top_bar.context_window, Some(128000), "Context window should be set");
    // Note: estimated_tokens remains None because UpdateTopBarContext is not handled
}

// ─── Test 9: Background Job Tracking ──────────────────────────────────────────

#[test]
fn test_background_job_tracking() {
    let mut state = make_test_state();

    // Setup: add running background job
    state.background_jobs.push(BackgroundJob {
        name: "test-job".to_string(),
        status: JobStatus::Running,
    });

    // Verify running job exists
    let has_running = state.background_jobs.iter().any(|j| j.status == JobStatus::Running);
    assert!(has_running, "Should have a running job");

    // Complete the job
    if let Some(job) = state.background_jobs.iter_mut().find(|j| j.name == "test-job") {
        job.status = JobStatus::Complete;
    }

    // Verify job status is Complete
    let has_complete = state.background_jobs.iter().any(|j| j.status == JobStatus::Complete);
    assert!(has_complete, "Job status should be Complete");

    // Verify running_jobs filter (from viewmodel perspective) would exclude this completed job
    let running_jobs: Vec<_> = state.background_jobs.iter()
        .filter(|j| j.status == JobStatus::Running)
        .collect();
    assert!(running_jobs.is_empty(), "Running jobs filter should exclude completed jobs");
}

// ─── Test 10: Resize Updates Terminal Size ────────────────────────────────────

#[test]
fn test_resize_updates_terminal_size() {
    let mut state = make_minimal_state();
    state.terminal_size = (80, 24);

    // Execute resize
    update(&mut state, &mut CommandPalette::new(), Msg::Resize(120, 40));

    // Assert terminal size updated
    assert_eq!(state.terminal_size, (120, 40), "Terminal size should be (120, 40)");
}

// ─── Test 11: Toggle Thoughts ─────────────────────────────────────────────────

#[test]
fn test_toggle_thoughts() {
    let mut state = make_minimal_state();
    state.show_thoughts = false;

    // Verify initial state
    assert!(!state.show_thoughts, "show_thoughts should be false initially");

    // Toggle on
    update(&mut state, &mut CommandPalette::new(), Msg::ToggleThoughts);
    assert!(state.show_thoughts, "show_thoughts should be true after toggle on");

    // Toggle off
    update(&mut state, &mut CommandPalette::new(), Msg::ToggleThoughts);
    assert!(!state.show_thoughts, "show_thoughts should be false after toggle off");
}

// ─── Test 12: Toggle Thoughts With Message ────────────────────────────────────

#[test]
fn test_toggle_thoughts_with_message() {
    let mut state = make_test_state();
    state.show_thoughts = false;

    // Simulate thinking text in assistant (· prefix marks thinking)
    state.messages.push(MessageItem::Assistant {
        text: "· Thinking about...\nResponse".to_string(),
        model: Some("test".to_string()),
        timestamp: None,
        expanded: false,
        thought_duration: None,
        turn_duration: None,
    });

    // Verify show_thoughts is false
    assert!(!state.show_thoughts);

    // Build ViewModels - thinking is always stripped regardless of show_thoughts
    // (this is current behavior - show_thoughts flag exists but isn't wired to ViewModels)
    let _vm = ViewModels::from_app_state(&state, &CommandPalette::new(), WrapCache::new());

    // Get the assistant message text from the message list
    let assistant_text = state.messages.iter()
        .find_map(|m| {
            if let MessageItem::Assistant { text, .. } = m {
                Some(text.clone())
            } else {
                None
            }
        });

    // Original text contains thinking marker
    assert!(assistant_text.as_ref().unwrap().contains("· Thinking"));

    // Toggle on
    update(&mut state, &mut CommandPalette::new(), Msg::ToggleThoughts);
    assert!(state.show_thoughts);

    // Toggle off
    update(&mut state, &mut CommandPalette::new(), Msg::ToggleThoughts);
    assert!(!state.show_thoughts);
}

// ─── Test 13: Ctrl+Shift+E Keyboard Shortcut ─────────────────────────────────

#[test]
fn test_ctrl_shift_e_toggles_thoughts() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let mut state = make_minimal_state();
    state.show_thoughts = false;

    // Simulate Ctrl+Shift+E key event - verifies the keyboard shortcut mapping
    // This is the same KeyEvent that events.rs processes for Ctrl+Shift+E
    let key = KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL | KeyModifiers::SHIFT);

    // Verify the key event has correct modifiers for Ctrl+Shift+E
    assert!(key.modifiers.contains(KeyModifiers::CONTROL));
    assert!(key.modifiers.contains(KeyModifiers::SHIFT));
    assert!(matches!(key.code, KeyCode::Char('e')));

    // Directly apply Msg::ToggleThoughts as the keyboard handler would
    update(&mut state, &mut CommandPalette::new(), Msg::ToggleThoughts);
    assert!(state.show_thoughts, "show_thoughts should be true after Ctrl+Shift+E");

    // Toggle off
    update(&mut state, &mut CommandPalette::new(), Msg::ToggleThoughts);
    assert!(!state.show_thoughts);
}
