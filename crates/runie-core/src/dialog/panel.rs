use crate::Event;

/// A single panel inside a dialog — title + list of items + selection state.
#[derive(Debug, Clone, PartialEq)]
pub struct Panel {
    pub id: String,
    pub title: String,
    pub items: Vec<PanelItem>,
    pub selected: usize,
    /// Optional filter text when the panel is filterable.
    pub filter: String,
    pub filterable: bool,
    /// For form panels: stores form values (key -> value)
    pub form_values: std::collections::HashMap<String, String>,
    /// For form panels: last action to execute on submit
    #[allow(dead_code)]
    last_action: Option<ItemAction>,
}

impl Panel {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            items: Vec::new(),
            selected: 0,
            filter: String::new(),
            filterable: false,
            form_values: std::collections::HashMap::new(),
            last_action: None,
        }
    }

    pub fn item(mut self, label: impl Into<String>, action: ItemAction) -> Self {
        self.items.push(PanelItem::Action {
            label: label.into(),
            action,
        });
        self
    }

    pub fn toggle(mut self, label: impl Into<String>, value: bool, key: impl Into<String>) -> Self {
        self.items.push(PanelItem::Toggle {
            label: label.into(),
            value,
            key: key.into(),
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

    pub fn form_field(
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

    pub fn form_submit(mut self, action: ItemAction) -> Self {
        self.items.push(PanelItem::FormSubmit);
        self.last_action = Some(action);
        self
    }

    pub fn with_filter(mut self) -> Self {
        self.filterable = true;
        self
    }

    /// Move selection up, wrapping around. Returns the new index.
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

    /// Move selection down, wrapping around. Returns the new index.
    pub fn select_down(&mut self) -> usize {
        let count = self.navigable_count();
        if count == 0 {
            return 0;
        }
        self.selected = (self.selected + 1) % count;
        self.selected
    }

    /// Items visible after applying the current filter.
    /// Headers/separators are kept if any navigable item below them matches.
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
                    if item.label().map_or(false, |l| l.to_lowercase().contains(&q)) {
                        result.extend(pending_headers.drain(..));
                        result.push(item);
                    }
                }
            }
        }
        result
    }

    /// Number of items that can receive selection (not headers/separators).
    pub fn navigable_count(&self) -> usize {
        self.filtered_items()
            .into_iter()
            .filter(|i| i.is_navigable())
            .count()
    }

    /// Map a navigable-item index to the raw item index in the ORIGINAL list.
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

    /// Get the currently selected navigable item from the FILTERED list.
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
    /// NOTE: This uses the ORIGINAL list since we need mutable access.
    pub fn selected_item_mut(&mut self) -> Option<&mut PanelItem> {
        self.raw_index(self.selected).and_then(|i| self.items.get_mut(i))
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

    /// Returns true if this panel contains any form fields.
    pub fn is_form(&self) -> bool {
        self.items.iter().any(|i| matches!(i, PanelItem::FormField { .. }))
    }

    /// Get the index of the currently selected form field (or None if not on a field).
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

    /// Update the value of a form field by its index.
    pub fn set_form_value(&mut self, field_index: usize, value: String) {
        if let Some(PanelItem::FormField { value: v, key, .. }) = self.items.get_mut(field_index) {
            *v = value.clone();
            self.form_values.insert(key.clone(), value);
        }
    }

    /// Get all form values as a map.
    pub fn get_form_values(&self) -> &std::collections::HashMap<String, String> {
        &self.form_values
    }
}

/// A single row inside a panel.
#[derive(Debug, Clone, PartialEq)]
pub enum PanelItem {
    Action { label: String, action: ItemAction },
    Toggle { label: String, value: bool, key: String },
    Select { label: String, current: String, options: Vec<String>, key: String },
    FormField {
        label: String,
        value: String,
        placeholder: String,
        key: String,
    },
    FormSubmit,
    Header(String),
    Separator,
}

impl PanelItem {
    pub fn label(&self) -> Option<&str> {
        match self {
            PanelItem::Action { label, .. } => Some(label),
            PanelItem::Toggle { label, .. } => Some(label),
            PanelItem::Select { label, .. } => Some(label),
            PanelItem::FormField { label, .. } => Some(label),
            PanelItem::FormSubmit => Some("Submit"),
            PanelItem::Header(text) => Some(text),
            PanelItem::Separator => None,
        }
    }

    pub fn is_navigable(&self) -> bool {
        matches!(
            self,
            PanelItem::Action { .. }
                | PanelItem::Toggle { .. }
                | PanelItem::Select { .. }
                | PanelItem::FormField { .. }
                | PanelItem::FormSubmit
        )
    }
}

/// What happens when a navigable panel item is activated.
#[derive(Debug, Clone, PartialEq)]
pub enum ItemAction {
    /// Navigate to another panel by id.
    Push(String),
    /// Go back to the previous panel.
    Pop,
    /// Close the entire dialog.
    Close,
    /// Emit an event to the main app.
    Emit(Event),
    /// Toggle a boolean setting by key.
    Toggle(String),
    /// Cycle a multi-choice setting by key.
    Cycle(String),
}
