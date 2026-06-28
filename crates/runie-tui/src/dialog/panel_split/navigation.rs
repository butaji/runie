//! Panel navigation and filter methods.
//!
//! Split from dialog/panel.rs to stay under the 500-line limit.

use super::{Panel, PanelItem};

impl Panel {
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
                let score = super::super::score::item_match_score(item, &q)?;
                Some((i, score))
            })
            .collect();
        scored.sort_by_key(|b| std::cmp::Reverse(b.1));
        scored.into_iter().map(|(i, _)| i).collect()
    }
}
