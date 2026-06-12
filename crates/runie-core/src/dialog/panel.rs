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
    /// When true, activating an item (Enter) does NOT close the dialog.
    /// Useful for previews (e.g. theme picker) and live toggles.
    pub keep_open_on_activate: bool,
    /// For form panels: stores form values (key -> value)
    pub form_values: std::collections::HashMap<String, String>,
}

impl Panel {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            items: Vec::new(),
            selected: 0,
            filter: String::new(),
            // List-style panels are searchable by default. Forms should opt-out
            // explicitly since they use keyboard input for field editing.
            filterable: true,
            keep_open_on_activate: false,
            form_values: std::collections::HashMap::new(),
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

    /// Add a form field with a pre-filled value.
    pub fn form_field_value(
        mut self,
        label: impl Into<String>,
        placeholder: impl Into<String>,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        let value_str = value.into();
        let key_str = key.into();
        self.items.push(PanelItem::FormField {
            label: label.into(),
            value: value_str.clone(),
            placeholder: placeholder.into(),
            key: key_str.clone(),
        });
        if !value_str.is_empty() {
            self.form_values.insert(key_str, value_str);
        }
        self
    }

    pub fn form_submit(mut self) -> Self {
        // Submit button. The form's submit dispatch is owned by `form_build_submit`
        // in update/mod.rs, which matches on `panel.id` and reads form values.
        // No need to store an action here — `PanelItem::FormSubmit` is purely
        // the visible "Submit" item.
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
    /// Use for previews like theme picker or live toggles.
    pub fn keep_open(mut self) -> Self {
        self.keep_open_on_activate = true;
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

    /// Get raw indices of navigable items that match the current filter,
    /// sorted by match quality (best matches first).
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
                let label = item.label()?;
                let score = match_score(label, &q)?;
                Some((i, score))
            })
            .collect();
        scored.sort_by(|a, b| b.1.cmp(&a.1)); // descending by score
        scored.into_iter().map(|(i, _)| i).collect()
    }

    /// Items visible after applying the current filter.
    /// When filtering, only navigable items are returned, sorted by match quality.
    /// Headers that precede at least one matching item are kept so grouping stays
    /// visible; separators and orphan headers are dropped.
    pub fn filtered_items(&self) -> Vec<&PanelItem> {
        if !self.filterable || self.filter.is_empty() {
            return self.items.iter().collect();
        }
        let mut out = Vec::new();
        let mut pending_header: Option<&PanelItem> = None;
        for item in &self.items {
            match item {
                PanelItem::Header(_) => {
                    pending_header = Some(item);
                }
                _ if item.is_navigable() => {
                    if item.label().is_some_and(|l| match_score(l, &self.filter).is_some()) {
                        if let Some(h) = pending_header.take() {
                            out.push(h);
                        }
                        out.push(item);
                    }
                }
                _ => {}
            }
        }
        out
    }

    /// Number of items that can receive selection (not headers/separators).
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

    /// Returns true if this panel contains any form fields.
    pub fn is_form(&self) -> bool {
        self.items
            .iter()
            .any(|i| matches!(i, PanelItem::FormField { .. }))
    }

    /// Get the index of the currently selected form field (or None if not on a field).
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
        if let Some(PanelItem::FormField { value: v, key, .. }) = self.items.get_mut(field_index) {
            *v = value.clone();
            self.form_values.insert(key.clone(), value);
        }
    }

    /// Get all form values as a map.
    pub fn get_form_values(&self) -> &std::collections::HashMap<String, String> {
        &self.form_values
    }

    /// Find a button (Action item) in this panel whose label contains an
    /// accelerator matching the given character. Labels use "_X" syntax
    /// where X is the accelerator key, e.g. "_Submit" → 'S'.
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
}

/// Score how well a label matches a query. Higher is better.
/// Priority: startsWith > contains > fuzzy variations.
fn match_score(label: &str, query: &str) -> Option<isize> {
    if query.is_empty() {
        return Some(0);
    }
    let label_lower = label.to_lowercase();
    let query_lower = query.to_lowercase();

    if label_lower.starts_with(&query_lower) {
        return Some(10_000 + (100 - label.len() as isize).max(0));
    }
    if label_lower.contains(&query_lower) {
        return Some(5_000 + (100 - label.len() as isize).max(0));
    }
    fuzzy_score(&label_lower, &query_lower)
}

fn fuzzy_score(label_lower: &str, query_lower: &str) -> Option<isize> {
    let label_chars: Vec<char> = label_lower.chars().collect();
    let query_chars: Vec<char> = query_lower.chars().collect();
    let mut q_idx = 0;
    let mut prev_match_idx: Option<usize> = None;
    let mut score: isize = 1_000;

    for (l_idx, l_ch) in label_chars.iter().enumerate() {
        if q_idx >= query_chars.len() {
            break;
        }
        if *l_ch == query_chars[q_idx] {
            if q_idx == 0 && l_idx == 0 {
                score += 50;
            }
            if let Some(prev) = prev_match_idx {
                let gap = l_idx - prev;
                if gap == 1 {
                    score += 20;
                } else {
                    score -= gap as isize * 5;
                }
            }
            prev_match_idx = Some(l_idx);
            q_idx += 1;
        }
    }

    if q_idx == query_chars.len() {
        Some(score)
    } else {
        None
    }
}

/// Parse an accelerator from a label like "_Submit" → Some('S').
/// The underscore is removed from display text.
pub fn parse_accel(label: &str) -> Option<char> {
    let mut chars = label.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '_' {
            return chars.next();
        }
    }
    None
}

/// Strip accelerator underscores from a label for display.
pub fn strip_accel(label: &str) -> String {
    label.replace('_', "")
}

/// A single row inside a panel.
#[derive(Debug, Clone, PartialEq)]
pub enum PanelItem {
    Action {
        label: String,
        action: ItemAction,
    },
    Toggle {
        label: String,
        value: bool,
        key: String,
    },
    Select {
        label: String,
        current: String,
        options: Vec<String>,
        key: String,
    },
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
