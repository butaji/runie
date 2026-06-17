//! Canonical event name mapping for bindable variants.

use super::Event;
use crate::event::EVENT_NAMES;

impl Event {
    /// Canonical string name for bindable variants (those in EVENT_NAMES).
    pub fn name(&self) -> Option<&'static str> {
        if is_named_input_variant(self)
            || is_named_dialog_variant(self)
            || is_named_system_variant(self)
        {
            return Some(<&str>::from(self.clone()));
        }
        None
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

fn is_named_input_variant(event: &Event) -> bool {
    matches!(
        event,
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
    )
}

fn is_named_dialog_variant(event: &Event) -> bool {
    is_named_palette_variant(event)
        || is_named_selector_variant(event)
        || is_named_form_variant(event)
        || matches!(
            event,
            Event::ToggleSettingsDialog
                | Event::SettingsUp
                | Event::SettingsDown
                | Event::SettingsLeft
                | Event::SettingsRight
                | Event::SettingsSelect
                | Event::SettingsClose
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
        )
}

fn is_named_palette_variant(event: &Event) -> bool {
    matches!(
        event,
        Event::ToggleCommandPalette
            | Event::PaletteBackspace
            | Event::PaletteUp
            | Event::PaletteDown
            | Event::PaletteSelect
            | Event::PaletteClose
    )
}

fn is_named_selector_variant(event: &Event) -> bool {
    matches!(
        event,
        Event::ToggleModelSelector
            | Event::ModelSelectorBackspace
            | Event::ModelSelectorUp
            | Event::ModelSelectorDown
            | Event::ModelSelectorSelect
            | Event::ModelSelectorClose
    )
}

fn is_named_form_variant(event: &Event) -> bool {
    matches!(
        event,
        Event::CommandFormBackspace
            | Event::CommandFormUp
            | Event::CommandFormDown
            | Event::CommandFormSubmit
            | Event::CommandFormClose
    )
}

fn is_named_system_variant(event: &Event) -> bool {
    matches!(
        event,
        Event::Quit
            | Event::ForceQuit
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
            | Event::CycleModelNext
            | Event::CycleModelPrev
            | Event::CycleThinkingLevel
            | Event::ToggleReadOnly
            | Event::TrustProject
            | Event::UntrustProject
            | Event::OpenAgentsManager
            | Event::ClearTransient
    )
}
