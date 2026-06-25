//! `Event::into_intent()` implementation.
//!
//! Split from [`variants`](super::variants) to keep files under the 500-line limit.
//! Each helper is ≤ 40 lines and has low complexity.

use super::intent::Intent;
use super::variants::Event;

// ── Category helpers (each ≤ 40 lines, low complexity) ────────────────────────

fn config_intent(e: &Event) -> Option<Intent> {
    use Intent as I;
    match e {
        Event::SwitchTheme { name } => Some(I::SetTheme { name: name.clone() }),
        Event::ReloadAll => Some(I::ReloadConfig),
        _ => None,
    }
}

fn trust_intent(e: &Event) -> Option<Intent> {
    use Intent as I;
    match e {
        Event::TrustProject => Some(I::TrustProject),
        Event::UntrustProject => Some(I::UntrustProject),
        _ => None,
    }
}

fn edit_intent(e: &Event) -> Option<Intent> {
    use Intent as I;
    match e {
        Event::PendingEdit { path, original, proposed } => {
            Some(I::PendingEdit { path: path.clone(), original: original.clone(), proposed: proposed.clone() })
        }
        Event::ApproveEdit => Some(I::ApproveEdit),
        Event::RejectEdit => Some(I::RejectEdit),
        _ => None,
    }
}

fn system_scroll_intent(e: &Event) -> Option<Intent> {
    use Intent as I;
    use super::TransientLevel;
    match e {
        Event::TransientMessage { content, level } => {
            Some(I::Notify { content: content.clone(), level: *level })
        }
        Event::TransientError { content } => {
            Some(I::Notify { content: content.clone(), level: TransientLevel::Error })
        }
        Event::ClearTransient => Some(I::ClearTransient),
        Event::ShowDiagnostics => Some(I::ShowDiagnostics),
        Event::Up => Some(I::ScrollUp),
        Event::Down => Some(I::ScrollDown),
        _ => None,
    }
}

fn control_intent(e: &Event) -> Option<Intent> {
    use Intent as I;
    match e {
        Event::Quit => Some(I::Quit),
        Event::ForceQuit => Some(I::ForceQuit),
        Event::Reset => Some(I::Reset),
        Event::Abort => Some(I::Abort),
        Event::FollowUp => Some(I::FollowUp),
        Event::ToggleExpand => Some(I::ToggleExpand),
        Event::Dequeue => Some(I::Dequeue),
        Event::OpenExternalEditor => Some(I::OpenExternalEditor),
        Event::ExternalEditorDone { content } => {
            Some(I::ExternalEditorDone { content: content.clone() })
        }
        Event::ShareSession => Some(I::ShareSession),
        Event::Suspend => Some(I::Suspend),
        Event::ToggleVimMode => Some(I::ToggleVimMode),
        Event::CopyLastResponse => Some(I::CopyLastResponse),
        Event::OpenSessionList => Some(I::OpenSessionList),
        Event::NewSession => Some(I::NewSession),
        Event::ResumeSession => Some(I::ResumeSession),
        Event::SelectSession { id } => Some(I::SelectSession { id: id.clone() }),
        Event::StarSession { id } => Some(I::StarSession { id: id.clone() }),
        Event::RenameSession { id, name } => {
            Some(I::RenameSession { id: id.clone(), name: name.clone() })
        }
        Event::DeleteSession { id } => Some(I::DeleteSession { id: id.clone() }),
        _ => None,
    }
}

fn model_config_intent(e: &Event) -> Option<Intent> {
    use Intent as I;
    match e {
        Event::SwitchModel { provider, model, explicit } => {
            Some(I::SwitchModel { provider: provider.clone(), model: model.clone(), explicit: *explicit })
        }
        Event::CycleModelNext => Some(I::CycleModelNext),
        Event::CycleModelPrev => Some(I::CycleModelPrev),
        Event::ToggleScopedModelsDialog => Some(I::ToggleScopedModelsDialog),
        Event::ScopedModelToggle { provider, name } => {
            Some(I::ScopedModelToggle { provider: provider.clone(), name: name.clone() })
        }
        Event::ScopedModelEnableAll => Some(I::ScopedModelEnableAll),
        Event::ScopedModelDisableAll => Some(I::ScopedModelDisableAll),
        Event::ScopedModelToggleProvider { provider } => {
            Some(I::ScopedModelToggleProvider { provider: provider.clone() })
        }
        Event::ToggleSettingsDialog => Some(I::ToggleSettingsDialog),
        Event::SettingsUp => Some(I::SettingsUp),
        Event::SettingsDown => Some(I::SettingsDown),
        Event::SettingsLeft => Some(I::SettingsLeft),
        Event::SettingsRight => Some(I::SettingsRight),
        Event::SettingsSelect => Some(I::SettingsSelect),
        Event::SettingsClose => Some(I::SettingsClose),
        Event::SettingsSwitchCategory { category } => {
            Some(I::SettingsSwitchCategory { category: *category })
        }
        Event::CycleThinkingLevel => Some(I::CycleThinkingLevel),
        Event::SetThinkingLevel(lvl) => Some(I::SetThinkingLevel(*lvl)),
        Event::ToggleReadOnly => Some(I::ToggleReadOnly),
        _ => None,
    }
}

fn dialog_intent_a(e: &Event) -> Option<Intent> {
    use Intent as I;
    match e {
        Event::ToggleWelcome => Some(I::ToggleWelcome),
        Event::ToggleCommandPalette => Some(I::ToggleCommandPalette),
        Event::PaletteFilter(c) => Some(I::PaletteFilter(*c)),
        Event::PaletteBackspace => Some(I::PaletteBackspace),
        Event::PaletteUp => Some(I::PaletteUp),
        Event::PaletteDown => Some(I::PaletteDown),
        Event::PaletteSelect => Some(I::PaletteSelect),
        Event::PaletteClose => Some(I::PaletteClose),
        Event::ToggleModelSelector => Some(I::ToggleModelSelector),
        Event::ModelSelectorFilter(c) => Some(I::ModelSelectorFilter(*c)),
        Event::ModelSelectorBackspace => Some(I::ModelSelectorBackspace),
        Event::ModelSelectorUp => Some(I::ModelSelectorUp),
        Event::ModelSelectorDown => Some(I::ModelSelectorDown),
        Event::ModelSelectorSelect => Some(I::ModelSelectorSelect),
        Event::ModelSelectorClose => Some(I::ModelSelectorClose),
        Event::TogglePathCompletion => Some(I::TogglePathCompletion),
        Event::PathCompletionUp => Some(I::PathCompletionUp),
        _ => None,
    }
}

fn dialog_intent_b(e: &Event) -> Option<Intent> {
    use Intent as I;
    match e {
        Event::PathCompletionDown => Some(I::PathCompletionDown),
        Event::PathCompletionSelect => Some(I::PathCompletionSelect),
        Event::PathCompletionClose => Some(I::PathCompletionClose),
        Event::CommandFormInput(c) => Some(I::CommandFormInput(*c)),
        Event::CommandFormBackspace => Some(I::CommandFormBackspace),
        Event::CommandFormUp => Some(I::CommandFormUp),
        Event::CommandFormDown => Some(I::CommandFormDown),
        Event::CommandFormSubmit => Some(I::CommandFormSubmit),
        Event::CommandFormClose => Some(I::CommandFormClose),
        Event::DialogBack => Some(I::DialogBack),
        Event::ProvidersDialog => Some(I::ProvidersDialog),
        Event::ProvidersSelectModel { provider, model } => {
            Some(I::ProvidersSelectModel { provider: provider.clone(), model: model.clone() })
        }
        Event::ProvidersDisconnect { provider } => {
            Some(I::ProvidersDisconnect { provider: provider.clone() })
        }
        Event::ProvidersAdd => Some(I::ProvidersAdd),
        Event::ProvidersEditModels { provider } => {
            Some(I::ProvidersEditModels { provider: provider.clone() })
        }
        Event::CopyToClipboard(s) => Some(I::CopyToClipboard(s.clone())),
        Event::CopySelectedBlock => Some(I::CopySelectedBlock),
        Event::CopyBlockMetadata => Some(I::CopyBlockMetadata),
        Event::AtFilePicker => Some(I::AtFilePicker),
        Event::InsertAtRef(s) => Some(I::InsertAtRef(s.clone())),
        _ => None,
    }
}

fn command_intent(e: &Event) -> Option<Intent> {
    use Intent as I;
    match e {
        Event::RunLoadCommand { name } => Some(I::RunLoadCommand { name: name.clone() }),
        Event::RunSaveCommand { name } => Some(I::RunSaveCommand { name: name.clone() }),
        Event::RunDeleteCommand { name } => Some(I::RunDeleteCommand { name: name.clone() }),
        Event::RunImportCommand { path } => Some(I::RunImportCommand { path: path.clone() }),
        Event::RunExportCommand { path } => Some(I::RunExportCommand { path: path.clone() }),
        Event::RunSkillCommand { name } => Some(I::RunSkillCommand { name: name.clone() }),
        Event::RunLoginCommand { provider, token } => {
            Some(I::RunLoginCommand { provider: provider.clone(), token: token.clone() })
        }
        Event::RunLogoutCommand { provider } => {
            Some(I::RunLogoutCommand { provider: provider.clone() })
        }
        Event::RunNameCommand { name } => Some(I::RunNameCommand { name: name.clone() }),
        Event::RunForkCommand { message_index } => {
            Some(I::RunForkCommand { message_index: message_index.clone() })
        }
        Event::RunCompactCommand { keep, focus } => {
            Some(I::RunCompactCommand { keep: keep.clone(), focus: focus.clone() })
        }
        Event::RunPromptCommand { name } => Some(I::RunPromptCommand { name: name.clone() }),
        Event::RunThinkingCommand { level } => Some(I::RunThinkingCommand { level: *level }),
        Event::RunPaletteCommand { name, args } => {
            Some(I::RunPaletteCommand { name: name.clone(), args: args.clone() })
        }
        _ => None,
    }
}

fn login_session_intent(e: &Event) -> Option<Intent> {
    use Intent as I;
    match e {
        Event::Start => Some(I::LoginStart),
        Event::SelectProvider { provider } => Some(I::SelectProvider { provider: provider.clone() }),
        Event::SubmitKey { provider, key } => {
            Some(I::SubmitKey { provider: provider.clone(), key: key.clone() })
        }
        Event::ToggleModel { model } => Some(I::ToggleModel { model: model.clone() }),
        Event::Save => Some(I::LoginSave),
        Event::Cancel => Some(I::LoginCancel),
        Event::ForkSession { message_index } => {
            Some(I::ForkSession { message_index: *message_index })
        }
        Event::CloneSession => Some(I::CloneSession),
        Event::ToggleSessionTree => Some(I::ToggleSessionTree),
        Event::SessionTreeFilterCycle => Some(I::SessionTreeFilterCycle),
        Event::SessionTreeSelect { id } => Some(I::SessionTreeSelect { id: id.clone() }),
        _ => None,
    }
}

fn input_intent_a(e: &Event) -> Option<Intent> {
    use Intent as I;
    match e {
        Event::Input(c) => Some(I::Input(*c)),
        Event::Backspace => Some(I::Backspace),
        Event::Newline => Some(I::Newline),
        Event::Submit => Some(I::Submit),
        Event::Escape => Some(I::Escape),
        Event::CursorLeft => Some(I::CursorLeft),
        Event::CursorRight => Some(I::CursorRight),
        Event::CursorStart => Some(I::CursorStart),
        Event::CursorEnd => Some(I::CursorEnd),
        Event::DeleteWord => Some(I::DeleteWord),
        Event::DeleteToEnd => Some(I::DeleteToEnd),
        Event::DeleteToStart => Some(I::DeleteToStart),
        Event::KillChar => Some(I::KillChar),
        Event::HistoryPrev => Some(I::HistoryPrev),
        Event::HistoryNext => Some(I::HistoryNext),
        Event::Undo => Some(I::Undo),
        Event::Redo => Some(I::Redo),
        Event::CursorWordLeft => Some(I::CursorWordLeft),
        Event::CursorWordRight => Some(I::CursorWordRight),
        _ => None,
    }
}

fn input_intent_b(e: &Event) -> Option<Intent> {
    use Intent as I;
    match e {
        Event::PageUp => Some(I::PageUp),
        Event::PageDown => Some(I::PageDown),
        Event::GoToTop => Some(I::GoToTop),
        Event::GoToBottom => Some(I::GoToBottom),
        Event::Paste(s) => Some(I::Paste(s.clone())),
        Event::PasteImage => Some(I::PasteImage),
        Event::MouseClick { row, col, button } => {
            Some(I::MouseClick { row: *row, col: *col, button: button.clone() })
        }
        Event::MouseRelease { row, col, button } => {
            Some(I::MouseRelease { row: *row, col: *col, button: button.clone() })
        }
        Event::MouseDrag { row, col, button } => {
            Some(I::MouseDrag { row: *row, col: *col, button: button.clone() })
        }
        Event::MouseMove { row, col } => Some(I::MouseMove { row: *row, col: *col }),
        Event::MouseScrollUp => Some(I::MouseScrollUp),
        Event::MouseScrollDown => Some(I::MouseScrollDown),
        Event::FocusGained => Some(I::FocusGained),
        Event::FocusLost => Some(I::FocusLost),
        Event::TerminalSize { width, height } => {
            Some(I::TerminalSize { width: *width, height: *height })
        }
        _ => None,
    }
}

fn permission_intent(e: &Event) -> Option<Intent> {
    use Intent as I;
    match e {
        Event::PermissionResponse { request_id, action } => {
            Some(I::PermissionResponse { request_id: request_id.clone(), action: *action })
        }
        _ => None,
    }
}

// ── Top-level dispatcher (nested to keep top-level complexity low) ─────────────

fn group_a(e: &Event) -> Option<Intent> {
    config_intent(e)
        .or_else(|| trust_intent(e))
        .or_else(|| edit_intent(e))
        .or_else(|| system_scroll_intent(e))
        .or_else(|| control_intent(e))
}

fn group_b(e: &Event) -> Option<Intent> {
    model_config_intent(e)
        .or_else(|| dialog_intent_a(e))
        .or_else(|| dialog_intent_b(e))
        .or_else(|| command_intent(e))
        .or_else(|| login_session_intent(e))
}

fn group_c(e: &Event) -> Option<Intent> {
    input_intent_a(e).or_else(|| input_intent_b(e)).or_else(|| permission_intent(e))
}

fn try_intent_helpers(e: &Event) -> Option<Intent> {
    group_a(e).or_else(|| group_b(e)).or_else(|| group_c(e))
}

// ── Fact sentinel (all non-intent events) ─────────────────────────────────────

/// Returns true if this event is a Fact (not an Intent).
/// Listed explicitly so the compiler catches new Fact variants that aren't handled.
fn is_fact_variant(e: &Event) -> bool {
    matches!(
        e,
        Event::Thinking { .. }
            | Event::ThoughtDone { .. }
            | Event::ToolStart { .. }
            | Event::ToolEnd { .. }
            | Event::ResponseDelta { .. }
            | Event::ThinkingDelta { .. }
            | Event::TextStart { .. }
            | Event::TextEnd { .. }
            | Event::ThinkingStart { .. }
            | Event::ThinkingEnd { .. }
            | Event::Response { .. }
            | Event::TurnComplete { .. }
            | Event::Done { .. }
            | Event::Error { .. }
            | Event::AssistantMessageReady { .. }
            | Event::MessageReplayed { .. }
            | Event::ConfigLoaded { .. }
            | Event::TrustLoaded { .. }
            | Event::TrustChanged { .. }
            | Event::TrustSet { .. }
            | Event::HistoryLoaded { .. }
            | Event::HistoryAppend { .. }
            | Event::SessionLoaded { .. }
            | Event::SessionSaved { .. }
            | Event::SessionDeleted { .. }
            | Event::SessionImported { .. }
            | Event::SessionExported { .. }
            | Event::SessionList { .. }
            | Event::SessionOperationFailed { .. }
            | Event::BashOutput { .. }
            | Event::FilesWritten { .. }
            | Event::SystemMessage { .. }
            | Event::ValidationFailed { .. }
            | Event::ModelsFetched { .. }
            | Event::PermissionRequest { .. }
    )
}

// ── Public impl (delegates to helpers) ────────────────────────────────────────

impl Event {
    /// Convert this event to a typed `Intent`, if it is an intent.
    ///
    /// Returns `None` for `Fact` and `Control` events — those have their
    /// own routing paths and do not convert to `Intent`.
    ///
    /// Use `Event::kind()` to determine the event category first if you
    /// need to distinguish between all three categories.
    pub fn into_intent(self) -> Option<Intent> {
        if is_fact_variant(&self) {
            return None;
        }
        try_intent_helpers(&self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fact_events_return_none() {
        for e in [
            Event::ConfigLoaded {
                config: Box::new(crate::config::Config::default()),
            },
            Event::TrustLoaded {
                decisions: Default::default(),
            },
            Event::SessionSaved { name: "test".into() },
            Event::BashOutput { command: "ls".into(), output: "/".into() },
        ] {
            assert!(
                e.clone().into_intent().is_none(),
                "{e:?} must return None"
            );
        }
    }

    #[test]
    fn intent_events_return_some() {
        let e = Event::SwitchTheme { name: "dark".into() };
        let i = e.into_intent().expect("SwitchTheme must convert to Intent");
        assert!(matches!(i, Intent::SetTheme { .. }));

        let e = Event::Quit;
        assert!(e.into_intent().is_some(), "Quit must convert to Intent");

        let e = Event::Submit;
        assert!(e.into_intent().is_some(), "Submit must convert to Intent");
    }

    #[test]
    fn switch_theme_is_intent_not_fact() {
        let e = Event::SwitchTheme { name: "dracula".into() };
        assert_eq!(e.kind(), super::super::kind::EventKind::Intent);
        assert!(e.into_intent().is_some());
    }
}
