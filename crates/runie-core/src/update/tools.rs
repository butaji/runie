#![allow(clippy::items_after_test_module)]
//! Bash command execution for ! prefix.
//!
//! All bash execution now lives in `runie_core::shell`.

// ── Form-submit and edit-event handling (merged from edit.rs) ─────────────────

use crate::model::AppState;
use crate::Event;

pub fn update(state: &mut AppState, event: Event) {
    match event {
        Event::PendingEdit { path, original, proposed } => {
            state
                .session
                .pending_edits
                .push(crate::edit_preview::EditPreview::new(
                    camino::Utf8PathBuf::from(path),
                    original,
                    proposed,
                ));
            state.view_mut().dirty = true;
        }
        Event::ApproveEdit => state.approve_edits(),
        Event::RejectEdit => state.reject_edits(),
        // intentionally ignored: other edit events fall through
        _ => {}
    }
}

// ── Edit approval/rejection (merged from edit_approval.rs) ───────────────────

impl AppState {
    /// Try to spawn IO write via actor_handles, else fallback to sync.
    fn try_spawn_io_write(&mut self) -> bool {
        // Clone handles first so we can borrow `self` mutably for drain.
        let handles = self.actor_handles().cloned();
        if let Some(h) = handles {
            let edits: Vec<(std::path::PathBuf, String)> = self
                .session_mut()
                .pending_edits
                .drain(..)
                .map(|p| {
                    let path: std::path::PathBuf = std::path::PathBuf::from(&p.path);
                    (path, p.proposed)
                })
                .collect();
            let _ = h.io.try_send(crate::actors::IoMsg::WriteFiles { edits });
            return true;
        }
        false
    }

    pub(crate) fn approve_edits(&mut self) {
        if self.session().pending_edits.is_empty() {
            self.add_system_msg("No pending edits to approve.".to_owned());
            return;
        }
        if self.try_spawn_io_write() {
            return;
        }
        let mut applied = 0;
        let mut errors = Vec::new();
        for preview in self.session_mut().pending_edits.drain(..) {
            let path = preview.path.clone();
            let content = preview.proposed.clone();
            match tokio::task::block_in_place(|| std::fs::write(&path, content)) {
                Ok(()) => applied += 1,
                Err(e) => errors.push(format!("{}: {}", path.as_str(), e)),
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
        let count = self.session().pending_edits.len();
        if count == 0 {
            self.add_system_msg("No pending edits to reject.".to_owned());
            return;
        }
        self.session_mut().pending_edits.clear();
        self.add_system_msg(format!("Rejected {} edit(s).", count));
    }
}
