//! Panel builder methods — fluent API.
//!
//! Split from dialog/panel.rs to stay under the 500-line limit.

use super::{Panel, PanelItem, PanelView};

impl Panel {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: crate::dialog::panel_split::helpers::normalize_title(title),
            items: Vec::new(),
            selected: 0,
            filter: String::new(),
            // List-style panels are searchable by default. Forms should opt-out
            // explicitly since they use keyboard input for field editing.
            filterable: true,
            keep_open_on_activate: false,
            closable: true,
            form_values: std::collections::HashMap::new(),
            cmd_name: None,
            field_keys: Vec::new(),
            view: PanelView::List,
        }
    }

    /// Set the visual layout to a list view.
    pub fn list(mut self) -> Self {
        self.view = PanelView::List;
        self
    }

    /// Set the visual layout to a form view and disable fuzzy filtering.
    pub fn form(mut self) -> Self {
        self.view = PanelView::Form;
        self.filterable = false;
        self
    }

    /// Set the panel title, normalizing it to exactly one leading and trailing space.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = crate::dialog::panel_split::helpers::normalize_title(title);
        self
    }

    /// Alias for `with_title()`.
    pub fn title(self, title: impl Into<String>) -> Self {
        self.with_title(title)
    }

    pub fn item(mut self, label: impl Into<String>, action: super::ItemAction) -> Self {
        self.items.push(PanelItem::Action {
            label: label.into(),
            action,
        });
        self
    }

    /// Add an action item. Alias for `item()`.
    pub fn action(mut self, label: impl Into<String>, action: super::ItemAction) -> Self {
        self.items.push(PanelItem::Action {
            label: label.into(),
            action,
        });
        self
    }

    /// Add an item with auto-generated label from action's default label.
    pub fn item_action(mut self, action: super::ItemAction) -> Self {
        let label = action.default_label();
        self.items.push(PanelItem::Action { label, action });
        self
    }

    /// Add a command-palette entry with separate name and description.
    pub fn command(
        mut self,
        name: impl Into<String>,
        desc: impl Into<String>,
        action: super::ItemAction,
    ) -> Self {
        let name = name.into();
        let desc = desc.into();
        let label = format!("{} {}", name, desc);
        self.items.push(PanelItem::Command {
            name,
            desc,
            label,
            action,
        });
        self
    }

    /// Add a toggle (checkbox) item.
    pub fn toggle(
        mut self,
        label: impl Into<String>,
        value: bool,
        action: super::ItemAction,
    ) -> Self {
        self.items.push(PanelItem::Toggle {
            label: label.into(),
            value,
            action,
        });
        self
    }

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

    pub fn header(mut self, text: impl Into<String>) -> Self {
        self.items.push(PanelItem::Header(text.into()));
        self
    }

    pub fn separator(mut self) -> Self {
        self.items.push(PanelItem::Separator);
        self
    }

    /// Add a separator. Alias for `separator()`.
    pub fn sep(self) -> Self {
        self.separator()
    }

    /// Add a form field. Alias for `form_field()`.
    pub fn field(
        self,
        label: impl Into<String>,
        placeholder: impl Into<String>,
        key: impl Into<String>,
    ) -> Self {
        self.form_field(label, placeholder, key)
    }

    /// Add a form field with a pre-filled value. Alias for `form_field_value()`.
    pub fn field_value(
        self,
        label: impl Into<String>,
        placeholder: impl Into<String>,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        self.form_field_value(label, placeholder, key, value)
    }

    pub fn form_field(
        mut self,
        label: impl Into<String>,
        placeholder: impl Into<String>,
        key: impl Into<String>,
    ) -> Self {
        self.view = PanelView::Form;
        self.filterable = false;
        self.items.push(PanelItem::FormField {
            label: label.into(),
            value: String::new(),
            placeholder: placeholder.into(),
            key: key.into(),
            cursor_pos: 0,
        });
        self
    }

    /// Add a form field with a pre-filled value.
    pub fn form_field_value(
        mut self,
        label: impl Into<String>,
        placeholder: impl Into<String>,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        self.view = PanelView::Form;
        self.filterable = false;
        let value_str = value.into();
        let key_str = key.into();
        self.items.push(PanelItem::FormField {
            label: label.into(),
            value: value_str.clone(),
            placeholder: placeholder.into(),
            key: key_str.clone(),
            cursor_pos: value_str.len(),
        });
        if !value_str.is_empty() {
            self.form_values.insert(key_str, value_str);
        }
        self
    }

    /// Add a hidden form value (no visible field) to be read by the submit factory.
    pub fn form_hidden(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.form_values.insert(key.into(), value.into());
        self
    }

    pub fn form_submit(mut self) -> Self {
        self.view = PanelView::Form;
        self.filterable = false;
        self.items.push(PanelItem::FormSubmit);
        self
    }

    pub fn with_filter(mut self) -> Self {
        self.filterable = true;
        self
    }

    /// Explicitly enable or disable fuzzy filtering for this panel.
    pub fn filterable(mut self, enabled: bool) -> Self {
        self.filterable = enabled;
        self
    }

    /// When true, the panel stays open after activating an item (Enter).
    pub fn keep_open(mut self) -> Self {
        self.keep_open_on_activate = true;
        self
    }

    /// Set whether the dialog can be dismissed from this (root) panel.
    pub fn closable(mut self, enabled: bool) -> Self {
        self.closable = enabled;
        self
    }

    /// Mark the root panel as non-closable. Alias for `closable(false)`.
    pub fn non_closable(self) -> Self {
        self.closable(false)
    }

    /// Add a header followed by items from a closure.
    pub fn section<F>(mut self, header: impl Into<String>, f: F) -> Self
    where
        F: FnOnce(Self) -> Self,
    {
        self = self.header(header);
        f(self)
    }

    /// Group items within a closure. Shorthand for `section` without a header.
    pub fn group<F>(self, f: F) -> Self
    where
        F: FnOnce(Self) -> Self,
    {
        f(self)
    }
}
