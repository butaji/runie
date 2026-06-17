//! Event name mapping — flat `Event` constructors for bindable names.
//!
//! `EVENT_NAMES` maps each bindable event name to its default constructor.
//! Names match the flat `Event` variant names directly.
//! Parameterized variants (those with data) are handled separately in `Event::from_name`.

use super::variants::Event;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Helper to build a zero-arg constructor for a flat event variant.
macro_rules! ctor {
    (Input::$v:ident) => {
        || Event::$v
    };
    (Agent::$v:ident) => {
        || Event::$v
    };
    (Scroll::$v:ident) => {
        || Event::$v
    };
    (Control::$v:ident) => {
        || Event::$v
    };
    (Dialog::$v:ident) => {
        || Event::$v
    };
    (Edit::$v:ident) => {
        || Event::$v
    };
    (System::$v:ident) => {
        || Event::$v
    };
    (Session::$v:ident) => {
        || Event::$v
    };
    (LoginFlow::$v:ident) => {
        || Event::$v
    };
    (Command::$v:ident) => {
        || Event::$v
    };
    (Sidebar::$v:ident) => {
        || Event::$v
    };
    (Orchestrator::$v:ident) => {
        || Event::$v
    };
    (ModelConfig::$v:ident) => {
        || Event::$v
    };
}

// ── Bindable event table ───────────────────────────────────────────────────────

/// Zero-argument event constructor signature.
pub type EventCtor = fn() -> Event;

/// Bindable event names paired with their default zero-arg constructors.
pub const EVENT_NAMES: &[(&str, EventCtor)] = &[
    // ── Input events (unit variants only) ──────────────────────────────────
    ("Backspace", ctor!(Input::Backspace)),
    ("Newline", ctor!(Input::Newline)),
    ("Submit", ctor!(Input::Submit)),
    ("Escape", ctor!(Input::Escape)),
    ("CursorLeft", ctor!(Input::CursorLeft)),
    ("CursorRight", ctor!(Input::CursorRight)),
    ("CursorStart", ctor!(Input::CursorStart)),
    ("CursorEnd", ctor!(Input::CursorEnd)),
    ("DeleteWord", ctor!(Input::DeleteWord)),
    ("DeleteToEnd", ctor!(Input::DeleteToEnd)),
    ("DeleteToStart", ctor!(Input::DeleteToStart)),
    ("KillChar", ctor!(Input::KillChar)),
    ("HistoryPrev", ctor!(Input::HistoryPrev)),
    ("HistoryNext", ctor!(Input::HistoryNext)),
    ("Undo", ctor!(Input::Undo)),
    ("Redo", ctor!(Input::Redo)),
    ("CursorWordLeft", ctor!(Input::CursorWordLeft)),
    ("CursorWordRight", ctor!(Input::CursorWordRight)),
    ("PageUp", ctor!(Input::PageUp)),
    ("PageDown", ctor!(Input::PageDown)),
    ("GoToTop", ctor!(Input::GoToTop)),
    ("GoToBottom", ctor!(Input::GoToBottom)),
    ("PasteImage", ctor!(Input::PasteImage)),
    ("FocusGained", ctor!(Input::FocusGained)),
    ("FocusLost", ctor!(Input::FocusLost)),
    // ── Control events ──────────────────────────────────────────────────────
    ("Quit", ctor!(Control::Quit)),
    ("ForceQuit", ctor!(Control::ForceQuit)),
    ("Reset", ctor!(Control::Reset)),
    ("Abort", ctor!(Control::Abort)),
    ("FollowUp", ctor!(Control::FollowUp)),
    ("ToggleExpand", ctor!(Control::ToggleExpand)),
    ("Dequeue", ctor!(Control::Dequeue)),
    ("OpenExternalEditor", ctor!(Control::OpenExternalEditor)),
    ("Suspend", ctor!(Control::Suspend)),
    ("ShareSession", ctor!(Control::ShareSession)),
    ("ToggleVimMode", ctor!(Control::ToggleVimMode)),
    ("CopyLastResponse", ctor!(Control::CopyLastResponse)),
    ("OpenSessionList", ctor!(Control::OpenSessionList)),
    ("NewSession", ctor!(Control::NewSession)),
    ("ResumeSession", ctor!(Control::ResumeSession)),
    ("CopySelectedBlock", ctor!(Dialog::CopySelectedBlock)),
    ("CopyBlockMetadata", ctor!(Dialog::CopyBlockMetadata)),
    // ── Dialog events ───────────────────────────────────────────────────────
    ("ToggleCommandPalette", ctor!(Dialog::ToggleCommandPalette)),
    ("PaletteBackspace", ctor!(Dialog::PaletteBackspace)),
    ("PaletteUp", ctor!(Dialog::PaletteUp)),
    ("PaletteDown", ctor!(Dialog::PaletteDown)),
    ("PaletteSelect", ctor!(Dialog::PaletteSelect)),
    ("PaletteClose", ctor!(Dialog::PaletteClose)),
    ("ToggleModelSelector", ctor!(Dialog::ToggleModelSelector)),
    (
        "ModelSelectorBackspace",
        ctor!(Dialog::ModelSelectorBackspace),
    ),
    ("ModelSelectorUp", ctor!(Dialog::ModelSelectorUp)),
    ("ModelSelectorDown", ctor!(Dialog::ModelSelectorDown)),
    ("ModelSelectorSelect", ctor!(Dialog::ModelSelectorSelect)),
    ("ModelSelectorClose", ctor!(Dialog::ModelSelectorClose)),
    ("ToggleSettingsDialog", ctor!(Dialog::ToggleSettingsDialog)),
    ("SettingsUp", ctor!(Dialog::SettingsUp)),
    ("SettingsDown", ctor!(Dialog::SettingsDown)),
    ("SettingsLeft", ctor!(Dialog::SettingsLeft)),
    ("SettingsRight", ctor!(Dialog::SettingsRight)),
    ("SettingsSelect", ctor!(Dialog::SettingsSelect)),
    ("SettingsClose", ctor!(Dialog::SettingsClose)),
    ("CommandFormBackspace", ctor!(Dialog::CommandFormBackspace)),
    ("CommandFormUp", ctor!(Dialog::CommandFormUp)),
    ("CommandFormDown", ctor!(Dialog::CommandFormDown)),
    ("CommandFormSubmit", ctor!(Dialog::CommandFormSubmit)),
    ("CommandFormClose", ctor!(Dialog::CommandFormClose)),
    (
        "ToggleScopedModelsDialog",
        ctor!(Dialog::ToggleScopedModelsDialog),
    ),
    ("ScopedModelEnableAll", ctor!(Dialog::ScopedModelEnableAll)),
    (
        "ScopedModelDisableAll",
        ctor!(Dialog::ScopedModelDisableAll),
    ),
    ("DialogBack", ctor!(Dialog::DialogBack)),
    ("TogglePathCompletion", ctor!(Dialog::TogglePathCompletion)),
    ("PathCompletionUp", ctor!(Dialog::PathCompletionUp)),
    ("PathCompletionDown", ctor!(Dialog::PathCompletionDown)),
    ("PathCompletionSelect", ctor!(Dialog::PathCompletionSelect)),
    ("PathCompletionClose", ctor!(Dialog::PathCompletionClose)),
    ("ProvidersDialog", ctor!(Dialog::ProvidersDialog)),
    ("ProvidersAdd", ctor!(Dialog::ProvidersAdd)),
    ("AtFilePicker", ctor!(Dialog::AtFilePicker)),
    // ── Model config events ─────────────────────────────────────────────────
    ("CycleModelNext", ctor!(ModelConfig::CycleModelNext)),
    ("CycleModelPrev", ctor!(ModelConfig::CycleModelPrev)),
    ("CycleThinkingLevel", ctor!(ModelConfig::CycleThinkingLevel)),
    // ── System events ───────────────────────────────────────────────────────
    ("ToggleReadOnly", ctor!(System::ToggleReadOnly)),
    ("TrustProject", ctor!(System::TrustProject)),
    ("UntrustProject", ctor!(System::UntrustProject)),
    ("OpenAgentsManager", ctor!(System::OpenAgentsManager)),
    ("ClearTransient", ctor!(System::ClearTransient)),
];
