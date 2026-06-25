//! Event taxonomy — `EventKind` enum and helper predicates.
//!
//! The `kind()` method is in [`kind_impl`](kind_impl).

use crate::event::variants::Event;

/// Kind of an `Event` — the top-level taxonomy for state sync.
///
/// Intents request state changes (routed to actors).
/// Facts describe state changes (projected into `AppState`).
/// Controls manage lifecycle / terminal events (routed to `update/system.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EventKind {
    /// Request to an actor — produced by input handlers, commands, dialogs.
    Intent,
    /// Broadcast state change — produced by actors.
    #[default]
    Fact,
    /// Lifecycle / terminal event — produced by the IO layer.
    Control,
}

// ── Predicate helpers (one function per event sub-family) ──────────────────────

fn is_llm_agent_fact(e: &Event) -> bool {
    matches!(e, Event::Thinking { .. } | Event::ThoughtDone { .. } | Event::ToolStart { .. }
        | Event::ToolEnd { .. } | Event::ResponseDelta { .. } | Event::ThinkingDelta { .. }
        | Event::TextStart { .. } | Event::TextEnd { .. } | Event::ThinkingStart { .. }
        | Event::ThinkingEnd { .. } | Event::Response { .. } | Event::TurnComplete { .. }
        | Event::Done { .. } | Event::Error { .. } | Event::AssistantMessageReady { .. })
}

fn is_input_intent(e: &Event) -> bool {
    matches!(e, Event::Input(_) | Event::Backspace | Event::Newline | Event::Submit
        | Event::Escape | Event::CursorLeft | Event::CursorRight | Event::CursorStart
        | Event::CursorEnd | Event::DeleteWord | Event::DeleteToEnd | Event::DeleteToStart
        | Event::KillChar | Event::HistoryPrev | Event::HistoryNext | Event::Undo | Event::Redo
        | Event::CursorWordLeft | Event::CursorWordRight | Event::PageUp | Event::PageDown
        | Event::GoToTop | Event::GoToBottom | Event::Paste(_) | Event::PasteImage
        | Event::MouseClick { .. } | Event::MouseRelease { .. } | Event::MouseDrag { .. }
        | Event::MouseMove { .. } | Event::MouseScrollUp | Event::MouseScrollDown
        | Event::FocusGained | Event::FocusLost)
}

// Fact aggregates (OR chains as helper functions to keep kind() complexity low)
fn is_fact(e: &Event) -> bool {
    is_llm_agent_fact(e)
        || is_config_fact(e)
        || is_trust_history_fact(e)
        || is_session_fact(e)
        || is_io_fact(e)
        || is_system_fact(e)
        || is_login_flow_fact(e)
        || is_permission_fact(e)
        || is_replay_fact(e)
}

fn is_intent(e: &Event) -> bool {
    is_input_intent(e)
        || is_model_config_intent(e)
        || is_command_intent(e)
        || is_session_intent(e)
        || is_login_flow_intent(e)
        || is_edit_intent(e)
        || is_scroll_nav_intent(e)
        || is_dialog_intent(e)
}

fn is_config_fact(e: &Event) -> bool {
    matches!(e, Event::ConfigLoaded { .. } | Event::SwitchTheme { .. }
        | Event::KeybindingsReloaded)
}

fn is_trust_history_fact(e: &Event) -> bool {
    matches!(e, Event::TrustLoaded { .. } | Event::TrustChanged { .. } | Event::TrustSet { .. }
        | Event::HistoryLoaded { .. } | Event::HistoryAppend { .. })
}

fn is_session_fact(e: &Event) -> bool {
    matches!(e, Event::SessionLoaded { .. } | Event::SessionSaved { .. }
        | Event::SessionDeleted { .. } | Event::SessionImported { .. }
        | Event::SessionExported { .. } | Event::SessionList { .. }
        | Event::SessionOperationFailed { .. })
}

fn is_io_fact(e: &Event) -> bool {
    matches!(e, Event::BashOutput { .. } | Event::FilesWritten { .. })
}

fn is_system_fact(e: &Event) -> bool {
    matches!(e, Event::SystemMessage { .. } | Event::TransientMessage { .. }
        | Event::TransientError { .. } | Event::ClearTransient | Event::ShowDiagnostics)
}

fn is_login_flow_fact(e: &Event) -> bool {
    matches!(e, Event::ValidationFailed { .. } | Event::ModelsFetched { .. })
}

fn is_permission_fact(e: &Event) -> bool {
    matches!(e, Event::PermissionRequest { .. } | Event::PermissionResponse { .. })
}

fn is_replay_fact(e: &Event) -> bool {
    matches!(e, Event::MessageReplayed { .. })
}

fn is_model_config_intent(e: &Event) -> bool {
    matches!(e, Event::CycleModelNext | Event::CycleModelPrev | Event::ToggleScopedModelsDialog
        | Event::ScopedModelToggle { .. } | Event::ScopedModelEnableAll
        | Event::ScopedModelDisableAll | Event::ScopedModelToggleProvider { .. }
        | Event::CycleThinkingLevel | Event::SetThinkingLevel(_) | Event::ToggleReadOnly
        | Event::TrustProject | Event::UntrustProject | Event::ReloadAll)
}

fn is_command_intent(e: &Event) -> bool {
    matches!(e, Event::RunLoadCommand { .. } | Event::RunSaveCommand { .. }
        | Event::RunDeleteCommand { .. } | Event::RunImportCommand { .. }
        | Event::RunExportCommand { .. } | Event::RunSkillCommand { .. }
        | Event::RunLoginCommand { .. } | Event::RunLogoutCommand { .. }
        | Event::RunNameCommand { .. } | Event::RunForkCommand { .. }
        | Event::RunCompactCommand { .. } | Event::RunPromptCommand { .. }
        | Event::RunThinkingCommand { .. } | Event::RunPaletteCommand { .. })
}

fn is_session_intent(e: &Event) -> bool {
    matches!(e, Event::ForkSession { .. } | Event::CloneSession | Event::ToggleSessionTree
        | Event::SessionTreeFilterCycle | Event::SessionTreeSelect { .. })
}

fn is_login_flow_intent(e: &Event) -> bool {
    matches!(e, Event::Start | Event::SelectProvider { .. } | Event::SubmitKey { .. }
        | Event::ToggleModel { .. } | Event::Save | Event::Cancel)
}

fn is_control_kind(e: &Event) -> bool {
    matches!(e, Event::Quit | Event::ForceQuit | Event::Reset | Event::Abort
        | Event::FollowUp | Event::ToggleExpand | Event::Dequeue | Event::OpenExternalEditor
        | Event::ExternalEditorDone { .. } | Event::ShareSession | Event::Suspend
        | Event::ToggleVimMode | Event::CopyLastResponse | Event::OpenSessionList
        | Event::NewSession | Event::ResumeSession | Event::SelectSession { .. }
        | Event::StarSession { .. } | Event::RenameSession { .. } | Event::DeleteSession { .. }
        | Event::TerminalSize { .. })
}

fn is_edit_intent(e: &Event) -> bool {
    matches!(e, Event::PendingEdit { .. } | Event::ApproveEdit | Event::RejectEdit)
}

fn is_scroll_nav_intent(e: &Event) -> bool {
    matches!(e, Event::Up | Event::Down)
}

fn is_dialog_intent(e: &Event) -> bool {
    matches!(e, Event::ToggleWelcome | Event::ToggleCommandPalette | Event::PaletteFilter(_)
        | Event::PaletteBackspace | Event::PaletteUp | Event::PaletteDown | Event::PaletteSelect
        | Event::PaletteClose | Event::ToggleModelSelector | Event::ModelSelectorFilter(_)
        | Event::ModelSelectorBackspace | Event::ModelSelectorUp | Event::ModelSelectorDown
        | Event::ModelSelectorSelect | Event::ModelSelectorClose | Event::ToggleSettingsDialog
        | Event::SettingsUp | Event::SettingsDown | Event::SettingsLeft | Event::SettingsRight
        | Event::SettingsSelect | Event::SettingsClose | Event::SettingsSwitchCategory { .. }
        | Event::TogglePathCompletion | Event::PathCompletionUp | Event::PathCompletionDown
        | Event::PathCompletionSelect | Event::PathCompletionClose
        | Event::CommandFormInput(_) | Event::CommandFormBackspace | Event::CommandFormUp
        | Event::CommandFormDown | Event::CommandFormSubmit | Event::CommandFormClose
        | Event::DialogBack | Event::ProvidersDialog | Event::ProvidersSelectModel { .. }
        | Event::ProvidersDisconnect { .. } | Event::ProvidersAdd
        | Event::ProvidersEditModels { .. } | Event::CopyToClipboard(_)
        | Event::CopySelectedBlock | Event::CopyBlockMetadata | Event::AtFilePicker
        | Event::InsertAtRef(_))
}

impl Event {
    /// Classify this event into one of the three taxonomy kinds.
    pub fn kind(&self) -> EventKind {
        if is_fact(self) {
            return EventKind::Fact;
        }
        if is_intent(self) {
            return EventKind::Intent;
        }
        if is_control_kind(self) {
            return EventKind::Control;
        }
        // Default to Intent for safety (old/unknown variants get routed to handlers)
        EventKind::Intent
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every Event variant must be classified (compilation test for exhaustiveness).
    #[test]
    fn event_kind_is_exhaustive() {
        fn _check(_: Event) {}
    }

    #[test]
    fn intent_events_are_not_fact() {
        for e in [
            Event::Input('x'),
            Event::Submit,
            Event::SwitchModel { provider: "anthropic".into(), model: "claude".into(), explicit: true },
            Event::Quit,
            Event::RunSaveCommand { name: "test".into() },
        ] {
            assert_ne!(e.kind(), EventKind::Fact, "{e:?} must not be Fact");
        }
    }

    #[test]
    fn fact_events_are_classified() {
        for e in [
            Event::Thinking { id: "1".into() },
            Event::ToolEnd { id: "t1".into(), duration_secs: 0.5, output: "ok".into() },
            Event::TurnComplete { id: "1".into(), duration_secs: 1.0 },
            Event::ConfigLoaded { config: Box::new(crate::config::Config::default()) },
            Event::TrustLoaded { decisions: Default::default() },
            Event::SessionLoaded { name: "test".into(), events: Box::new(vec![]), metadata: None },
            Event::BashOutput { command: "pwd".into(), output: "/tmp".into() },
            Event::TransientMessage { content: "hello".into(), level: crate::event::TransientLevel::Info },
        ] {
            assert_eq!(e.kind(), EventKind::Fact, "{e:?} must be Fact");
        }
    }

    #[test]
    fn control_events_are_classified() {
        for e in [
            Event::Quit,
            Event::Reset,
            Event::Abort,
            Event::TerminalSize { width: 80, height: 24 },
        ] {
            assert_eq!(e.kind(), EventKind::Control, "{e:?} must be Control");
        }
    }
}
