//! Inline slash-command dropdown state (grok parity: slash/mod.rs).
//!
//! Typing `/` at empty input opens an autocomplete dropdown anchored above
//! the input box instead of the centered command-palette modal. The dropdown
//! tracks the typed query, the filtered command matches, and the selected
//! row; Up/Down wrap, Enter accepts (submitting the `/cmd` through the normal
//! slash-command path), Esc closes and keeps the typed `/…` text.

use crate::commands::CommandRegistry;

/// A single dropdown row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlashMatch {
    pub name: String,
    pub desc: String,
}

/// Live dropdown state. `None` in view state means the dropdown is closed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SlashDropdown {
    /// Command-name part typed so far (without the leading `/`).
    pub query: String,
    /// Selected row index into `matches` (wraps).
    pub selected: usize,
    /// Filtered command rows.
    pub matches: Vec<SlashMatch>,
}

/// Max visible rows before the panel scrolls (grok `MAX_VISIBLE_SUGGESTIONS`).
pub const MAX_VISIBLE_ROWS: usize = 6;

impl SlashDropdown {
    pub fn open(registry: &CommandRegistry, query: &str) -> Self {
        Self { query: query.to_owned(), selected: 0, matches: Self::filter(registry, query) }
    }

    /// Name-ranked filter: commands whose NAME contains the query sort first
    /// (grok parity: name-match ranking), then desc-only matches — so a typed
    /// prefix like "/mode" selects the `mode` command, not a command whose
    /// description merely mentions "mode".
    pub(crate) fn filter(registry: &CommandRegistry, query: &str) -> Vec<SlashMatch> {
        let q = query.to_lowercase();
        let mut name_matches: Vec<SlashMatch> = registry
            .list()
            .into_iter()
            .filter(|d| d.name.to_lowercase().contains(&q))
            .map(|d| SlashMatch { name: d.name.clone(), desc: d.desc.clone() })
            .collect();
        name_matches.sort_by(|a, b| a.name.cmp(&b.name));
        let mut desc_matches: Vec<SlashMatch> = registry
            .list()
            .into_iter()
            .filter(|d| !d.name.to_lowercase().contains(&q) && d.desc.to_lowercase().contains(&q))
            .map(|d| SlashMatch { name: d.name.clone(), desc: d.desc.clone() })
            .collect();
        desc_matches.sort_by(|a, b| a.name.cmp(&b.name));
        name_matches.extend(desc_matches);
        name_matches
    }

    /// Refresh the query + matches from the current input text (called on
    /// every InputChanged echo while the dropdown is open).
    pub fn refresh(&mut self, registry: &CommandRegistry, input: &str) {
        let query = input.strip_prefix('/').unwrap_or(input).to_owned();
        self.query = query.clone();
        self.matches = Self::filter(registry, &query);
        self.selected = self.selected.min(self.matches.len().saturating_sub(1));
    }

    /// Move the selection by `delta` rows, wrapping around (grok rem_euclid).
    pub fn move_selection(&mut self, delta: isize) {
        if self.matches.is_empty() {
            return;
        }
        let len = self.matches.len() as isize;
        self.selected = ((self.selected as isize + delta).rem_euclid(len)) as usize;
    }

    pub fn selected_name(&self) -> Option<&str> {
        self.matches.get(self.selected).map(|m| m.name.as_str())
    }
}
