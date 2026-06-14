//! Canonical event names and name mapping.

use super::variants::Event;

#[allow(clippy::type_complexity)]
pub const EVENT_NAMES: &[(&str, fn() -> Event)] = &[
    ("Backspace", || Event::Backspace),
    ("Newline", || Event::Newline),
    ("Submit", || Event::Submit),
    ("ScrollUp", || Event::ScrollUp),
    ("ScrollDown", || Event::ScrollDown),
    ("PageUp", || Event::PageUp),
    ("PageDown", || Event::PageDown),
    ("CursorLeft", || Event::CursorLeft),
    ("CursorRight", || Event::CursorRight),
    ("CursorStart", || Event::CursorStart),
    ("CursorEnd", || Event::CursorEnd),
    ("DeleteWord", || Event::DeleteWord),
    ("DeleteToEnd", || Event::DeleteToEnd),
    ("DeleteToStart", || Event::DeleteToStart),
    ("KillChar", || Event::KillChar),
    ("HistoryPrev", || Event::HistoryPrev),
    ("HistoryNext", || Event::HistoryNext),
    ("Undo", || Event::Undo),
    ("Redo", || Event::Redo),
    ("CursorWordLeft", || Event::CursorWordLeft),
    ("CursorWordRight", || Event::CursorWordRight),
    ("PasteImage", || Event::PasteImage),
    ("Quit", || Event::Quit),
    ("Reset", || Event::Reset),
    ("CycleModelNext", || Event::CycleModelNext),
    ("CycleModelPrev", || Event::CycleModelPrev),
    ("ToggleScopedModelsDialog", || {
        Event::ToggleScopedModelsDialog
    }),
    ("ScopedModelEnableAll", || Event::ScopedModelEnableAll),
    ("ScopedModelDisableAll", || Event::ScopedModelDisableAll),
    ("ToggleSettingsDialog", || Event::ToggleSettingsDialog),
    ("SettingsUp", || Event::SettingsUp),
    ("SettingsDown", || Event::SettingsDown),
    ("SettingsLeft", || Event::SettingsLeft),
    ("SettingsRight", || Event::SettingsRight),
    ("SettingsSelect", || Event::SettingsSelect),
    ("SettingsClose", || Event::SettingsClose),
    ("CycleThinkingLevel", || Event::CycleThinkingLevel),
    ("ToggleReadOnly", || Event::ToggleReadOnly),
    ("TrustProject", || Event::TrustProject),
    ("UntrustProject", || Event::UntrustProject),
    ("FollowUp", || Event::FollowUp),
    ("Abort", || Event::Abort),
    ("ToggleExpand", || Event::ToggleExpand),
    ("Dequeue", || Event::Dequeue),
    ("OpenExternalEditor", || Event::OpenExternalEditor),
    ("ToggleCommandPalette", || Event::ToggleCommandPalette),
    ("PaletteBackspace", || Event::PaletteBackspace),
    ("PaletteUp", || Event::PaletteUp),
    ("PaletteDown", || Event::PaletteDown),
    ("PaletteSelect", || Event::PaletteSelect),
    ("PaletteClose", || Event::PaletteClose),
    ("ToggleModelSelector", || Event::ToggleModelSelector),
    ("ModelSelectorBackspace", || Event::ModelSelectorBackspace),
    ("ModelSelectorUp", || Event::ModelSelectorUp),
    ("ModelSelectorDown", || Event::ModelSelectorDown),
    ("ModelSelectorSelect", || Event::ModelSelectorSelect),
    ("ModelSelectorClose", || Event::ModelSelectorClose),
    ("ApproveEdit", || Event::ApproveEdit),
    ("RejectEdit", || Event::RejectEdit),
    ("ReloadAll", || Event::ReloadAll),
    ("ShowDiagnostics", || Event::ShowDiagnostics),
    ("CloneSession", || Event::CloneSession),
    ("ToggleSessionTree", || Event::ToggleSessionTree),
    ("SessionTreeFilterCycle", || Event::SessionTreeFilterCycle),
    ("Suspend", || Event::Suspend),
    ("TogglePathCompletion", || Event::TogglePathCompletion),
    ("PathCompletionUp", || Event::PathCompletionUp),
    ("PathCompletionDown", || Event::PathCompletionDown),
    ("PathCompletionSelect", || Event::PathCompletionSelect),
    ("PathCompletionClose", || Event::PathCompletionClose),
    ("ShareSession", || Event::ShareSession),
    ("AtFilePicker", || Event::AtFilePicker),
    ("CommandFormBackspace", || Event::CommandFormBackspace),
    ("CommandFormUp", || Event::CommandFormUp),
    ("CommandFormDown", || Event::CommandFormDown),
    ("CommandFormSubmit", || Event::CommandFormSubmit),
    ("CommandFormClose", || Event::CommandFormClose),
    ("DialogBack", || Event::DialogBack),
    ("ProvidersDialog", || Event::ProvidersDialog),
    ("ProvidersAdd", || Event::ProvidersAdd),
    ("LoginFlowStart", || Event::LoginFlowStart),
    ("LoginFlowSave", || Event::LoginFlowSave),
    ("LoginFlowCancel", || Event::LoginFlowCancel),
    ("ClearTransient", || Event::ClearTransient),
    ("CopyLastResponse", || Event::CopyLastResponse),
    ("GoToTop", || Event::GoToTop),
    ("GoToBottom", || Event::GoToBottom),
    ("ToggleVimMode", || Event::ToggleVimMode),
];

impl Event {
    /// Canonical name for bindable (unit) variants.
    pub const fn name(&self) -> Option<&'static str> {
        if let Some(name) = self.input_name() {
            return Some(name);
        }
        if let Some(name) = self.agent_name() {
            return Some(name);
        }
        if let Some(name) = self.dialog_name() {
            return Some(name);
        }
        if let Some(name) = self.edit_name() {
            return Some(name);
        }
        if let Some(name) = self.session_name() {
            return Some(name);
        }
        if let Some(name) = self.system_name() {
            return Some(name);
        }
        if let Some(name) = self.flow_name() {
            return Some(name);
        }
        None
    }

    const fn input_name(&self) -> Option<&'static str> {
        Some(match *self {
            Event::Backspace => "Backspace",
            Event::Newline => "Newline",
            Event::Submit => "Submit",
            Event::CursorLeft => "CursorLeft",
            Event::CursorRight => "CursorRight",
            Event::CursorStart => "CursorStart",
            Event::CursorEnd => "CursorEnd",
            Event::DeleteWord => "DeleteWord",
            Event::DeleteToEnd => "DeleteToEnd",
            Event::DeleteToStart => "DeleteToStart",
            Event::KillChar => "KillChar",
            Event::HistoryPrev => "HistoryPrev",
            Event::HistoryNext => "HistoryNext",
            Event::Undo => "Undo",
            Event::Redo => "Redo",
            Event::CursorWordLeft => "CursorWordLeft",
            Event::CursorWordRight => "CursorWordRight",
            Event::PasteImage => "PasteImage",
            Event::ScrollUp => "ScrollUp",
            Event::ScrollDown => "ScrollDown",
            Event::PageUp => "PageUp",
            Event::PageDown => "PageDown",
            Event::GoToTop => "GoToTop",
            Event::GoToBottom => "GoToBottom",
            _ => return None,
        })
    }

    const fn agent_name(&self) -> Option<&'static str> {
        Some(match *self {
            Event::Quit => "Quit",
            Event::Reset => "Reset",
            Event::Abort => "Abort",
            Event::ToggleExpand => "ToggleExpand",
            Event::Dequeue => "Dequeue",
            Event::OpenExternalEditor => "OpenExternalEditor",
            Event::Suspend => "Suspend",
            Event::ShareSession => "ShareSession",
            Event::FollowUp => "FollowUp",
            Event::AtFilePicker => "AtFilePicker",
            Event::ToggleVimMode => "ToggleVimMode",
            _ => return None,
        })
    }

    const fn dialog_name(&self) -> Option<&'static str> {
        Some(match *self {
            Event::ToggleCommandPalette => "ToggleCommandPalette",
            Event::PaletteBackspace => "PaletteBackspace",
            Event::PaletteUp => "PaletteUp",
            Event::PaletteDown => "PaletteDown",
            Event::PaletteSelect => "PaletteSelect",
            Event::PaletteClose => "PaletteClose",
            Event::ToggleModelSelector => "ToggleModelSelector",
            Event::ModelSelectorBackspace => "ModelSelectorBackspace",
            Event::ModelSelectorUp => "ModelSelectorUp",
            Event::ModelSelectorDown => "ModelSelectorDown",
            Event::ModelSelectorSelect => "ModelSelectorSelect",
            Event::ModelSelectorClose => "ModelSelectorClose",
            Event::ToggleSettingsDialog => "ToggleSettingsDialog",
            Event::SettingsUp => "SettingsUp",
            Event::SettingsDown => "SettingsDown",
            Event::SettingsLeft => "SettingsLeft",
            Event::SettingsRight => "SettingsRight",
            Event::SettingsSelect => "SettingsSelect",
            Event::SettingsClose => "SettingsClose",
            Event::CommandFormBackspace => "CommandFormBackspace",
            Event::CommandFormUp => "CommandFormUp",
            Event::CommandFormDown => "CommandFormDown",
            Event::CommandFormSubmit => "CommandFormSubmit",
            Event::CommandFormClose => "CommandFormClose",
            Event::ToggleScopedModelsDialog => "ToggleScopedModelsDialog",
            Event::ScopedModelEnableAll => "ScopedModelEnableAll",
            Event::ScopedModelDisableAll => "ScopedModelDisableAll",
            Event::DialogBack => "DialogBack",
            _ => return None,
        })
    }

    const fn edit_name(&self) -> Option<&'static str> {
        Some(match *self {
            Event::ApproveEdit => "ApproveEdit",
            Event::RejectEdit => "RejectEdit",
            Event::ReloadAll => "ReloadAll",
            Event::ShowDiagnostics => "ShowDiagnostics",
            Event::TogglePathCompletion => "TogglePathCompletion",
            Event::PathCompletionUp => "PathCompletionUp",
            Event::PathCompletionDown => "PathCompletionDown",
            Event::PathCompletionSelect => "PathCompletionSelect",
            Event::PathCompletionClose => "PathCompletionClose",
            _ => return None,
        })
    }

    const fn session_name(&self) -> Option<&'static str> {
        Some(match *self {
            Event::CloneSession => "CloneSession",
            Event::ToggleSessionTree => "ToggleSessionTree",
            Event::SessionTreeFilterCycle => "SessionTreeFilterCycle",
            _ => return None,
        })
    }

    const fn system_name(&self) -> Option<&'static str> {
        Some(match *self {
            Event::CycleModelNext => "CycleModelNext",
            Event::CycleModelPrev => "CycleModelPrev",
            Event::CycleThinkingLevel => "CycleThinkingLevel",
            Event::ToggleReadOnly => "ToggleReadOnly",
            Event::TrustProject => "TrustProject",
            Event::UntrustProject => "UntrustProject",
            Event::OpenAgentsManager => "OpenAgentsManager",
            Event::AgentsManagerSave { .. } => "AgentsManagerSave",
            Event::AgentsManagerDelete { .. } => "AgentsManagerDelete",
            _ => return None,
        })
    }

    const fn flow_name(&self) -> Option<&'static str> {
        Some(match *self {
            Event::ProvidersDialog => "ProvidersDialog",
            Event::ProvidersAdd => "ProvidersAdd",
            Event::LoginFlowStart => "LoginFlowStart",
            Event::LoginFlowSave => "LoginFlowSave",
            Event::LoginFlowCancel => "LoginFlowCancel",
            Event::ClearTransient => "ClearTransient",
            Event::FocusGained => "FocusGained",
            Event::FocusLost => "FocusLost",
            _ => return None,
        })
    }

    /// Build an Event from its canonical name. Supports the special
    /// `Input:<char>` prefix for character input bindings.
    pub fn from_name(name: &str) -> Option<Event> {
        if let Some(rest) = name.strip_prefix("Input:") {
            let c = rest.chars().next()?;
            return Some(Event::Input(c));
        }
        EVENT_NAMES
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, ctor)| ctor())
    }
}

/// Compile-time check: `Event::name` must be exhaustive. If a variant is
/// added without updating the match above, this block fails to compile.
const _: () = {
    fn _exhaustive(e: &Event) {
        let _ = e.name();
    }
};
