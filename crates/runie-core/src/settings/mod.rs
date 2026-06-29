//! Settings dialog state and items.

mod dialog;

pub use dialog::{build_setting_categories, build_setting_items, handle_settings_category, provider_model_lists};

use crate::Event;

/// Category for grouping settings in the dialog.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[derive(strum::Display)]
pub enum SettingsCategory {
    Models,
    Appearance,
    Behavior,
    Safety,
}

impl SettingsCategory {
    /// String representation (PascalCase).
    pub fn as_str(&self) -> &'static str {
        match self {
            SettingsCategory::Models => "Models",
            SettingsCategory::Appearance => "Appearance",
            SettingsCategory::Behavior => "Behavior",
            SettingsCategory::Safety => "Safety",
        }
    }

    pub fn all() -> &'static [SettingsCategory] {
        &[
            SettingsCategory::Models,
            SettingsCategory::Appearance,
            SettingsCategory::Behavior,
            SettingsCategory::Safety,
        ]
    }
}

/// Value type for a setting.
#[derive(Clone, Debug, PartialEq)]
pub enum SettingValue {
    Bool(bool),
    Cycle {
        current: String,
        options: Vec<String>,
    },
    Action(Event),
    /// Multi-select checkbox list. `current` is the set of selected options;
    /// `options` is the full set of available options.
    MultiSelect {
        current: Vec<String>,
        options: Vec<String>,
    },
}

/// A single setting item displayed in the dialog.
#[derive(Clone, Debug, PartialEq)]
pub struct SettingItem {
    pub key: String,
    pub label: String,
    pub value: SettingValue,
    pub description: String,
    pub category: SettingsCategory,
}

impl SettingItem {
    pub fn new(
        key: &str,
        label: &str,
        value: SettingValue,
        description: &str,
        category: SettingsCategory,
    ) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            value,
            description: description.into(),
            category,
        }
    }
}
