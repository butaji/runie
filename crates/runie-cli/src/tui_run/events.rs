use runie_agent::events::AgentEvent;
use runie_tui::Msg;
use runie_tui::pipe::InputMsg;

/// Convert InputMsg to Msg
pub fn input_to_msg(input_msg: InputMsg) -> Msg {
    match input_msg {
        InputMsg::Key(key) => Msg::TextareaKey(key),
        InputMsg::Paste(text) => Msg::Paste(text),
        InputMsg::Resize(w, h) => Msg::Resize(w, h),
    }
}

/// Convert raw key events to proper routed messages
pub fn route_key_event(msg: Msg, state: &runie_tui::AppState) -> Vec<Msg> {
    match msg {
        Msg::TextareaKey(key) => runie_tui::event_to_msg(crossterm::event::Event::Key(key), state),
        other => vec![other],
    }
}

/// Check if a message triggers state change (for render optimization)
pub fn triggers_state_change(msg: &Msg) -> bool {
    is_tick_msg(msg) || is_state_mutating_msg(msg) || is_render_triggering_msg(msg)
}

fn is_tick_msg(msg: &Msg) -> bool {
    matches!(msg, Msg::Tick)
}

fn is_state_mutating_msg(msg: &Msg) -> bool {
    matches!(
        msg,
        Msg::AgentEvent(AgentEvent::MessageUpdate { .. })
            | Msg::AgentEvent(AgentEvent::PermissionRequest { .. })
            | Msg::AgentEvent(AgentEvent::Error { .. })
    )
}

fn is_render_triggering_msg(msg: &Msg) -> bool {
    // Input messages
    is_input_msg(msg)
        // Command palette
        || is_command_palette_msg(msg)
        // Navigation
        || is_navigation_msg(msg)
        // Permission
        || is_permission_msg(msg)
        // Mode changes
        || is_mode_change_msg(msg)
        // Terminal events
        || matches!(msg, Msg::Resize(..))
        // Commands
        || is_command_msg(msg)
        // State updates
        || is_state_update_msg(msg)
}

fn is_input_msg(msg: &Msg) -> bool {
    matches!(
        msg,
        Msg::TextareaKey(_)
            | Msg::InsertNewline
            | Msg::Paste(_)
            | Msg::ClearInput
            | Msg::ClearInputConfirm
    )
}

fn is_command_palette_msg(msg: &Msg) -> bool {
    matches!(
        msg,
        Msg::CommandPaletteFilter(_)
            | Msg::CommandPaletteBackspace
            | Msg::CommandPaletteUp
            | Msg::CommandPaletteDown
            | Msg::CommandPaletteConfirm
            | Msg::CommandPaletteCancelArgument
    )
}

fn is_navigation_msg(msg: &Msg) -> bool {
    matches!(
        msg,
        Msg::ScrollUp | Msg::ScrollDown | Msg::ScrollPageUp | Msg::ScrollPageDown
            | Msg::SessionTreeUp | Msg::SessionTreeDown | Msg::SessionTreeConfirm
            | Msg::OnboardingNavigateUp | Msg::OnboardingNavigateDown
            | Msg::OnboardingSelectProvider(_) | Msg::OnboardingSelectModel(_)
            | Msg::OnboardingKeyInput(_) | Msg::OnboardingKeyBackspace
            | Msg::OnboardingSearchInput(_) | Msg::OnboardingSearchBackspace
            | Msg::SelectUp | Msg::SelectDown
            | Msg::SelectConfirm | Msg::SelectToggleDetails
    )
}

fn is_permission_msg(msg: &Msg) -> bool {
    matches!(
        msg,
        Msg::PermissionConfirm
            | Msg::PermissionCancel
            | Msg::PermissionAlways
            | Msg::PermissionSkip
    )
}

fn is_mode_change_msg(msg: &Msg) -> bool {
    matches!(
        msg,
        Msg::OpenCommandPalette | Msg::CloseModal | Msg::ConfirmModal
            | Msg::ToggleSessionTree | Msg::ToggleSidebar
            | Msg::SwitchModel
            | Msg::OnboardingNext
            | Msg::OnboardingBack
            | Msg::OnboardingSubmit
            | Msg::OnboardingSkip
            | Msg::EnterOnboarding
            | Msg::DirectCommand(_)
    )
}

fn is_command_msg(msg: &Msg) -> bool {
    matches!(msg, Msg::Submit | Msg::Quit | Msg::ClearChat | Msg::Stop)
}

fn is_state_update_msg(msg: &Msg) -> bool {
    matches!(
        msg,
        Msg::ModelsFetched(_)
            | Msg::ModelsFetchFailed(_)
            | Msg::SetGitInfo { .. }
            | Msg::SetTopBarMockChecks { .. }
            | Msg::SetTopBarRealChecks { .. }
            | Msg::SetInputRightInfo(_)
            | Msg::SetCurrentModel(_)
            | Msg::SetMockMode(_)
            | Msg::ResetAgentState
            | Msg::UpdateTopBarContext { .. }
            | Msg::SlashCommand(_)
            | Msg::PermissionTimeout
    )
}
