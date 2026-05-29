use super::*;

#[test]
fn test_e2e_mode_chat_to_palette_to_chat() {
    let mut state = make_state_with_model("openai/gpt-4o");
    let mut palette = CommandPalette::new();

    // Start in Chat
    assert_eq!(state.mode, TuiMode::Chat);

    // Open palette
    update(&mut state, &mut palette, Msg::OpenCommandPalette);
    assert_eq!(state.mode, TuiMode::CommandPalette);

    // Cancel back to Chat
    update(&mut state, &mut palette, Msg::CloseModal);
    assert_eq!(state.mode, TuiMode::Chat);
}

#[test]
fn test_e2e_mode_chat_to_permission_to_chat() {
    let mut state = make_state_with_model("openai/gpt-4o");
    let mut palette = CommandPalette::new();

    // Trigger permission request
    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::PermissionRequest {
        tool_call_id: "tool_test".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "ls".to_string(),
        tool_description: "List files".to_string(),
        turn: 1,
        context_window_usage: 0.5,
    }));
    assert_eq!(state.mode, TuiMode::Permission);

    // Confirm and return to Chat
    update(&mut state, &mut palette, Msg::PermissionConfirm);
    assert_eq!(state.mode, TuiMode::Chat);
}

#[test]
fn test_e2e_mode_permission_deny_returns_to_chat() {
    let mut state = make_state_with_model("openai/gpt-4o");
    let mut palette = CommandPalette::new();

    // Set up permission mode manually
    state.mode = TuiMode::Permission;
    state.permission_modal.tool = Some("bash".to_string());
    state.permission_modal.tool_call_id = Some("tool_deny".to_string());

    // Deny
    update(&mut state, &mut palette, Msg::PermissionCancel);
    assert_eq!(state.mode, TuiMode::Chat);
}

#[test]
fn test_e2e_mode_chat_to_onboarding() {
    let mut state = make_state_with_model("openai/gpt-4o");
    let mut palette = CommandPalette::new();

    // Enter onboarding
    update(&mut state, &mut palette, Msg::EnterOnboarding);
    assert_eq!(state.mode, TuiMode::Onboarding);

    // Exit onboarding
    update(&mut state, &mut palette, Msg::OnboardingSkip);
    assert_eq!(state.mode, TuiMode::Chat);
}
