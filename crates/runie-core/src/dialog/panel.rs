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

    /// Number of items that can receive selection (not headers/separators).
    pub fn navigable_count(&self) -> usize {
        self.items
            .iter()
            .filter(|i| matches!(i, PanelItem::Action { .. } | PanelItem::Toggle { .. } | PanelItem::Select { .. }))
            .count()
    }

    /// Map a navigable-item index to the raw item index.
    pub fn raw_index(&self, nav_index: usize) -> Option<usize> {
        let mut nav = 0;
        for (i, item) in self.items.iter().enumerate() {
            if matches!(item, PanelItem::Action { .. } | PanelItem::Toggle { .. } | PanelItem::Select { .. }) {
                if nav == nav_index {
                    return Some(i);
                }
                nav += 1;
            }
        }
        None
    }

    /// Get the currently selected navigable item.
    pub fn selected_item(&self) -> Option<&PanelItem> {
        self.raw_index(self.selected).and_then(|i| self.items.get(i))
    }

    /// Mutable access to the currently selected navigable item.
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
}

/// A single row inside a panel.
#[derive(Debug, Clone, PartialEq)]
pub enum PanelItem {
    Action { label: String, action: ItemAction },
    Toggle { label: String, value: bool, key: String },
    Select { label: String, current: String, options: Vec<String>, key: String },
    Header(String),
    Separator,
}

impl PanelItem {
    pub fn label(&self) -> Option<&str> {
        match self {
            PanelItem::Action { label, .. } => Some(label),
            PanelItem::Toggle { label, .. } => Some(label),
            PanelItem::Select { label, .. } => Some(label),
            PanelItem::Header(text) => Some(text),
            PanelItem::Separator => None,
        }
    }

    pub fn is_navigable(&self) -> bool {
        matches!(self, PanelItem::Action { .. } | PanelItem::Toggle { .. } | PanelItem::Select { .. })
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
