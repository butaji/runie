//! Panel Builder Functions
//!
//! These are the public DSL constructors. They return `runie_core::dialog::Panel` values.

use super::super::panel_split::Panel;

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
    use super::super::ItemAction;

    #[test]
    fn test_panel_builder_chain() {
        let p = panel("test", "Test")
            .header("Section 1")
            .action("Option A", ItemAction::Close)
            .toggle("Enable", false, ItemAction::Toggle("enabled".into()))
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
            .with_filter()
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
            p.toggle("Option 1", false, ItemAction::Toggle("opt1".into()))
                .toggle("Option 2", true, ItemAction::Toggle("opt2".into()))
        });
        assert_eq!(p.items.len(), 3);
    }

    #[test]
    fn test_item_action_auto_label() {
        let p = panel("test", "Test").item_action(ItemAction::Push("next".into()));
        assert_eq!(p.items.len(), 1);
        // default_label() for Push("next") is "Go to next"
        assert_eq!(p.items[0].label(), Some("Go to next"));
    }
}
