//! `Event::into_intent()` implementation.
//! Generated from `taxonomy.json`. DO NOT EDIT.

use super::super::intent::Intent;
use super::super::variants::Event;
use super::facts::is_fact_variant;

impl Event {
    /// Convert this event to a typed `Intent`, if it is an intent.
    pub fn into_intent(self) -> Option<Intent> {
        if is_fact_variant(&self) {
            return None;
        }
        match self {
            Event::RunLoadCommand { name } => Some(Intent::RunLoadCommand { name: name.clone() }),
            Event::RunSaveCommand { name } => Some(Intent::RunSaveCommand { name: name.clone() }),
            Event::RunDeleteCommand { name } => {
                Some(Intent::RunDeleteCommand { name: name.clone() })
            }
            Event::RunImportCommand { path } => {
                Some(Intent::RunImportCommand { path: path.clone() })
            }
            Event::RunExportCommand { path } => {
                Some(Intent::RunExportCommand { path: path.clone() })
            }
            Event::RunSkillCommand { name } => Some(Intent::RunSkillCommand { name: name.clone() }),
            Event::RunLoginCommand { provider, token } => Some(Intent::RunLoginCommand {
                provider: provider.clone(),
                token: token.clone(),
            }),
            Event::RunLogoutCommand { provider } => Some(Intent::RunLogoutCommand {
                provider: provider.clone(),
            }),
            Event::RunNameCommand { name } => Some(Intent::RunNameCommand { name: name.clone() }),
            Event::RunForkCommand { message_index } => Some(Intent::RunForkCommand {
                message_index: message_index.clone(),
            }),
            Event::RunCompactCommand { keep, focus } => Some(Intent::RunCompactCommand {
                keep: keep.clone(),
                focus: focus.clone(),
            }),
            Event::RunPromptCommand { name } => {
                Some(Intent::RunPromptCommand { name: name.clone() })
            }
            Event::RunThinkingCommand { level } => Some(Intent::RunThinkingCommand { level }),
            Event::RunPaletteCommand { name, args } => Some(Intent::RunPaletteCommand {
                name: name.clone(),
                args: args.clone(),
            }),
            Event::Quit => Some(Intent::Quit),
            Event::ForceQuit => Some(Intent::ForceQuit),
            Event::Reset => Some(Intent::Reset),
            Event::Abort => Some(Intent::Abort),
            Event::ClearQueues => Some(Intent::ClearQueues),
            Event::FollowUp => Some(Intent::FollowUp),
            Event::ToggleExpand => Some(Intent::ToggleExpand),
            Event::Dequeue => Some(Intent::Dequeue),
            Event::OpenExternalEditor => Some(Intent::OpenExternalEditor),
            Event::ExternalEditorDone { content } => Some(Intent::ExternalEditorDone {
                content: content.clone(),
            }),
            Event::ShareSession => Some(Intent::ShareSession),
            Event::Suspend => Some(Intent::Suspend),
            Event::ToggleVimMode => Some(Intent::ToggleVimMode),
            Event::CopyLastResponse => Some(Intent::CopyLastResponse),
            Event::OpenSessionList => Some(Intent::OpenSessionList),
            Event::NewSession => Some(Intent::NewSession),
            Event::ResumeSession => Some(Intent::ResumeSession),
            Event::SelectSession { id } => Some(Intent::SelectSession { id: id.clone() }),
            Event::StarSession { id } => Some(Intent::StarSession { id: id.clone() }),
            Event::RenameSession { id, name } => Some(Intent::RenameSession {
                id: id.clone(),
                name: name.clone(),
            }),
            Event::DeleteSession { id } => Some(Intent::DeleteSession { id: id.clone() }),
            Event::ToggleWelcome => Some(Intent::ToggleWelcome),
            Event::ToggleCommandPalette => Some(Intent::ToggleCommandPalette),
            Event::PaletteFilter(c) => Some(Intent::PaletteFilter(c)),
            Event::PaletteBackspace => Some(Intent::PaletteBackspace),
            Event::PaletteUp => Some(Intent::PaletteUp),
            Event::PaletteDown => Some(Intent::PaletteDown),
            Event::PaletteSelect => Some(Intent::PaletteSelect),
            Event::PaletteClose => Some(Intent::PaletteClose),
            Event::ToggleModelSelector => Some(Intent::ToggleModelSelector),
            Event::ModelSelectorFilter(c) => Some(Intent::ModelSelectorFilter(c)),
            Event::ModelSelectorBackspace => Some(Intent::ModelSelectorBackspace),
            Event::ModelSelectorUp => Some(Intent::ModelSelectorUp),
            Event::ModelSelectorDown => Some(Intent::ModelSelectorDown),
            Event::ModelSelectorSelect => Some(Intent::ModelSelectorSelect),
            Event::ModelSelectorClose => Some(Intent::ModelSelectorClose),
            Event::TogglePathCompletion => Some(Intent::TogglePathCompletion),
            Event::PathCompletionUp => Some(Intent::PathCompletionUp),
            Event::PathCompletionDown => Some(Intent::PathCompletionDown),
            Event::PathCompletionSelect => Some(Intent::PathCompletionSelect),
            Event::PathCompletionClose => Some(Intent::PathCompletionClose),
            Event::CommandFormInput(c) => Some(Intent::CommandFormInput(c)),
            Event::CommandFormBackspace => Some(Intent::CommandFormBackspace),
            Event::CommandFormUp => Some(Intent::CommandFormUp),
            Event::CommandFormDown => Some(Intent::CommandFormDown),
            Event::CommandFormSubmit => Some(Intent::CommandFormSubmit),
            Event::CommandFormClose => Some(Intent::CommandFormClose),
            Event::DialogBack => Some(Intent::DialogBack),
            Event::ProvidersDialog => Some(Intent::ProvidersDialog),
            Event::ProvidersSelectModel { provider, model } => Some(Intent::ProvidersSelectModel {
                provider: provider.clone(),
                model: model.clone(),
            }),
            Event::ProvidersDisconnect { provider } => Some(Intent::ProvidersDisconnect {
                provider: provider.clone(),
            }),
            Event::ProvidersAdd => Some(Intent::ProvidersAdd),
            Event::ProvidersEditModels { provider } => Some(Intent::ProvidersEditModels {
                provider: provider.clone(),
            }),
            Event::CopyToClipboard(s) => Some(Intent::CopyToClipboard(s)),
            Event::CopySelectedBlock => Some(Intent::CopySelectedBlock),
            Event::CopyBlockMetadata => Some(Intent::CopyBlockMetadata),
            Event::AtFilePicker => Some(Intent::AtFilePicker),
            Event::InsertAtRef(s) => Some(Intent::InsertAtRef(s)),
            Event::PendingEdit {
                path,
                original,
                proposed,
            } => Some(Intent::PendingEdit {
                path: path.clone(),
                original: original.clone(),
                proposed: proposed.clone(),
            }),
            Event::ApproveEdit => Some(Intent::ApproveEdit),
            Event::RejectEdit => Some(Intent::RejectEdit),
            Event::Input(c) => Some(Intent::Input(c)),
            Event::Backspace => Some(Intent::Backspace),
            Event::Newline => Some(Intent::Newline),
            Event::Submit => Some(Intent::Submit),
            Event::Escape => Some(Intent::Escape),
            Event::CursorLeft => Some(Intent::CursorLeft),
            Event::CursorRight => Some(Intent::CursorRight),
            Event::CursorStart => Some(Intent::CursorStart),
            Event::CursorEnd => Some(Intent::CursorEnd),
            Event::DeleteWord => Some(Intent::DeleteWord),
            Event::DeleteToEnd => Some(Intent::DeleteToEnd),
            Event::DeleteToStart => Some(Intent::DeleteToStart),
            Event::KillChar => Some(Intent::KillChar),
            Event::HistoryPrev => Some(Intent::HistoryPrev),
            Event::HistoryNext => Some(Intent::HistoryNext),
            Event::Undo => Some(Intent::Undo),
            Event::Redo => Some(Intent::Redo),
            Event::CursorWordLeft => Some(Intent::CursorWordLeft),
            Event::CursorWordRight => Some(Intent::CursorWordRight),
            Event::PageUp => Some(Intent::PageUp),
            Event::PageDown => Some(Intent::PageDown),
            Event::GoToTop => Some(Intent::GoToTop),
            Event::GoToBottom => Some(Intent::GoToBottom),
            Event::Paste(s) => Some(Intent::Paste(s)),
            Event::PasteImage => Some(Intent::PasteImage),
            Event::MouseClick {
                ref row,
                ref col,
                ref button,
            } => Some(Intent::MouseClick {
                row: *row,
                col: *col,
                button: button.clone(),
            }),
            Event::MouseRelease {
                ref row,
                ref col,
                ref button,
            } => Some(Intent::MouseRelease {
                row: *row,
                col: *col,
                button: button.clone(),
            }),
            Event::MouseDrag {
                ref row,
                ref col,
                ref button,
            } => Some(Intent::MouseDrag {
                row: *row,
                col: *col,
                button: button.clone(),
            }),
            Event::MouseMove { ref row, ref col } => Some(Intent::MouseMove {
                row: *row,
                col: *col,
            }),
            Event::MouseScrollUp => Some(Intent::MouseScrollUp),
            Event::MouseScrollDown => Some(Intent::MouseScrollDown),
            Event::FocusGained => Some(Intent::FocusGained),
            Event::FocusLost => Some(Intent::FocusLost),
            Event::TerminalSize {
                ref width,
                ref height,
            } => Some(Intent::TerminalSize {
                width: *width,
                height: *height,
            }),
            Event::Start => Some(Intent::LoginStart),
            Event::SelectProvider { provider } => Some(Intent::SelectProvider {
                provider: provider.clone(),
            }),
            Event::SubmitKey { provider, key } => Some(Intent::SubmitKey {
                provider: provider.clone(),
                key: key.clone(),
            }),
            Event::ToggleModel { model } => Some(Intent::ToggleModel {
                model: model.clone(),
            }),
            Event::Save => Some(Intent::LoginSave),
            Event::Cancel => Some(Intent::LoginCancel),
            Event::SwitchModel {
                ref provider,
                ref model,
                ref explicit,
            } => Some(Intent::SwitchModel {
                provider: (*provider).clone(),
                model: (*model).clone(),
                explicit: *explicit,
            }),
            Event::SwitchTheme { name } => Some(Intent::SetTheme { name: name.clone() }),
            Event::CycleModelNext => Some(Intent::CycleModelNext),
            Event::CycleModelPrev => Some(Intent::CycleModelPrev),
            Event::ToggleScopedModelsDialog => Some(Intent::ToggleScopedModelsDialog),
            Event::ScopedModelToggle { provider, name } => Some(Intent::ScopedModelToggle {
                provider: provider.clone(),
                name: name.clone(),
            }),
            Event::ScopedModelEnableAll => Some(Intent::ScopedModelEnableAll),
            Event::ScopedModelDisableAll => Some(Intent::ScopedModelDisableAll),
            Event::ScopedModelToggleProvider { provider } => {
                Some(Intent::ScopedModelToggleProvider {
                    provider: provider.clone(),
                })
            }
            Event::ToggleSettingsDialog => Some(Intent::ToggleSettingsDialog),
            Event::SettingsUp => Some(Intent::SettingsUp),
            Event::SettingsDown => Some(Intent::SettingsDown),
            Event::SettingsLeft => Some(Intent::SettingsLeft),
            Event::SettingsRight => Some(Intent::SettingsRight),
            Event::SettingsSelect => Some(Intent::SettingsSelect),
            Event::SettingsClose => Some(Intent::SettingsClose),
            Event::SettingsSwitchCategory { ref category } => {
                Some(Intent::SettingsSwitchCategory {
                    category: *category,
                })
            }
            Event::CycleThinkingLevel => Some(Intent::CycleThinkingLevel),
            Event::SetThinkingLevel(lvl) => Some(Intent::SetThinkingLevel(lvl)),
            Event::ToggleReadOnly => Some(Intent::ToggleReadOnly),
            Event::TrustProject => Some(Intent::TrustProject),
            Event::UntrustProject => Some(Intent::UntrustProject),
            Event::ReloadAll => Some(Intent::ReloadConfig),
            Event::PermissionResponse {
                ref request_id,
                ref action,
            } => Some(Intent::PermissionResponse {
                request_id: request_id.clone(),
                action: *action,
            }),
            Event::Up => Some(Intent::ScrollUp),
            Event::Down => Some(Intent::ScrollDown),
            Event::ForkSession { message_index } => Some(Intent::ForkSession { message_index }),
            Event::CloneSession => Some(Intent::CloneSession),
            Event::ToggleSessionTree => Some(Intent::ToggleSessionTree),
            Event::SessionTreeFilterCycle => Some(Intent::SessionTreeFilterCycle),
            Event::SessionTreeSelect { id } => Some(Intent::SessionTreeSelect { id: id.clone() }),
            _ => None,
        }
    }
}
