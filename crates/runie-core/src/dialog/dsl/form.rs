//! Form Panel Builder - Fluent API for creating forms with submit handling

use crate::dialog::Panel;
use crate::Event;

/// Form panel builder with submit handling
#[derive(Debug, Clone)]
pub struct FormPanel {
    panel: Panel,
}

impl FormPanel {
    /// Create a new form panel
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        // Forms never use fuzzy filtering — keystrokes edit field values.
        let panel = Panel::new(id, title).form();
        Self { panel }
    }

    /// Add a form field
    pub fn field(
        mut self,
        label: impl Into<String>,
        placeholder: impl Into<String>,
        key: impl Into<String>,
    ) -> Self {
        self.panel = self.panel.field(label, placeholder, key);
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
        self.panel = self.panel.field_value(label, placeholder, key, value);
        self
    }

    /// Set the factory that produces the submit event from form values.
    pub fn on_submit(
        mut self,
        factory: fn(&std::collections::HashMap<String, String>) -> Event,
    ) -> Self {
        self.panel = self.panel.form_submit_with(factory);
        self
    }

    /// Build into Panel
    pub fn build(self) -> Panel {
        self.panel
    }

    /// Build into PanelStack
    pub fn into_stack(self) -> super::super::PanelStack {
        super::super::PanelStack::new(self.panel)
    }
}

impl From<FormPanel> for Panel {
    fn from(form: FormPanel) -> Self {
        form.panel
    }
}

/// Create a new form panel builder
pub fn form(id: impl Into<String>, title: impl Into<String>) -> FormPanel {
    FormPanel::new(id, title)
}

/// Read a form field value, returning an empty string when missing.
pub fn get_field(values: &std::collections::HashMap<String, String>, key: &str) -> String {
    values.get(key).cloned().unwrap_or_default()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::{form, get_field};
    use crate::dialog::dsl::panel;
    use crate::dialog::ItemAction;
    use crate::Event;

    fn save_submit(values: &std::collections::HashMap<String, String>) -> Event {
        crate::Event::RunSaveCommand {
            name: get_field(values, "name"),
        }
    }

    #[test]
    fn test_form_panel_builder() {
        let p = form("save", "Save")
            .field("Name", "session", "name")
            .field("Tags", "tag1, tag2", "tags")
            .on_submit(save_submit)
            .build();

        assert!(p.is_form());
        assert_eq!(p.items.len(), 3);
        assert!(p.submit_factory.is_some());
    }

    #[test]
    fn test_form_to_stack() {
        let stack = form("save", "Save")
            .field("Name", "session", "name")
            .on_submit(save_submit)
            .into_stack();

        assert_eq!(stack.len(), 1);
        assert!(stack.current().unwrap().is_form());
    }

    #[test]
    fn test_unified_panel_form_view() {
        let p = panel("save", "Save")
            .form()
            .field("Name", "session", "name")
            .action("Cancel", ItemAction::Close);

        assert!(p.is_form());
        assert!(!p.filterable);
    }

    #[test]
    fn test_get_field_returns_value() {
        let mut values = std::collections::HashMap::new();
        values.insert("name".into(), "session-a".into());
        assert_eq!(get_field(&values, "name"), "session-a");
    }

    #[test]
    fn test_get_field_returns_default() {
        let values = std::collections::HashMap::<String, String>::new();
        assert_eq!(get_field(&values, "missing"), "");
    }
}
