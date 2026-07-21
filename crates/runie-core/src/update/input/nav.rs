//! Cursor & vim navigation.
//!
//! Cursor mutations are delegated to `InputActor` via `InputMsg`.
//! UI side effects (scroll clamp, ghost, dirty flag) are handled here.

use crate::model::AppState;

pub const PAGE_SIZE: usize = 5;

fn count_input_lines(input: &str) -> usize {
    if input.is_empty() {
        return 1;
    }
    input.lines().count().max(1)
}

impl AppState {
    /// Adjust `input_scroll` so the cursor is visible in the input box.
    pub(crate) fn clamp_input_scroll(&mut self) {
        let input = self.input();
        let total_lines = count_input_lines(&input.input);
        if total_lines <= 1 {
            self.input_mut().input_scroll = 0;
            return;
        }
        const MAX_INPUT_HEIGHT: usize = 10;
        const BORDER_ROWS: usize = 2;
        let visible_height = MAX_INPUT_HEIGHT.saturating_sub(BORDER_ROWS);
        if total_lines <= visible_height {
            self.input_mut().input_scroll = 0;
            return;
        }
        let pos = input.cursor_pos.min(input.input.len());
        let cursor_line = input.input[..pos].chars().filter(|&c| c == '\n').count();
        if cursor_line < input.input_scroll {
            self.input_mut().input_scroll = cursor_line;
        } else if cursor_line >= input.input_scroll + visible_height {
            self.input_mut().input_scroll = cursor_line.saturating_sub(visible_height - 1);
        }
        let max_scroll = total_lines.saturating_sub(visible_height);
        self.input_mut().input_scroll = self.input_mut().input_scroll.min(max_scroll);
    }

    pub(crate) fn move_cursor_to_line_start(&mut self) {
        try_send_input(self, crate::actors::InputMsg::CursorStart);
        self.clamp_input_scroll();
        self.view_mut().dirty = true;
    }

    pub(crate) fn move_cursor_to_line_end(&mut self) {
        try_send_input(self, crate::actors::InputMsg::CursorEnd);
        self.clamp_input_scroll();
        self.view_mut().dirty = true;
    }

    pub(crate) fn move_cursor_up(&mut self) {
        // Line-aware cursor move lives in `InputMsg::CursorLineUp` so the
        // InputActor (production) and the synchronous test path share one
        // implementation. Single-line drafts move to the start of the text;
        // history recall is handled by the nav-mode dispatch, not here.
        try_send_input(self, crate::actors::InputMsg::CursorLineUp);
        self.clamp_input_scroll();
        self.view_mut().dirty = true;
    }

    pub(crate) fn move_cursor_down(&mut self) {
        try_send_input(self, crate::actors::InputMsg::CursorLineDown);
        self.clamp_input_scroll();
        self.view_mut().dirty = true;
    }

    pub(crate) fn cursor_left(&mut self) {
        let at_start = self.input().cursor_pos == 0;
        try_send_input(self, crate::actors::InputMsg::CursorLeft);
        if at_start {
            self.input_mut().input_flash = 3;
        }
        self.clear_ghost();
        self.clamp_input_scroll();
        self.view_mut().dirty = true;
    }

    pub(crate) fn cursor_right(&mut self) {
        if self.input().ghost_completion.is_some() {
            self.accept_ghost();
            return;
        }
        let at_end = self.input().cursor_pos >= self.input().input.len();
        try_send_input(self, crate::actors::InputMsg::CursorRight);
        if at_end {
            self.input_mut().input_flash = 3;
        }
        self.clamp_input_scroll();
        self.view_mut().dirty = true;
    }

    pub(crate) fn cursor_start(&mut self) {
        if self.input().input.contains('\n') {
            self.move_cursor_to_line_start();
        } else if self.input().cursor_pos != 0 {
            try_send_input(self, crate::actors::InputMsg::CursorStart);
            self.clear_ghost();
            self.clamp_input_scroll();
            self.view_mut().dirty = true;
        } else {
            self.input_mut().input_flash = 3;
        }
    }

    pub(crate) fn cursor_end(&mut self) {
        if self.input().input.contains('\n') {
            self.move_cursor_to_line_end();
        } else if self.input().cursor_pos != self.input().input.len() {
            try_send_input(self, crate::actors::InputMsg::CursorEnd);
            self.clear_ghost();
            self.clamp_input_scroll();
            self.view_mut().dirty = true;
        } else {
            self.input_mut().input_flash = 3;
        }
    }

    pub(crate) fn cursor_word_left(&mut self) {
        try_send_input(self, crate::actors::InputMsg::CursorWordLeft);
        self.clear_ghost();
        self.clamp_input_scroll();
        self.view_mut().dirty = true;
    }

    pub(crate) fn cursor_word_right(&mut self) {
        try_send_input(self, crate::actors::InputMsg::CursorWordRight);
        self.clear_ghost();
        self.clamp_input_scroll();
        self.view_mut().dirty = true;
    }

    pub(crate) fn handle_vim_nav_char(&mut self, c: char) {
        // Close feed element detail overlay on Esc/q
        if self.view().feed_element_detail.is_some() {
            if c == 'q' || c == 'Q' {
                self.view_mut().feed_element_detail = None;
                self.view_mut().dirty = true;
                return;
            }
        }
        if c == ' ' {
            self.view_mut().vim_nav_mode = false;
            self.view_mut().selected_post = None;
            self.insert_char(' ');
            return;
        }
        if c == 'i' || c == 'I' {
            self.view_mut().vim_nav_mode = false;
            self.view_mut().selected_post = None;
            self.view_mut().dirty = true;
            return;
        }
        if let Some(handled) = self.try_vim_nav_motion(c) {
            if handled {
                return;
            }
        }
        if let Some(evt) = self.vim_motion_event(c) {
            self.update(evt);
            return;
        }
        self.view_mut().vim_nav_mode = false;
        self.view_mut().selected_post = None;
        self.insert_char(c);
    }

    pub(crate) fn try_vim_nav_motion(&mut self, c: char) -> Option<bool> {
        let last = self.snapshot().posts.len().saturating_sub(1);
        match c {
            'j' => Some(self.handle_vim_jump_down(last)),
            'k' => Some(self.handle_vim_jump_up()),
            'g' => {
                self.update(crate::Event::GoToTop);
                Some(true)
            }
            'G' => {
                self.update(crate::Event::GoToBottom);
                Some(true)
            }
            'y' => Some(self.handle_vim_copy(crate::Event::CopySelectedBlock)),
            'Y' => Some(self.handle_vim_copy(crate::Event::CopyBlockMetadata)),
            // Grok-style turn navigation: h/l jump between user prompt boundaries
            'h' => {
                crate::update::input::prev_turn(self);
                Some(true)
            }
            'l' => {
                crate::update::input::next_turn(self);
                Some(true)
            }
            // Grok-style response anchor nav: K/J snap to prev/next agent message
            'K' => {
                crate::update::input::prev_response(self);
                Some(true)
            }
            'J' => {
                crate::update::input::next_response(self);
                Some(true)
            }
            _ => None,
        }
    }

    fn handle_vim_jump_down(&mut self, last: usize) -> bool {
        if self.view_mut().selected_post.unwrap_or(0) >= last {
            self.view_mut().vim_nav_mode = false;
            self.view_mut().selected_post = None;
            self.view_mut().dirty = true;
            true
        } else {
            crate::update::input::element_jump_down(self);
            true
        }
    }

    fn handle_vim_jump_up(&mut self) -> bool {
        if self.view_mut().selected_post.unwrap_or(0) == 0 {
            self.input_mut().input_flash = 3;
            self.view_mut().dirty = true;
            true
        } else {
            crate::update::input::element_jump_up(self);
            true
        }
    }

    fn handle_vim_copy(&mut self, evt: crate::Event) -> bool {
        self.update(evt);
        self.view_mut().vim_nav_mode = false;
        self.view_mut().selected_post = None;
        self.view_mut().dirty = true;
        true
    }

    pub(crate) fn handle_vim_nav_event(&mut self, event: &crate::Event) -> Option<bool> {
        match event {
            crate::Event::Up | crate::Event::HistoryPrev => {
                self.vim_nav_up();
                Some(false)
            }
            crate::Event::Down | crate::Event::HistoryNext => {
                self.vim_nav_down();
                Some(false)
            }
            crate::Event::PageUp
            | crate::Event::PageDown
            | crate::Event::GoToTop
            | crate::Event::GoToBottom => {
                crate::update::input::scroll_event(self, event.clone());
                Some(false)
            }
            crate::Event::ToggleCommandPalette => {
                crate::update::dialog::dialog_toggle_event(
                    self,
                    crate::Event::ToggleCommandPalette,
                );
                Some(false)
            }
            // Enter in vim nav mode: on a subagent row open the subagent detail
            // overlay; on any other feed element open the feed_element_detail
            // overlay (Grok-style: Enter opens a full-detail dialog for the
            // element). On a collapsible (summarized) post — or one the user
            // already expanded individually — toggle that post only (the "Enter
            // expand" hint, grok's per-item Ctrl+E). Works in both global modes:
            // thoughts are collapsed by default, so per-item expansion must not
            // require global collapse first. On any other post it keeps the
            // legacy behavior: toggle global expand/collapse (same as Ctrl+O).
            crate::Event::Submit => {
                if let Some(sel) = self.view().selected_post {
                    let snap = self.snapshot();
                    if let Some(post) = snap.posts.get(sel) {
                        if post.kind == crate::view::elements::PostKind::SubagentRow {
                            if let Some(crate::view::elements::Element::SubagentRow { id, .. }) =
                                snap.elements.get(post.start)
                            {
                                self.view_mut().subagent_detail =
                                    Some(crate::model::SubagentDetail {
                                        worker_id: id.clone(),
                                        scroll: 0,
                                    });
                                self.view_mut().dirty = true;
                                return Some(true);
                            }
                        }
                        // Grok-style: Enter on any feed element opens the detail overlay.
                        // Skip elements that already have their own dedicated overlay
                        // (SubagentRow above) and skip non-visual kinds (Thinking).
                        use crate::model::feed_detail::FeedElementDetail;
                        if let Some(detail) =
                            FeedElementDetail::from_postkind(post.kind, post.start)
                        {
                            self.view_mut().feed_element_detail = Some(detail);
                            self.view_mut().dirty = true;
                            return Some(true);
                        }
                    }
                    let collapsible = snap.posts.get(sel).is_some_and(|p| !p.expanded);
                    let individually_expanded = self.view().expanded_posts.contains(&sel);
                    if collapsible || individually_expanded {
                        let set = &mut self.view_mut().expanded_posts;
                        if !set.remove(&sel) {
                            set.insert(sel);
                        }
                        self.messages_changed();
                        return Some(true);
                    }
                }
                self.update(crate::Event::ToggleExpand);
                Some(true)
            }
            _ => Some(true),
        }
    }

    pub(crate) fn vim_nav_up(&mut self) {
        if self.view_mut().selected_post.unwrap_or(0) == 0 {
            self.input_mut().input_flash = 3;
            self.view_mut().dirty = true;
        } else {
            crate::update::input::element_jump_up(self);
        }
    }

    pub(crate) fn vim_nav_down(&mut self) -> bool {
        let last = self.snapshot().posts.len().saturating_sub(1);
        if self.view_mut().selected_post.unwrap_or(0) >= last {
            self.view_mut().vim_nav_mode = false;
            self.view_mut().selected_post = None;
            self.view_mut().dirty = true;
            false
        } else {
            crate::update::input::element_jump_down(self);
            true
        }
    }

    pub(crate) fn vim_motion_event(&self, c: char) -> Option<crate::Event> {
        match c {
            'j' => Some(crate::Event::Up),
            'k' => Some(crate::Event::Down),
            'g' => Some(crate::Event::GoToTop),
            'G' => Some(crate::Event::GoToBottom),
            '/' => Some(crate::Event::ToggleCommandPalette),
            _ => None,
        }
    }
}

/// Fire-and-forget send to InputActor.
/// In test mode (no actor handles), applies the mutation synchronously so that
/// synchronous tests can assert on the updated state without awaiting the actor.
fn try_send_input(state: &mut AppState, msg: crate::actors::InputMsg) {
    if let Some(handles) = state.actor_handles() {
        let _ = handles.input.send_message(msg);
    } else {
        // Test mode: apply synchronously to AppState projection.
        msg.apply_to(state.input_mut());
    }
}
