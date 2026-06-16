//! `Display` for DialogEvent — split from dialog.rs to keep each file under 50 lines.

use std::fmt;
use strum::IntoStaticStr;

/// Events that open, navigate, or close dialogs and palettes.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, IntoStaticStr)]
#[strum(serialize_all = "PascalCase")]
pub enum DialogEvent {
    // Welcome / launcher
    ToggleWelcome,
    // Command palette
    ToggleCommandPalette,
    PaletteFilter(char),
    PaletteBackspace,
    PaletteUp,
    PaletteDown,
    PaletteSelect,
    PaletteClose,
    // Model selector
    ToggleModelSelector,
    ModelSelectorFilter(char),
    ModelSelectorBackspace,
    ModelSelectorUp,
    ModelSelectorDown,
    ModelSelectorSelect,
    ModelSelectorClose,
    // Settings dialog
    ToggleSettingsDialog,
    SettingsUp,
    SettingsDown,
    SettingsLeft,
    SettingsRight,
    SettingsSelect,
    SettingsClose,
    // Path completion
    TogglePathCompletion,
    PathCompletionUp,
    PathCompletionDown,
    PathCompletionSelect,
    PathCompletionClose,
    // Form dialogs
    CommandFormInput(char),
    CommandFormBackspace,
    CommandFormUp,
    CommandFormDown,
    CommandFormSubmit,
    CommandFormClose,
    RunSaveCommand { name: String },
    DialogBack,
    // Scoped models
    ToggleScopedModelsDialog,
    ScopedModelEnableAll,
    ScopedModelDisableAll,
    // Providers dialog
    ProvidersDialog,
    ProvidersSelectModel { provider: String, model: String },
    ProvidersDisconnect { provider: String },
    ProvidersAdd,
    // Agent manager
    OpenAgentsManager,
    AgentsManagerSetField { name: String, field: String, value: String },
    AgentsManagerSave { name: String },
    AgentsManagerDelete { name: String },
    // Clipboard & reference insertion
    CopyToClipboard(String),
    CopyLastResponse,
    CopySelectedBlock,
    CopyBlockMetadata,
    AtFilePicker,
    InsertAtRef(String),
    // Mode toggles
    ToggleVimMode,
}

impl DialogEvent {
    /// Canonical name for bindable events. Returns `None` for parameterized variants.
    pub fn variant_name(&self) -> Option<&'static str> {
        match self {
            DialogEvent::ToggleWelcome => Some("ToggleWelcome"),
            DialogEvent::ToggleCommandPalette => Some("ToggleCommandPalette"),
            DialogEvent::PaletteFilter(_) => None,
            DialogEvent::PaletteBackspace => Some("PaletteBackspace"),
            DialogEvent::PaletteUp | DialogEvent::PaletteDown => None,
            DialogEvent::PaletteSelect | DialogEvent::PaletteClose => None,
            DialogEvent::ToggleModelSelector => Some("ToggleModelSelector"),
            DialogEvent::ModelSelectorFilter(_) | DialogEvent::ModelSelectorBackspace => None,
            DialogEvent::ModelSelectorUp | DialogEvent::ModelSelectorDown => None,
            DialogEvent::ModelSelectorSelect | DialogEvent::ModelSelectorClose => None,
            DialogEvent::ToggleSettingsDialog => Some("ToggleSettingsDialog"),
            DialogEvent::SettingsUp | DialogEvent::SettingsDown | DialogEvent::SettingsLeft | DialogEvent::SettingsRight => None,
            DialogEvent::SettingsSelect | DialogEvent::SettingsClose => None,
            DialogEvent::TogglePathCompletion => Some("TogglePathCompletion"),
            DialogEvent::PathCompletionUp | DialogEvent::PathCompletionDown => None,
            DialogEvent::PathCompletionSelect | DialogEvent::PathCompletionClose => None,
            DialogEvent::CommandFormInput(_) | DialogEvent::CommandFormBackspace => None,
            DialogEvent::CommandFormUp | DialogEvent::CommandFormDown => None,
            DialogEvent::CommandFormSubmit | DialogEvent::CommandFormClose => None,
            DialogEvent::RunSaveCommand { .. } => None,
            DialogEvent::DialogBack => Some("DialogBack"),
            DialogEvent::ToggleScopedModelsDialog => Some("ToggleScopedModelsDialog"),
            DialogEvent::ScopedModelEnableAll => Some("ScopedModelEnableAll"),
            DialogEvent::ScopedModelDisableAll => Some("ScopedModelDisableAll"),
            DialogEvent::ProvidersDialog => Some("ProvidersDialog"),
            DialogEvent::ProvidersSelectModel { .. } => None,
            DialogEvent::ProvidersDisconnect { .. } => None,
            DialogEvent::ProvidersAdd => Some("ProvidersAdd"),
            DialogEvent::OpenAgentsManager => Some("OpenAgentsManager"),
            DialogEvent::AgentsManagerSetField { .. } => None,
            DialogEvent::AgentsManagerSave { .. } => None,
            DialogEvent::AgentsManagerDelete { .. } => None,
            DialogEvent::CopyToClipboard(_) => None,
            DialogEvent::CopyLastResponse => Some("CopyLastResponse"),
            DialogEvent::CopySelectedBlock => Some("CopySelectedBlock"),
            DialogEvent::CopyBlockMetadata => Some("CopyBlockMetadata"),
            DialogEvent::AtFilePicker => Some("AtFilePicker"),
            DialogEvent::InsertAtRef(_) => None,
            DialogEvent::ToggleVimMode => Some("ToggleVimMode"),
        }
    }
}

impl fmt::Display for DialogEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // Single-word toggles
            DialogEvent::ToggleWelcome => write!(f, "ToggleWelcome"),
            DialogEvent::ToggleCommandPalette => write!(f, "ToggleCommandPalette"),
            DialogEvent::ToggleModelSelector => write!(f, "ToggleModelSelector"),
            DialogEvent::ToggleSettingsDialog => write!(f, "ToggleSettingsDialog"),
            DialogEvent::TogglePathCompletion => write!(f, "TogglePathCompletion"),
            DialogEvent::ToggleScopedModelsDialog => write!(f, "ToggleScopedModelsDialog"),
            DialogEvent::DialogBack => write!(f, "DialogBack"),
            DialogEvent::ProvidersDialog => write!(f, "ProvidersDialog"),
            DialogEvent::OpenAgentsManager => write!(f, "OpenAgentsManager"),
            DialogEvent::CopyLastResponse => write!(f, "CopyLastResponse"),
            DialogEvent::CopySelectedBlock => write!(f, "CopySelectedBlock"),
            DialogEvent::CopyBlockMetadata => write!(f, "CopyBlockMetadata"),
            DialogEvent::AtFilePicker => write!(f, "AtFilePicker"),
            DialogEvent::ToggleVimMode => write!(f, "ToggleVimMode"),
            DialogEvent::ProvidersAdd => write!(f, "ProvidersAdd"),
            // All nav variants (cursor/signal input)
            DialogEvent::PaletteFilter(_) | DialogEvent::PaletteBackspace => write!(f, "PaletteFilter"),
            DialogEvent::PaletteUp | DialogEvent::PaletteDown => write!(f, "PaletteNav"),
            DialogEvent::PaletteSelect | DialogEvent::PaletteClose => write!(f, "PaletteAction"),
            DialogEvent::ModelSelectorFilter(_) | DialogEvent::ModelSelectorBackspace => write!(f, "ModelSelectorFilter"),
            DialogEvent::ModelSelectorUp | DialogEvent::ModelSelectorDown => write!(f, "ModelSelectorNav"),
            DialogEvent::ModelSelectorSelect | DialogEvent::ModelSelectorClose => write!(f, "ModelSelectorAction"),
            DialogEvent::SettingsUp | DialogEvent::SettingsDown | DialogEvent::SettingsLeft | DialogEvent::SettingsRight => write!(f, "SettingsNav"),
            DialogEvent::SettingsSelect | DialogEvent::SettingsClose => write!(f, "SettingsAction"),
            DialogEvent::PathCompletionUp | DialogEvent::PathCompletionDown => write!(f, "PathCompletionNav"),
            DialogEvent::PathCompletionSelect | DialogEvent::PathCompletionClose => write!(f, "PathCompletionAction"),
            DialogEvent::CommandFormInput(_) | DialogEvent::CommandFormBackspace => write!(f, "CommandFormInput"),
            DialogEvent::CommandFormUp | DialogEvent::CommandFormDown => write!(f, "CommandFormNav"),
            DialogEvent::CommandFormSubmit | DialogEvent::CommandFormClose => write!(f, "CommandFormAction"),
            DialogEvent::ScopedModelEnableAll | DialogEvent::ScopedModelDisableAll => write!(f, "ScopedModelToggleAll"),
            // Struct variants
            DialogEvent::RunSaveCommand { .. } => write!(f, "RunSaveCommand"),
            DialogEvent::ProvidersSelectModel { .. } => write!(f, "ProvidersSelectModel"),
            DialogEvent::ProvidersDisconnect { .. } => write!(f, "ProvidersDisconnect"),
            DialogEvent::AgentsManagerSetField { .. } => write!(f, "AgentsManagerSetField"),
            DialogEvent::AgentsManagerSave { .. } => write!(f, "AgentsManagerSave"),
            DialogEvent::AgentsManagerDelete { .. } => write!(f, "AgentsManagerDelete"),
            DialogEvent::CopyToClipboard(_) => write!(f, "CopyToClipboard"),
            DialogEvent::InsertAtRef(_) => write!(f, "InsertAtRef"),
        }
    }
}
