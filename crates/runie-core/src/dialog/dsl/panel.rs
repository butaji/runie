//! Panel Builder - Fluent API for creating panels

use super::{ItemAction, PanelItem};
use crate::Event;
use std::collections::HashMap;

/// A panel builder with fluent API
#[derive(Debug, Clone)]
pub struct Panel {
    pub id: String,
    pub title: String,
    pub items: Vec<PanelItem>,
    pub selected: usize,
    pub filter: String,
    pub filterable: bool,
    pub keep_open_on_activate: bool,
    pub form_values: HashMap<String, String>,
}

impl Panel {
    /// Create a new panel builder
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            items: Vec::new(),
            selected: 0,
            filter: String::new(),
            // List-style panels are searchable by default. Forms opt-out via the
            // form builder, which disables filtering before adding fields.
            filterable: true,
            keep_open_on_activate: false,
            form_values: HashMap::new(),
        }
    }

    /// Add an action item
    pub fn action(mut self, label: impl Into<String>, action: impl Into<ItemAction>) -> Self {
        let label = label.into();
        let action = action.into();
        self.items.push(PanelItem::Action { label, action });
        self
    }

    /// Add an item with auto-generated label from action
    pub fn item(mut self, action: impl Into<ItemAction>) -> Self {
        let action = action.into();
        let label = action.default_label();
        self.items.push(PanelItem::Action { label, action });
        self
    }

    /// Add a toggle item
    pub fn toggle(mut self, label: impl Into<String>, value: bool, key: impl Into<String>) -> Self {
        self.items.push(PanelItem::Toggle {
            label: label.into(),
            value,
            key: key.into(),
        });
        self
    }

    /// Add a select item
    pub fn select(
        mut self,
        label: impl Into<String>,
        current: impl Into<String>,
        options: Vec<String>,
        key: impl Into<String>,
    ) -> Self {
        self.items.push(PanelItem::Select {
            label: label.into(),
            current: current.into(),
            options,
            key: key.into(),
        });
        self
    }

    /// Add a header (non-navigable)
    pub fn header(mut self, text: impl Into<String>) -> Self {
        self.items.push(PanelItem::Header(text.into()));
        self
    }

    /// Add a separator (non-navigable)
    pub fn sep(mut self) -> Self {
        self.items.push(PanelItem::Separator);
        self
    }

    /// Add a form field
    pub fn field(
        mut self,
        label: impl Into<String>,
        placeholder: impl Into<String>,
        key: impl Into<String>,
    ) -> Self {
        self.items.push(PanelItem::FormField {
            label: label.into(),
            value: String::new(),
            placeholder: placeholder.into(),
            key: key.into(),
        });
        self
    }

    /// Add a form field with pre-filled value
    pub fn field_value(
        mut self,
        label: impl Into<String>,
        placeholder: impl Into<String>,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        let value = value.into();
        let key_str = key.into();
        self.items.push(PanelItem::FormField {
            label: label.into(),
            value: value.clone(),
            placeholder: placeholder.into(),
            key: key_str.clone(),
        });
        if !value.is_empty() {
            self.form_values.insert(key_str, value);
        }
        self
    }

    /// Mark as filterable (typing filters items). No-op for list panels
    /// because they are already searchable by default.
    pub fn searchable(mut self) -> Self {
        self.filterable = true;
        self
    }

    /// Explicitly enable or disable fuzzy filtering for this panel.
    pub fn filterable(mut self, enabled: bool) -> Self {
        self.filterable = enabled;
        self
    }

    /// When true, the panel stays open after activating an item (Enter).
    /// Use for previews like theme picker or live toggles.
    pub fn keep_open(mut self) -> Self {
        self.keep_open_on_activate = true;
        self
    }

    /// Add items from a closure (for sections/groups)
    pub fn section<F>(mut self, header: impl Into<String>, f: F) -> Self
    where
        F: FnOnce(Panel) -> Panel,
    {
        self.items.push(PanelItem::Header(header.into()));
        self = f(self);
        self
    }

    /// Shorthand for grouping items
    pub fn group<F>(mut self, f: F) -> Self
    where
        F: FnOnce(Panel) -> Panel,
    {
        f(self)
    }

    /// Navigate selection up
    pub fn select_up(&mut self) {
        let count = self.navigable_count();
        if count == 0 {
            return;
        }
        self.selected = if self.selected == 0 {
            count - 1
        } else {
            self.selected - 1
        };
    }

    /// Navigate selection down
    pub fn select_down(&mut self) {
        let count = self.navigable_count();
        if count == 0 {
            return;
        }
        self.selected = (self.selected + 1) % count;
    }

    /// Count of navigable items
    pub fn navigable_count(&self) -> usize {
        self.filtered_items()
            .iter()
            .filter(|i| i.is_navigable())
            .count()
    }

    /// Get filtered items
    pub fn filtered_items(&self) -> Vec<&PanelItem> {
        if !self.filterable || self.filter.is_empty() {
            return self.items.iter().collect();
        }
        let q = self.filter.to_lowercase();
        let mut result = Vec::new();
        let mut pending_headers: Vec<&PanelItem> = Vec::new();
        for item in &self.items {
            match item {
                PanelItem::Header(_) | PanelItem::Separator => {
                    pending_headers.push(item);
                }
                _ => {
                    if item
                        .label()
                        .map_or(false, |l| l.to_lowercase().contains(&q))
                    {
                        result.extend(pending_headers.drain(..));
                        result.push(item);
                    }
                }
            }
        }
        result
    }

    /// Map nav index to raw index
    pub fn raw_index(&self, nav_index: usize) -> Option<usize> {
        let mut nav = 0;
        for (i, item) in self.items.iter().enumerate() {
            if item.is_navigable() {
                if nav == nav_index {
                    return Some(i);
                }
                nav += 1;
            }
        }
        None
    }

    /// Get selected item
    pub fn selected_item(&self) -> Option<&PanelItem> {
        let filtered = self.filtered_items();
        let mut nav = 0;
        for item in filtered {
            if item.is_navigable() {
                if nav == self.selected {
                    return Some(item);
                }
                nav += 1;
            }
        }
        None
    }

    /// Mutable access to selected item
    pub fn selected_item_mut(&mut self) -> Option<&mut PanelItem> {
        self.raw_index(self.selected)
            .and_then(|i| self.items.get_mut(i))
    }

    /// Push filter character
    pub fn push_filter(&mut self, c: char) {
        self.filter.push(c);
        self.selected = 0;
    }

    /// Pop filter character
    pub fn pop_filter(&mut self) {
        self.filter.pop();
        self.selected = 0;
    }

    /// Check if panel has form fields
    pub fn is_form(&self) -> bool {
        self.items
            .iter()
            .any(|i| matches!(i, PanelItem::FormField { .. }))
    }

    /// Get selected form field index
    pub fn selected_form_field(&self) -> Option<usize> {
        let filtered = self.filtered_items();
        let mut nav = 0;
        for (i, item) in self.items.iter().enumerate() {
            if matches!(item, PanelItem::FormField { .. }) {
                if nav == self.selected {
                    return Some(i);
                }
                nav += 1;
            }
        }
        None
    }

    /// Set form field value
    pub fn set_form_value(&mut self, field_index: usize, value: String) {
        if let Some(PanelItem::FormField { value: v, key, .. }) = self.items.get_mut(field_index) {
            *v = value.clone();
            self.form_values.insert(key.clone(), value);
        }
    }

    /// Get all form values
    pub fn get_form_values(&self) -> &HashMap<String, String> {
        &self.form_values
    }

    /// Convert to core Panel
    pub fn into_core(self) -> super::super::Panel {
        super::super::Panel {
            id: self.id,
            title: self.title,
            items: self.items,
            selected: self.selected,
            filter: self.filter,
            filterable: self.filterable,
            keep_open_on_activate: self.keep_open_on_activate,
            form_values: self.form_values,
        }
    }
}

/// Create a new list-view panel builder. Alias for [`panel`].
pub fn list(id: impl Into<String>, title: impl Into<String>) -> Panel {
    Panel::new(id, title)
}

/// Create a new list-view panel builder.
pub fn panel(id: impl Into<String>, title: impl Into<String>) -> Panel {
    Panel::new(id, title)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialog::ItemAction;

    #[test]
    fn test_panel_builder_chain() {
        let p = panel("test", "Test")
            .header("Section 1")
            .action("Option A", ItemAction::Close)
            .toggle("Enable", false, "enabled")
            .sep()
            .select("Choice", "a", vec!["a".into(), "b".into()], "choice");

        assert_eq!(p.id, "test");
        assert_eq!(p.items.len(), 5);
    }

    #[test]
    fn test_panel_navigation() {
        let mut p = panel("test", "Test")
            .header("Group")
            .action("A", ItemAction::Close)
            .action("B", ItemAction::Close);

        assert_eq!(p.navigable_count(), 2);
        p.select_down();
        assert_eq!(p.selected, 1);
        p.select_down();
        assert_eq!(p.selected, 0);
    }

    #[test]
    fn test_panel_filter() {
        let mut p = panel("test", "Test")
            .searchable()
            .action("alpha", ItemAction::Close)
            .action("beta", ItemAction::Close)
            .action("gamma", ItemAction::Close);

        p.push_filter('g');
        let filtered = p.filtered_items();
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn test_panel_section() {
        let p = panel("test", "Test").section("Settings", |p| {
            p.toggle("Option 1", false, "opt1")
                .toggle("Option 2", true, "opt2")
        });
        assert_eq!(p.items.len(), 3);
    }

    #[test]
    fn test_panel_into_core() {
        let dsl_panel = panel("test", "Test").action("Click me", ItemAction::Close);

        let core = dsl_panel.into_core();
        assert_eq!(core.id, "test");
        assert_eq!(core.title, "Test");
    }
}
