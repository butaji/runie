//! Settings dialog builder.

use super::{ItemAction, Panel, PanelItem, PanelStack};
use crate::settings::SettingValue;

/// One row in the settings dialog.
#[derive(Debug, Clone)]
pub struct SettingsRow {
    pub label: String,
    pub key: String,
    pub kind: SettingValue,
}

/// Build a settings dialog panel with category headers.
pub fn settings(categories: Vec<(String, Vec<SettingsRow>)>) -> PanelStack {
    let mut panel = Panel::new("settings", " Settings ");
    let _cat_count = categories.len();
    for (i, (cat_name, rows)) in categories.into_iter().enumerate() {
        if i > 0 {
            panel = panel.separator();
        }
        panel = panel.header(cat_name);
        for row in rows {
            let item = match row.kind {
                SettingValue::Bool(value) => PanelItem::Toggle {
                    label: row.label,
                    value,
                    action: ItemAction::Toggle(row.key.clone()),
                },
                SettingValue::Cycle { current, options } => PanelItem::Select {
                    label: row.label,
                    current,
                    options,
                    key: row.key,
                },
                SettingValue::Action(evt) => PanelItem::Action {
                    label: row.label,
                    action: ItemAction::Emit(evt),
                },
            };
            panel.items.push(item);
        }
    }
    PanelStack::new(panel)
}
