//! Canonical event name mapping for bindable variants.

use super::Event;
use crate::event::EVENT_NAMES;

impl Event {
    /// Canonical string name for bindable variants (those in EVENT_NAMES).
    pub fn name(&self) -> Option<&'static str> {
        match self {
            Event::Backspace
            | Event::Newline
            | Event::Submit
            | Event::Escape
            | Event::CursorLeft
            | Event::CursorRight
            | Event::CursorStart
            | Event::CursorEnd
            | Event::DeleteWord
            | Event::DeleteToEnd
            | Event::DeleteToStart
            | Event::KillChar
            | Event::HistoryPrev
            | Event::HistoryNext
            | Event::Undo
            | Event::Redo
            | Event::CursorWordLeft
            | Event::CursorWordRight
            | Event::PageUp
            | Event::PageDown
            | Event::GoToTop
            | Event::GoToBottom
            | Event::PasteImage
            | Event::FocusGained
            | Event::FocusLost
            | Event::Quit
            | Event::Reset
            | Event::Abort
            | Event::FollowUp
            | Event::ToggleExpand
            | Event::Dequeue
            | Event::OpenExternalEditor
            | Event::Suspend
            | Event::ShareSession
            | Event::ToggleVimMode
            | Event::CopyLastResponse
            | Event::OpenSessionList
            | Event::NewSession
            | Event::ResumeSession
            | Event::CopySelectedBlock
            | Event::CopyBlockMetadata
            | Event::ToggleCommandPalette
            | Event::PaletteBackspace
            | Event::PaletteUp
            | Event::PaletteDown
            | Event::PaletteSelect
            | Event::PaletteClose
            | Event::ToggleModelSelector
            | Event::ModelSelectorBackspace
            | Event::ModelSelectorUp
            | Event::ModelSelectorDown
            | Event::ModelSelectorSelect
            | Event::ModelSelectorClose
            | Event::ToggleSettingsDialog
            | Event::SettingsUp
            | Event::SettingsDown
            | Event::SettingsLeft
            | Event::SettingsRight
            | Event::SettingsSelect
            | Event::SettingsClose
            | Event::CommandFormBackspace
            | Event::CommandFormUp
            | Event::CommandFormDown
            | Event::CommandFormSubmit
            | Event::CommandFormClose
            | Event::ToggleScopedModelsDialog
            | Event::ScopedModelEnableAll
            | Event::ScopedModelDisableAll
            | Event::DialogBack
            | Event::TogglePathCompletion
            | Event::PathCompletionUp
            | Event::PathCompletionDown
            | Event::PathCompletionSelect
            | Event::PathCompletionClose
            | Event::ProvidersDialog
            | Event::ProvidersAdd
            | Event::AtFilePicker
            | Event::CycleModelNext
            | Event::CycleModelPrev
            | Event::CycleThinkingLevel
            | Event::ToggleReadOnly
            | Event::TrustProject
            | Event::UntrustProject
            | Event::OpenAgentsManager
            | Event::ClearTransient => Some(<&str>::from(self.clone())),
            _ => None,
        }
    }

    /// Build an Event from its canonical name. Supports `Input:<char>` prefix.
    pub fn from_name(name: &str) -> Option<Event> {
        if let Some(rest) = name.strip_prefix("Input:") {
            let c = rest.chars().next()?;
            return Some(Event::Input(c));
        }
        for (n, ctor) in EVENT_NAMES {
            if *n == name {
                return Some(ctor());
            }
        }
        None
    }
}
