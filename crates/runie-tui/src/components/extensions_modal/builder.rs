//! Builder for ExtensionsModal

use super::{ExtensionsModal, ExtensionTab, FilterScope};

pub struct ExtensionsModalBuilder {
    active_tab: ExtensionTab,
    filter_scope: FilterScope,
}

impl ExtensionsModalBuilder {
    pub fn new() -> Self {
        Self {
            active_tab: ExtensionTab::Hooks,
            filter_scope: FilterScope::Workspace,
        }
    }

    pub fn active_tab(mut self, tab: ExtensionTab) -> Self {
        self.active_tab = tab;
        self
    }

    pub fn filter_scope(mut self, scope: FilterScope) -> Self {
        self.filter_scope = scope;
        self
    }

    pub fn build(self) -> ExtensionsModal {
        ExtensionsModal {
            active_tab: self.active_tab,
            search_query: String::new(),
            selected_index: 0,
            filter_scope: self.filter_scope,
            items: ExtensionsModal::mock_items_for_tab(self.active_tab),
            theme: crate::theme::ThemeWrapper::default(),
        }
    }
}

impl Default for ExtensionsModalBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_default() {
        let modal = ExtensionsModalBuilder::new().build();
        assert_eq!(modal.active_tab, ExtensionTab::Hooks);
        assert_eq!(modal.filter_scope, FilterScope::Workspace);
        assert!(modal.search_query.is_empty());
        assert_eq!(modal.selected_index, 0);
    }

    #[test]
    fn test_builder_with_tab() {
        let modal = ExtensionsModalBuilder::new()
            .active_tab(ExtensionTab::Plugins)
            .build();
        assert_eq!(modal.active_tab, ExtensionTab::Plugins);
        // Items should be for Plugins tab
        assert!(!modal.items.is_empty());
    }
}
