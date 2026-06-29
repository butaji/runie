use super::dialog::open_session_tree_dialog;
use crate::actors::TurnMsg;
use crate::commands::DialogKind;
use crate::model::AppState;
use crate::session::turn_queue::TurnQueue;

impl AppState {
    // === Session Event Handler ===

    pub(super) fn toggle_session_tree_dialog(&mut self) {
        use crate::commands::DialogState;
        if matches!(
            self.open_dialog,
            Some(DialogState::Active {
                kind: DialogKind::SessionTree,
                panels: _
            })
        ) {
            *self.open_dialog_mut() = None;
            self.view_mut().input_receiver = crate::model::InputReceiver::ChatInput;
            self.view_mut().dirty = true;
        } else {
            self.view_mut().cached_session_tree_valid = false;
            open_session_tree_dialog(self);
        }
    }

    pub(super) fn cycle_session_tree_filter(&mut self) {
        use crate::commands::DialogState;
        if let Some(DialogState::Active {
            kind: DialogKind::SessionTree,
            panels: stack,
        }) = &mut *self.open_dialog_mut()
        {
            if let Some(_panel) = stack.current_mut() {
                // cycle through filter variants based on panel id or custom logic
                // For now just mark dirty so the panel re-renders
                self.view_mut().dirty = true;
            }
        }
    }

    pub(super) fn fork_session_at(&mut self, message_index: usize) {
        if let Some(ref mut tree) = self.session_mut().session_tree {
            if let Some(path) = tree.fork_at(message_index) {
                tree.navigate_to(&path);
                self.add_system_msg(format!("Forked at message {}.", message_index));
            }
        } else {
            let mut tree =
                crate::session::tree::SessionTree::from_messages(&self.session_mut().messages);
            if let Some(path) = tree.fork_at(message_index) {
                tree.navigate_to(&path);
                self.session_mut().session_tree = Some(tree);
                self.add_system_msg(format!("Forked at message {}.", message_index));
            }
        }
    }

    pub(super) fn clone_session(&mut self) {
        let tree = self.session_mut().session_tree.clone().unwrap_or_else(|| {
            crate::session::tree::SessionTree::from_messages(&self.session_mut().messages)
        });
        self.session_mut().session_tree = Some(tree);
        self.add_system_msg("Session cloned at current position.".into());
    }

    pub(super) fn session_tree_select(&mut self, id: &str) {
        let navigated = self
            .session
            .session_tree
            .as_mut()
            .and_then(|tree| tree.find_path_by_id(id))
            .map(|path| {
                self.session
                    .session_tree
                    .as_mut()
                    .unwrap()
                    .navigate_to(&path);
                true
            })
            .unwrap_or(false);
        if navigated {
            *self.open_dialog_mut() = None;
            self.view_mut().input_receiver = crate::model::InputReceiver::ChatInput;
            self.add_system_msg("Switched to selected branch.".into());
        }
    }

    /// Replay a persisted message into the session without side effects.
    pub(crate) fn replay_message(
        &mut self,
        id: String,
        role: String,
        content: String,
        timestamp: f64,
        provider: String,
    ) {
        let role = crate::model::Role::parse(&role).unwrap_or(crate::model::Role::Assistant);
        self.session_mut().messages.push(crate::model::ChatMessage {
            role,
            timestamp,
            id,
            provider,
            parts: vec![runie_core::message::Part::Text { content }],
            ..Default::default()
        });
        self.messages_changed();
    }
}

// ── Message queue (merged from queue.rs) ─────────────────────────────────────

use super::now;
use crate::model::{ChatMessage, DeliveryMode, Role};

impl AppState {
    pub(crate) fn queue_follow_up(&mut self) {
        if self.input_mut().input.is_empty() {
            return;
        }
        let content = std::mem::take(&mut self.input_mut().input)
            .trim()
            .to_owned();
        self.input_mut().cursor_pos = 0;
        if content.is_empty() {
            return;
        }
        // Route through TurnActor to maintain authoritative queue state
        let handles = self.actor_handles().cloned();
        if let Some(ref h) = handles {
            if tokio::runtime::Handle::try_current().is_ok() {
                let _ = h.turn.try_send(TurnMsg::QueueFollowUp { content });
            } else {
                // Test mode: apply synchronously
                self.agent_state_mut()
                    .message_queue
                    .push(crate::model::QueuedMessage {
                        content,
                        kind: crate::model::QueuedMessageKind::FollowUp,
                    });
            }
        } else {
            // Test mode without handles: apply synchronously
            self.agent_state_mut()
                .message_queue
                .push(crate::model::QueuedMessage {
                    content,
                    kind: crate::model::QueuedMessageKind::FollowUp,
                });
        }
        self.view_mut().scroll = 0;
        self.view_mut().dirty = true;
    }

    pub(super) fn abort_queue(&mut self) {
        if self.completion_mut().at_suggestions.take().is_some() {
            self.completion_mut().at_selected = None;
            self.completion_mut().last_at_query = None;
            self.view_mut().dirty = true;
            return;
        }
        // Drain via TurnQueue, then emit facts
        let msgs: Vec<_> = TurnQueue::new(self.agent_state_mut().message_queue.drain(..).collect())
            .drain()
            .into_iter()
            .rev()
            .collect();
        for msg in msgs {
            self.apply_queue_aborted(msg.content);
        }
    }

    pub(crate) fn deliver_queued(&mut self) {
        if self.agent_state_mut().message_queue.is_empty() {
            return;
        }
        let steering_mode = self.config().steering_mode;
        let follow_up_mode = self.config().follow_up_mode;
        let handles = self.actor_handles().cloned();
        if let Some(ref h) = handles {
            if tokio::runtime::Handle::try_current().is_ok() {
                let _ = h.turn.try_send(TurnMsg::DeliverQueued {
                    steering_mode,
                    follow_up_mode,
                });
                self.agent_state_mut().message_queue.clear();
            } else {
                // Test mode without runtime: use TurnQueue directly
                self.deliver_via_turn_queue(steering_mode, follow_up_mode);
            }
        } else {
            // Test mode without handles: use TurnQueue directly
            self.deliver_via_turn_queue(steering_mode, follow_up_mode);
        }
        self.view_mut().scroll = 0;
    }

    /// Sync delivery via TurnQueue — replaces the old sync fallback methods.
    fn deliver_via_turn_queue(
        &mut self,
        steering_mode: DeliveryMode,
        follow_up_mode: DeliveryMode,
    ) {
        let mut queue = TurnQueue::new(std::mem::take(&mut self.agent_state_mut().message_queue));
        // Try steering first
        if let Some(r) = queue.pop_steering(steering_mode) {
            self.agent_state_mut().message_queue = queue.into_inner();
            self.push_user_from_queue(r.content);
            if follow_up_mode == DeliveryMode::All {
                // Also deliver follow-ups in All mode
                let mut q =
                    TurnQueue::new(std::mem::take(&mut self.agent_state_mut().message_queue));
                if let Some(r) = q.pop_all_follow_ups() {
                    self.agent_state_mut().message_queue = q.into_inner();
                    self.push_user_from_queue(r.content);
                } else {
                    self.agent_state_mut().message_queue = q.into_inner();
                }
            }
        } else if let Some(r) = queue.pop_follow_up(follow_up_mode) {
            self.agent_state_mut().message_queue = queue.into_inner();
            self.push_user_from_queue(r.content);
        } else {
            self.agent_state_mut().message_queue = queue.into_inner();
        }
    }

    fn push_user_from_queue(&mut self, content: String) {
        let id = self.next_id();
        self.session_mut().messages.push(ChatMessage {
            role: Role::User,
            timestamp: now(),
            id: id.clone(),
            parts: vec![runie_core::message::Part::Text {
                content: content.clone(),
            }],
            ..Default::default()
        });
        self.agent_state_mut()
            .request_queue
            .push_back((content, id));
        self.messages_changed();
    }

    pub(crate) fn dequeue(&mut self) {
        if let Some(msg) = self.agent_state_mut().message_queue.pop() {
            self.input_mut().input = msg.content;
            self.input_mut().cursor_pos = self.input_mut().input.len();
            self.view_mut().dirty = true;
        } else {
            self.input_mut().input_flash = 3;
            self.view_mut().dirty = true;
        }
    }
}

// ── Session event dispatcher ─────────────────────────────────────────────────

pub(super) fn handle_session_event(state: &mut AppState, event: crate::Event) {
    match event {
        crate::Event::ForkSession { message_index } => {
            state.fork_session_at(message_index);
            state.view_mut().cached_session_tree_valid = false;
        }
        crate::Event::CloneSession => {
            state.clone_session();
            state.view_mut().cached_session_tree_valid = false;
        }
        crate::Event::ToggleSessionTree => {
            state.toggle_session_tree_dialog();
            state.view_mut().cached_session_tree_valid = false;
        }
        crate::Event::SessionTreeFilterCycle => {
            state.cycle_session_tree_filter();
        }
        crate::Event::SessionTreeSelect { id } => {
            state.session_tree_select(&id);
        }
        // intentionally ignored: other session events fall through
        _ => {}
    }
}
