use crate::model::AppState;

impl AppState {
    pub(crate) fn approve_edits(&mut self) {
        if self.session.pending_edits.is_empty() {
            self.add_system_msg("No pending edits to approve.".to_string());
            return;
        }
        let mut applied = 0;
        let mut errors = Vec::new();
        for preview in self.session.pending_edits.drain(..) {
            match std::fs::write(&preview.path, &preview.proposed) {
                Ok(()) => applied += 1,
                Err(e) => errors.push(format!("{}: {}", preview.path.display(), e)),
            }
        }
        let mut msg = format!("Applied {} edit(s).", applied);
        if !errors.is_empty() {
            msg.push_str(" Errors: ");
            msg.push_str(&errors.join(", "));
        }
        self.add_system_msg(msg);
    }

    pub(crate) fn reject_edits(&mut self) {
        let count = self.session.pending_edits.len();
        if count == 0 {
            self.add_system_msg("No pending edits to reject.".to_string());
            return;
        }
        self.session.pending_edits.clear();
        self.add_system_msg(format!("Rejected {} edit(s).", count));
    }
}
