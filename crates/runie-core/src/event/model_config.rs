//! Model and config event variants (provider, model, theme, thinking level).

use std::fmt;
use strum::IntoStaticStr;

/// Events that change the active model, provider, theme, or thinking config.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, IntoStaticStr)]
#[strum(serialize_all = "PascalCase")]
pub enum ModelConfigEvent {
    SwitchModel { provider: String, model: String },
    SwitchTheme { name: String },
    CycleModelNext,
    CycleModelPrev,
    ToggleScopedModelsDialog,
    ScopedModelToggle { name: String },
    ScopedModelEnableAll,
    ScopedModelDisableAll,
    ScopedModelToggleProvider { provider: String },
    ToggleSettingsDialog,
    SettingsUp,
    SettingsDown,
    SettingsLeft,
    SettingsRight,
    SettingsSelect,
    SettingsClose,
    SettingsSwitchCategory { category: crate::settings::SettingsCategory },
    CycleThinkingLevel,
    SetThinkingLevel(crate::model::ThinkingLevel),
    ToggleReadOnly,
    TrustProject,
    UntrustProject,
    /// Reload all configuration (provider + theme).
    ReloadAll,
    /// Keybindings section in config.toml was reloaded.
    KeybindingsReloaded,
}

impl ModelConfigEvent {
    /// Canonical name for bindable events. Returns `None` for parameterized variants.
    pub fn variant_name(&self) -> Option<&'static str> {
        match self {
            ModelConfigEvent::SwitchModel { .. } => None,
            ModelConfigEvent::SwitchTheme { .. } => None,
            ModelConfigEvent::CycleModelNext => Some("CycleModelNext"),
            ModelConfigEvent::CycleModelPrev => Some("CycleModelPrev"),
            ModelConfigEvent::ToggleScopedModelsDialog => Some("ToggleScopedModelsDialog"),
            ModelConfigEvent::ScopedModelToggle { .. } => None,
            ModelConfigEvent::ScopedModelEnableAll => Some("ScopedModelEnableAll"),
            ModelConfigEvent::ScopedModelDisableAll => Some("ScopedModelDisableAll"),
            ModelConfigEvent::ScopedModelToggleProvider { .. } => None,
            ModelConfigEvent::ToggleSettingsDialog => Some("ToggleSettingsDialog"),
            ModelConfigEvent::SettingsUp => Some("SettingsUp"),
            ModelConfigEvent::SettingsDown => Some("SettingsDown"),
            ModelConfigEvent::SettingsLeft => Some("SettingsLeft"),
            ModelConfigEvent::SettingsRight => Some("SettingsRight"),
            ModelConfigEvent::SettingsSelect => Some("SettingsSelect"),
            ModelConfigEvent::SettingsClose => Some("SettingsClose"),
            ModelConfigEvent::SettingsSwitchCategory { .. } => None,
            ModelConfigEvent::CycleThinkingLevel => Some("CycleThinkingLevel"),
            ModelConfigEvent::SetThinkingLevel(_) => None,
            ModelConfigEvent::ToggleReadOnly => Some("ToggleReadOnly"),
            ModelConfigEvent::TrustProject => Some("TrustProject"),
            ModelConfigEvent::UntrustProject => Some("UntrustProject"),
            ModelConfigEvent::ReloadAll => Some("ReloadAll"),
            ModelConfigEvent::KeybindingsReloaded => Some("KeybindingsReloaded"),
        }
    }
}

impl fmt::Display for ModelConfigEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModelConfigEvent::SwitchModel { .. } => write!(f, "SwitchModel"),
            ModelConfigEvent::SwitchTheme { .. } => write!(f, "SwitchTheme"),
            ModelConfigEvent::CycleModelNext => write!(f, "CycleModelNext"),
            ModelConfigEvent::CycleModelPrev => write!(f, "CycleModelPrev"),
            ModelConfigEvent::ToggleScopedModelsDialog => write!(f, "ToggleScopedModelsDialog"),
            ModelConfigEvent::ScopedModelToggle { .. } => write!(f, "ScopedModelToggle"),
            ModelConfigEvent::ScopedModelEnableAll => write!(f, "ScopedModelEnableAll"),
            ModelConfigEvent::ScopedModelDisableAll => write!(f, "ScopedModelDisableAll"),
            ModelConfigEvent::ScopedModelToggleProvider { .. } => write!(f, "ScopedModelToggleProvider"),
            ModelConfigEvent::ToggleSettingsDialog => write!(f, "ToggleSettingsDialog"),
            ModelConfigEvent::SettingsUp => write!(f, "SettingsUp"),
            ModelConfigEvent::SettingsDown => write!(f, "SettingsDown"),
            ModelConfigEvent::SettingsLeft => write!(f, "SettingsLeft"),
            ModelConfigEvent::SettingsRight => write!(f, "SettingsRight"),
            ModelConfigEvent::SettingsSelect => write!(f, "SettingsSelect"),
            ModelConfigEvent::SettingsClose => write!(f, "SettingsClose"),
            ModelConfigEvent::SettingsSwitchCategory { .. } => write!(f, "SettingsSwitchCategory"),
            ModelConfigEvent::CycleThinkingLevel => write!(f, "CycleThinkingLevel"),
            ModelConfigEvent::SetThinkingLevel(_) => write!(f, "SetThinkingLevel"),
            ModelConfigEvent::ToggleReadOnly => write!(f, "ToggleReadOnly"),
            ModelConfigEvent::TrustProject => write!(f, "TrustProject"),
            ModelConfigEvent::UntrustProject => write!(f, "UntrustProject"),
            ModelConfigEvent::ReloadAll => write!(f, "ReloadAll"),
            ModelConfigEvent::KeybindingsReloaded => write!(f, "KeybindingsReloaded"),
        }
    }
}
