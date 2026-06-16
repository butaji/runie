//! Event name mapping — generated from sub-enum `IntoStaticStr` derives.
//!
//! `EVENT_NAMES` maps each bindable event name to its default constructor.
//! Names are derived from the sub-enum variant names (via `strum::IntoStaticStr`).
//! Parameterized variants (those with data) are handled separately in `Event::from_name`.
//!
//! To add a new bindable event:
//! 1. Ensure the sub-enum variant has no data (or is explicitly handled in `from_name`)
//! 2. The name is automatically derived from the variant name via `#[strum(serialize_all = "PascalCase")]`
//! 3. Add the constructor to the appropriate list below

use super::variants::Event;
use super::{
    ControlEvent, DialogEvent, InputEvent, ModelConfigEvent, SystemEvent,
};

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Helper to build a zero-arg constructor for a sub-enum variant.
macro_rules! ctor {
    (Input::$v:ident) => {|| Event::Input(InputEvent::$v)};
    (Agent::$v:ident) => {|| Event::Agent(AgentEvent::$v)};
    (Control::$v:ident) => {|| Event::Control(ControlEvent::$v)};
    (Dialog::$v:ident) => {|| Event::Dialog(DialogEvent::$v)};
    (Edit::$v:ident) => {|| Event::Edit(EditEvent::$v)};
    (System::$v:ident) => {|| Event::System(SystemEvent::$v)};
    (Scroll::$v:ident) => {|| Event::Scroll(ScrollEvent::$v)};
    (Session::$v:ident) => {|| Event::Session(SessionEvent::$v)};
    (LoginFlow::$v:ident) => {|| Event::LoginFlow(LoginFlowEvent::$v)};
    (Command::$v:ident) => {|| Event::Command(CommandEvent::$v)};
    (ModelConfig::$v:ident) => {|| Event::ModelConfig(ModelConfigEvent::$v)};
}

// ── Bindable event table ───────────────────────────────────────────────────────

/// Bindable event names paired with their default zero-arg constructors.
/// This is generated from the sub-enum variants; see module docs for how to extend.
#[allow(clippy::type_complexity)]
pub const EVENT_NAMES: &[(&str, fn() -> Event)] = &[
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
    ("ModelSelectorBackspace", ctor!(Dialog::ModelSelectorBackspace)),
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
    ("ToggleScopedModelsDialog", ctor!(Dialog::ToggleScopedModelsDialog)),
    ("ScopedModelEnableAll", ctor!(Dialog::ScopedModelEnableAll)),
    ("ScopedModelDisableAll", ctor!(Dialog::ScopedModelDisableAll)),
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
