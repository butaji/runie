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
    match msg {
        // Agent events that affect UI
        Msg::AgentEvent(AgentEvent::MessageUpdate { .. }) => true,
        Msg::AgentEvent(AgentEvent::PermissionRequest { .. }) => true,
        Msg::AgentEvent(AgentEvent::Error { .. }) => true,
        // Input messages
        Msg::TextareaKey(_) => true,
        Msg::InsertNewline => true,
        Msg::Paste(_) => true,
        Msg::ClearInput => true,
        Msg::ClearInputConfirm => true,
        // Command palette
        Msg::CommandPaletteFilter(_) => true,
        Msg::CommandPaletteBackspace => true,
        Msg::CommandPaletteUp => true,
        Msg::CommandPaletteDown => true,
        Msg::CommandPaletteConfirm => true,
        Msg::CommandPaletteCancelArgument => true,
        // Navigation
        Msg::ScrollUp | Msg::ScrollDown | Msg::ScrollPageUp | Msg::ScrollPageDown => true,
        Msg::SessionTreeUp | Msg::SessionTreeDown => true,
        Msg::SessionTreeConfirm => true,
        Msg::OnboardingNavigateUp | Msg::OnboardingNavigateDown => true,
        Msg::OnboardingSelectProvider(_) | Msg::OnboardingSelectModel(_) => true,
        Msg::OnboardingKeyInput(_) | Msg::OnboardingKeyBackspace => true,
        Msg::OnboardingSearchInput(_) | Msg::OnboardingSearchBackspace => true,
        Msg::SelectUp | Msg::SelectDown => true,
        Msg::SelectConfirm | Msg::SelectToggleDetails => true,
        // Permission
        Msg::PermissionConfirm | Msg::PermissionCancel | Msg::PermissionAlways | Msg::PermissionSkip => true,
        // Mode changes
        Msg::OpenCommandPalette | Msg::CloseModal | Msg::ConfirmModal => true,
        Msg::ToggleSessionTree | Msg::ToggleSidebar => true,
        Msg::SwitchModel => true,
        Msg::OnboardingNext | Msg::OnboardingBack | Msg::OnboardingSubmit | Msg::OnboardingSkip => true,
        Msg::EnterOnboarding => true,
        Msg::DirectCommand(_) => true,
        // Terminal events
        Msg::Resize(..) => true,
        // Commands
        Msg::Submit | Msg::Quit | Msg::ClearChat => true,
        Msg::Stop => true,
        // State updates
        Msg::ModelsFetched(_) | Msg::ModelsFetchFailed(_) => true,
        Msg::SetGitInfo { .. } | Msg::SetTopBarMockChecks { .. } | Msg::SetTopBarRealChecks { .. } => true,
        Msg::SetInputRightInfo(_) | Msg::SetCurrentModel(_) | Msg::SetMockMode(_) => true,
        Msg::ResetAgentState | Msg::UpdateTopBarContext { .. } => true,
        Msg::SlashCommand(_) => true,
        Msg::PermissionTimeout => true,
        _ => false,
    }
}
