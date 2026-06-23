//! Panel state and builder methods.

use super::item::parse_accel;
use super::score::item_match_score;
use super::{ItemAction, PanelItem};
use crate::Event;

/// Function that builds the submit event from collected form values.
pub type FormSubmitFn = fn(&std::collections::HashMap<String, String>) -> Event;

/// Visual layout of a panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PanelView {
    /// Scrollable list with fuzzy search.
    #[default]
    List,
    /// Form with labeled fields and bottom button bar.
    Form,
}

/// A single panel inside a dialog — title + list of items + selection state.
#[allow(unpredictable_function_pointer_comparisons)]
#[derive(Debug, Clone, PartialEq)]
pub struct Panel {
    pub id: String,
    pub title: String,
    pub items: Vec<PanelItem>,
    pub selected: usize,
    /// Optional filter text when the panel is filterable.
    pub filter: String,
    pub filterable: bool,
    /// When true, activating an item (Enter) does NOT close the dialog.
    /// Useful for previews (e.g. theme picker) and live toggles.
    pub keep_open_on_activate: bool,
    /// When false, the dialog cannot be dismissed from the root panel
    /// (Esc/DialogBack, Abort, Quit are ignored). ForceQuit still works.
    pub closable: bool,
    /// For form panels: stores form values (key -> value)
    pub form_values: std::collections::HashMap<String, String>,
    /// For form panels: factory that turns form values into the submit event.
    pub submit_factory: Option<FormSubmitFn>,
    /// Visual layout of this panel.
    pub view: PanelView,
}

impl Panel {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: Self::normalize_title(title),
            items: Vec::new(),
            selected: 0,
            filter: String::new(),
            // List-style panels are searchable by default. Forms should opt-out
            // explicitly since they use keyboard input for field editing.
            filterable: true,
            keep_open_on_activate: false,
            closable: true,
            form_values: std::collections::HashMap::new(),
            submit_factory: None,
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

    /// Set the panel title, normalizing it to exactly one leading and
    /// trailing space.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Self::normalize_title(title);
        self
    }

    /// Alias for `with_title()`.
    pub fn title(self, title: impl Into<String>) -> Self {
        self.with_title(title)
    }

    pub fn item(mut self, label: impl Into<String>, action: ItemAction) -> Self {
        self.items.push(PanelItem::Action {
            label: label.into(),
            action,
        });
        self
    }

    /// Add an action item. Alias for `item()`.
    pub fn action(mut self, label: impl Into<String>, action: ItemAction) -> Self {
        self.items.push(PanelItem::Action {
            label: label.into(),
            action,
        });
        self
    }

    /// Add an item with auto-generated label from action's default label.
    pub fn item_action(mut self, action: ItemAction) -> Self {
        let label = action.default_label();
        self.items.push(PanelItem::Action { label, action });
        self
    }

    /// Add a command-palette entry with separate name and description.
    pub fn command(
        mut self,
        name: impl Into<String>,
        desc: impl Into<String>,
        action: ItemAction,
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
    pub fn toggle(mut self, label: impl Into<String>, value: bool, action: ItemAction) -> Self {
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

    pub fn form_submit_with(mut self, factory: FormSubmitFn) -> Self {
        self.view = PanelView::Form;
        self.filterable = false;
        self.submit_factory = Some(factory);
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

    /// Move selection up, wrapping around.
    pub fn select_up(&mut self) -> usize {
        let count = self.navigable_count();
        if count == 0 {
            return 0;
        }
        self.selected = if self.selected == 0 {
            count - 1
        } else {
            self.selected - 1
        };
        self.selected
    }

    /// Move selection down, wrapping around.
    pub fn select_down(&mut self) -> usize {
        let count = self.navigable_count();
        if count == 0 {
            return 0;
        }
        self.selected = (self.selected + 1) % count;
        self.selected
    }

    /// Items visible after applying the current filter.
    pub fn filtered_items(&self) -> Vec<&PanelItem> {
        if !self.filterable || self.filter.is_empty() {
            return self.items.iter().collect();
        }
        let filtered = self
            .filtered_navigable_indices()
            .iter()
            .map(|&i| &self.items[i])
            .collect::<Vec<_>>();
        // If filter has matches, return filtered results
        // Otherwise, fall back to all navigable items (no filter)
        if !filtered.is_empty() {
            filtered
        } else {
            self.items.iter().filter(|i| i.is_navigable()).collect()
        }
    }

    /// Returns true if the current filter has at least one match.
    pub fn has_filter_matches(&self) -> bool {
        self.filter.is_empty() || !self.filtered_navigable_indices().is_empty()
    }

    /// Number of items that can receive selection.
    pub fn navigable_count(&self) -> usize {
        if !self.filterable || self.filter.is_empty() {
            self.items.iter().filter(|i| i.is_navigable()).count()
        } else {
            self.filtered_navigable_indices().len()
        }
    }

    /// Map a navigable-item index to the raw item index.
    pub fn raw_index(&self, nav_index: usize) -> Option<usize> {
        if !self.filterable || self.filter.is_empty() {
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
        } else {
            self.filtered_navigable_indices().get(nav_index).copied()
        }
    }

    /// Get the currently selected navigable item from the filtered list.
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

    /// Mutable access to the currently selected navigable item.
    pub fn selected_item_mut(&mut self) -> Option<&mut PanelItem> {
        self.raw_index(self.selected)
            .and_then(|i| self.items.get_mut(i))
    }

    /// Append a filter character.
    pub fn push_filter(&mut self, c: char) {
        self.filter.push(c);
        self.selected = 0;
    }
    /// Backspace in filter.
    pub fn pop_filter(&mut self) {
        self.filter.pop();
        self.selected = 0;
    }
    /// Set filter and reset selection to 0.
    pub fn set_filter(&mut self, filter: &str) {
        self.filter.clear();
        self.filter.push_str(filter);
        self.selected = 0;
    }

    /// Returns true if this panel renders as a form view.
    pub fn is_form(&self) -> bool {
        matches!(self.view, PanelView::Form)
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

    /// Find a button whose label contains an accelerator matching `c`.
    pub fn find_button_by_accel(&self, c: char) -> Option<&ItemAction> {
        let c = c.to_ascii_lowercase();
        for item in &self.items {
            if let PanelItem::Action { label, action } = item {
                if let Some(accel) = parse_accel(label) {
                    if accel.to_ascii_lowercase() == c {
                        return Some(action);
                    }
                }
            }
        }
        None
    }

    /// Raw indices of navigable items that match the current filter.
    fn filtered_navigable_indices(&self) -> Vec<usize> {
        if !self.filterable || self.filter.is_empty() {
            return self
                .items
                .iter()
                .enumerate()
                .filter(|(_, i)| i.is_navigable())
                .map(|(i, _)| i)
                .collect();
        }
        let q = self.filter.to_lowercase();
        let mut scored: Vec<(usize, isize)> = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, i)| i.is_navigable())
            .filter_map(|(i, item)| {
                let score = super::score::item_match_score(item, &q)?;
                Some((i, score))
            })
            .collect();
        scored.sort_by_key(|b| std::cmp::Reverse(b.1));
        scored.into_iter().map(|(i, _)| i).collect()
    }

    /// Normalize title: one leading/trailing space, trimmed empty titles stay empty.
    pub(crate) fn normalize_title(title: impl Into<String>) -> String {
        let trimmed = title.into().trim().to_string();
        if trimmed.is_empty() {
            trimmed
        } else {
            format!(" {} ", trimmed)
        }
    }
}
