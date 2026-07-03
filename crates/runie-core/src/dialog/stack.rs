use super::{ItemAction, Panel, PanelItem};

/// Identifier for a panel within a dialog.
pub type PanelId = String;

/// A stack of panels. The last panel is the visible one.
#[derive(Debug, Clone, PartialEq)]
pub struct PanelStack {
    pub panels: Vec<Panel>,
}

impl PanelStack {
    pub fn new(root: Panel) -> Self {
        Self { panels: vec![root] }
    }

    pub fn is_empty(&self) -> bool {
        self.panels.is_empty()
    }

    pub fn len(&self) -> usize {
        self.panels.len()
    }

    /// The currently visible panel (top of stack).
    pub fn current(&self) -> Option<&Panel> {
        self.panels.last()
    }

    pub fn current_mut(&mut self) -> Option<&mut Panel> {
        self.panels.last_mut()
    }

    /// The root panel of the stack.
    pub fn root(&self) -> Option<&Panel> {
        self.panels.first()
    }

    /// Push a new panel onto the stack.
    pub fn push(&mut self, panel: Panel) {
        self.panels.push(panel);
    }

    /// Pop the current panel, returning it. Returns `None` if only root remains.
    pub fn pop(&mut self) -> Option<Panel> {
        if self.panels.len() > 1 {
            self.panels.pop()
        } else {
            None
        }
    }

    /// Breadcrumb path of panel titles (without the rendering padding).
    pub fn breadcrumb(&self) -> Vec<&str> {
        self.panels.iter().map(|p| p.title.trim()).collect()
    }

    /// Navigate up in the current panel.
    pub fn select_up(&mut self) {
        if let Some(panel) = self.current_mut() {
            panel.select_up();
        }
    }

    /// Navigate down in the current panel.
    pub fn select_down(&mut self) {
        if let Some(panel) = self.current_mut() {
            panel.select_down();
        }
    }

    /// Push filter character into current panel.
    pub fn push_filter(&mut self, c: char) {
        if let Some(panel) = self.current_mut() {
            if panel.filterable {
                panel.push_filter(c);
            }
        }
    }

    /// Backspace filter in current panel.
    pub fn pop_filter(&mut self) {
        if let Some(panel) = self.current_mut() {
            if panel.filterable {
                panel.pop_filter();
            }
        }
    }

    /// Activate the currently selected item in the current panel.
    /// Returns the action to be handled by the caller. For Toggle items,
    /// the stored action is returned directly (e.g. `ItemAction::Toggle(key)`
    /// for settings, or `ItemAction::Emit(event)` for custom toggles).
    pub fn activate(&mut self) -> Option<ItemAction> {
        let panel = self.current_mut()?;
        let item = panel.selected_item()?;
        match item {
            PanelItem::Action { action, .. } | PanelItem::Command { action, .. } => {
                Some(action.clone())
            }
            PanelItem::Toggle { action, .. } => Some(action.clone()),
            PanelItem::Select { key, .. } => Some(ItemAction::Cycle(key.clone())),
            _ => None,
        }
    }
}
