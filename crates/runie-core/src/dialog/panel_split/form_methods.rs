//! Panel form methods.
//!
//! Split from dialog/panel.rs to stay under the 500-line limit.

use super::{Panel, PanelItem};

impl Panel {
    /// Returns true if this panel renders as a form view.
    pub fn is_form(&self) -> bool {
        matches!(self.view, super::PanelView::Form)
    }

    /// Get the index of the currently selected form field.
    pub fn selected_form_field(&self) -> Option<usize> {
        let mut nav = 0;
        for (i, item) in self.items.iter().enumerate() {
            if item.is_navigable() {
                if nav == self.selected && matches!(item, PanelItem::FormField { .. }) {
                    return Some(i);
                }
                nav += 1;
            }
        }
        None
    }

    /// Update the value of a form field by its index.
    pub fn set_form_value(&mut self, field_index: usize, value: String) {
        if let Some(PanelItem::FormField { value: v, key, cursor_pos, .. }) = self.items.get_mut(field_index) {
            *v = value.clone();
            *cursor_pos = v.len();
            self.form_values.insert(key.clone(), value);
        }
    }

    /// Get all form values as a map.
    pub fn get_form_values(&self) -> &std::collections::HashMap<String, String> {
        &self.form_values
    }

    /// Get form field keys in declaration order (matching the `fields` order in
    /// `CommandSpec`). This is used by the command-registry submit path to
    /// serialize form values as positional arguments.
    pub fn get_form_field_keys(&self) -> Vec<&str> {
        self.items
            .iter()
            .filter_map(|item| {
                if let PanelItem::FormField { key, .. } = item {
                    Some(key.as_str())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Find a button whose label contains an accelerator matching `c`.
    pub fn find_button_by_accel(&self, c: char) -> Option<&super::ItemAction> {
        let c = c.to_ascii_lowercase();
        for item in &self.items {
            if let PanelItem::Action { label, action } = item {
                if let Some(accel) = super::super::item::parse_accel(label) {
                    if accel.to_ascii_lowercase() == c {
                        return Some(action);
                    }
                }
            }
        }
        None
    }
}
