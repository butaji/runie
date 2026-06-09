use crate::model::AppState;
use crate::Event;

impl AppState {
    // === Session Event Handler ===
    pub(crate) fn session_event(&mut self, event: Event) {
        match event {
            Event::ToggleExpand => self.toggle_expand_all(),
            Event::ToggleSessionTree => self.toggle_session_tree_dialog(),
            Event::SessionTreeFilterCycle => self.cycle_session_tree_filter(),
            Event::ForkSession { message_index } => self.fork_session_at(message_index),
            Event::CloneSession => self.clone_session(),
            _ => {}
        }
    }

    pub(crate) fn toggle_session_tree_dialog(&mut self) {
        if matches!(self.open_dialog, Some(crate::commands::DialogState::SessionTree { .. })) {
            self.open_dialog = None;
        } else {
            self.open_dialog = Some(crate::commands::DialogState::SessionTree {
                filter: crate::session_tree::SessionTreeFilter::All,
                selected: 0,
            });
        }
        self.mark_dirty();
    }

    pub(crate) fn cycle_session_tree_filter(&mut self) {
        if let Some(crate::commands::DialogState::SessionTree { ref mut filter, .. }) = self.open_dialog {
            *filter = filter.cycle();
            self.mark_dirty();
        }
    }

    pub(crate) fn fork_session_at(&mut self, message_index: usize) {
        if let Some(ref mut tree) = self.session.session_tree {
            if let Some(path) = tree.fork_at(message_index) {
                tree.navigate_to(&path);
                self.add_system_msg(format!("Forked at message {}.", message_index));
            }
        } else {
            let mut tree = crate::session_tree::SessionTree::from_messages(&self.session.messages);
            if let Some(path) = tree.fork_at(message_index) {
                tree.navigate_to(&path);
                self.session.session_tree = Some(tree);
                self.add_system_msg(format!("Forked at message {}.", message_index));
            }
        }
    }

    pub(crate) fn clone_session(&mut self) {
        let tree = self.session.session_tree.clone().unwrap_or_else(|| {
            crate::session_tree::SessionTree::from_messages(&self.session.messages)
        });
        self.session.session_tree = Some(tree);
        self.add_system_msg("Session cloned at current position.".into());
    }
}
