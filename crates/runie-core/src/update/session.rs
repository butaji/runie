use crate::model::AppState;

impl AppState {
    // === Session Event Handler ===

    pub(super) fn toggle_session_tree_dialog(&mut self) {
        use crate::commands::DialogState;
        if matches!(self.open_dialog, Some(DialogState::SessionTree(_))) {
            self.open_dialog = None;
            self.mark_dirty();
        } else {
            self.open_session_tree_dialog();
        }
    }

    pub(super) fn cycle_session_tree_filter(&mut self) {
        use crate::commands::DialogState;
        if let Some(DialogState::SessionTree(stack)) = &mut self.open_dialog {
            if let Some(_panel) = stack.current_mut() {
                // cycle through filter variants based on panel id or custom logic
                // For now just mark dirty so the panel re-renders
                self.mark_dirty();
            }
        }
    }

    pub(super) fn fork_session_at(&mut self, message_index: usize) {
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

    pub(super) fn clone_session(&mut self) {
        let tree = self.session.session_tree.clone().unwrap_or_else(|| {
            crate::session_tree::SessionTree::from_messages(&self.session.messages)
        });
        self.session.session_tree = Some(tree);
        self.add_system_msg("Session cloned at current position.".into());
    }

    pub(super) fn session_tree_select(&mut self, id: &str) {
        let navigated = self
            .session
            .session_tree
            .as_mut()
            .and_then(|tree| tree.find_path_by_id(id))
            .map(|path| {
                self.session.session_tree.as_mut().unwrap().navigate_to(&path);
                true
            })
            .unwrap_or(false);
        if navigated {
            self.open_dialog = None;
            self.add_system_msg("Switched to selected branch.".into());
        }
    }
}
