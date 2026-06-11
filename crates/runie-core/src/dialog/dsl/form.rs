//! Form Panel Builder - Fluent API for creating forms with submit handling

use super::{Panel, panel, PanelItem, ItemAction};
use crate::Event;

/// Form panel builder with submit handling
#[derive(Debug, Clone)]
pub struct FormPanel {
    panel: Panel,
    submit_event: Option<Event>,
}

impl FormPanel {
    /// Create a new form panel
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            panel: Panel::new(id, title),
            submit_event: None,
        }
    }

    /// Add a form field
    pub fn field(mut self, label: impl Into<String>, placeholder: impl Into<String>, key: impl Into<String>) -> Self {
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

    /// Set the event to emit on submit
    pub fn on_submit(mut self, event: Event) -> Self {
        self.submit_event = Some(event.clone());
        self.panel.last_action = Some(ItemAction::Emit(event));
        self.panel.items.push(PanelItem::FormSubmit);
        self
    }

    /// Build into Panel
    pub fn build(self) -> Panel {
        self.panel
    }

    /// Build into PanelStack
    pub fn into_stack(self) -> super::super::PanelStack {
        super::super::PanelStack::new(self.panel.into_core())
    }
}

/// Create a new form panel builder
pub fn form(id: impl Into<String>, title: impl Into<String>) -> FormPanel {
    FormPanel::new(id, title)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_form_panel_builder() {
        let p = form("save", "Save")
            .field("Name", "session", "name")
            .field("Tags", "tag1, tag2", "tags")
            .on_submit(Event::RunSaveCommand { name: String::new() })
            .build();

        assert!(p.is_form());
        assert_eq!(p.items.len(), 3);
    }

    #[test]
    fn test_form_to_stack() {
        let stack = form("save", "Save")
            .field("Name", "session", "name")
            .on_submit(Event::RunSaveCommand { name: String::new() })
            .into_stack();

        assert_eq!(stack.len(), 1);
        assert!(stack.current().unwrap().is_form());
    }
}
