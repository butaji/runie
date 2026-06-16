//! Settings dialog state and items.

/// Category for grouping settings in the dialog.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SettingsCategory {
    Models,
    Appearance,
    Behavior,
    Safety,
}

impl SettingsCategory {
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SettingValue {
    Bool(bool),
    Enum {
        current: String,
        options: Vec<String>,
    },
}

/// A single setting item displayed in the dialog.
#[derive(Clone, Debug, PartialEq, Eq)]
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
