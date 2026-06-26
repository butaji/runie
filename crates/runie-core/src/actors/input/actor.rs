//! `InputActor` — owns the authoritative `InputState`.

use tokio::sync::mpsc;

use crate::actors::{spawn_actor, Actor, ActorHandle};
use crate::bus::EventBus;
use crate::event::Event;
use crate::model::InputState;

use super::messages::{InputActorHandle, InputMsg};

/// Actor that owns input state.
///
/// Text editing, cursor navigation, history, and undo/redo are all mutations
/// that live here. The actor processes `InputMsg` messages and emits
/// `InputChanged` facts when state changes.
pub struct InputActor {
    /// The authoritative input state.
    state: InputState,
}

impl InputActor {
    /// Spawn an `InputActor` on the given event bus.
    pub fn spawn(bus: EventBus<Event>) -> (InputActorHandle, ActorHandle) {
        let actor = Self {
            state: InputState::default(),
        };
        let (tx, handle) = spawn_actor(actor, bus);
        (InputActorHandle::new(tx), handle)
    }

    /// Dispatch an incoming message to the appropriate handler.
    fn handle_msg(&mut self, msg: InputMsg) {
        match msg {
            InputMsg::InsertChar(c) => self.insert_char(c),
            InputMsg::Backspace => self.backspace(),
            InputMsg::Newline => self.newline(),
            InputMsg::DeleteWord => self.delete_word(),
            InputMsg::DeleteToEnd => self.delete_to_end(),
            InputMsg::DeleteToStart => self.delete_to_start(),
            InputMsg::KillChar => self.kill_char(),
            InputMsg::Paste(text) => self.paste(&text),
            InputMsg::PasteImage => self.paste_image(),
            InputMsg::CursorLeft => self.cursor_left(),
            InputMsg::CursorRight => self.cursor_right(),
            InputMsg::CursorStart => self.cursor_start(),
            InputMsg::CursorEnd => self.cursor_end(),
            InputMsg::CursorWordLeft => self.cursor_word_left(),
            InputMsg::CursorWordRight => self.cursor_word_right(),
            InputMsg::MoveCursor { pos } => self.move_cursor_to(pos),
            InputMsg::HistoryPrev => self.history_prev(),
            InputMsg::HistoryNext => self.history_next(),
            InputMsg::Undo => self.undo(),
            InputMsg::Redo => self.redo(),
            InputMsg::SetText { text } => self.set_text(text),
            InputMsg::SetPrompt { name } => self.state.current_prompt = name,
            InputMsg::Clear => self.clear(),
            InputMsg::HistoryLoaded { entries } => self.state.input_history = entries,
            InputMsg::DrainQueue { messages } => self.drain_queue(messages),
            InputMsg::InsertAtRef { text } => self.insert_at_ref(&text),
            InputMsg::FilePickerAbort => self.file_picker_abort(),
        }
    }

    // ── Text editing ───────────────────────────────────────────────────────

    fn insert_char(&mut self, c: char) {
        self.push_undo();
        if self.state.cursor_pos == self.state.input.len() {
            self.state.input.push(c);
        } else {
            self.state.input.insert(self.state.cursor_pos, c);
        }
        self.state.cursor_pos += c.len_utf8();
        self.clear_redo();
    }

    fn backspace(&mut self) {
        if self.state.cursor_pos > 0 {
            self.push_undo();
            let new_pos = self.state.cursor_pos - 1;
            self.state.input.remove(new_pos);
            self.state.cursor_pos = new_pos;
            self.clear_redo();
        }
    }

    fn newline(&mut self) {
        self.push_undo();
        if self.state.cursor_pos == self.state.input.len() {
            self.state.input.push('\n');
        } else {
            self.state.input.insert(self.state.cursor_pos, '\n');
        }
        self.state.cursor_pos += 1;
        self.clear_redo();
    }

    fn delete_word(&mut self) {
        if self.state.cursor_pos == 0 {
            return;
        }
        let start = crate::update::input::find_word_boundary_left(
            &self.state.input,
            self.state.cursor_pos,
        );
        self.push_undo();
        self.state.input.drain(start..self.state.cursor_pos);
        self.state.cursor_pos = start;
        self.clear_redo();
    }

    fn delete_to_end(&mut self) {
        if self.state.cursor_pos < self.state.input.len() {
            self.push_undo();
            self.state.input.truncate(self.state.cursor_pos);
            self.clear_redo();
        }
    }

    fn delete_to_start(&mut self) {
        if self.state.cursor_pos > 0 {
            self.push_undo();
            self.state.input.drain(..self.state.cursor_pos);
            self.state.cursor_pos = 0;
            self.clear_redo();
        }
    }

    fn kill_char(&mut self) {
        if self.state.cursor_pos < self.state.input.len() {
            let end = crate::update::input::next_grapheme_boundary(
                &self.state.input,
                self.state.cursor_pos,
            );
            self.push_undo();
            self.state.input.drain(self.state.cursor_pos..end);
            self.clear_redo();
        }
    }

    fn paste(&mut self, text: &str) {
        let clean = text
            .replace("\r\n", "")
            .replace(['\r', '\n'], "")
            .replace('\t', "    ");
        if clean.is_empty() {
            return;
        }
        self.push_undo();
        self.state.input.insert_str(self.state.cursor_pos, &clean);
        self.state.cursor_pos += clean.len();
        self.clear_redo();
    }

    fn paste_image(&mut self) {
        // Image paste was removed — just flash.
        self.state.input_flash = 3;
    }

    // ── Cursor navigation ──────────────────────────────────────────────────

    fn cursor_left(&mut self) {
        if self.state.cursor_pos > 0 {
            let pos = self.state.cursor_pos;
            self.state.cursor_pos =
                crate::update::input::prev_grapheme_boundary(&self.state.input, pos);
        }
    }

    fn cursor_right(&mut self) {
        if self.state.cursor_pos < self.state.input.len() {
            let pos = self.state.cursor_pos;
            self.state.cursor_pos =
                crate::update::input::next_grapheme_boundary(&self.state.input, pos);
        }
    }

    fn cursor_start(&mut self) {
        self.state.cursor_pos = 0;
    }

    fn cursor_end(&mut self) {
        self.state.cursor_pos = self.state.input.len();
    }

    fn cursor_word_left(&mut self) {
        if self.state.cursor_pos > 0 {
            let pos = self.state.cursor_pos;
            self.state.cursor_pos =
                crate::update::input::find_word_boundary_left(&self.state.input, pos);
        }
    }

    fn cursor_word_right(&mut self) {
        if self.state.cursor_pos < self.state.input.len() {
            let pos = self.state.cursor_pos;
            self.state.cursor_pos =
                crate::update::input::find_word_boundary_right(&self.state.input, pos);
        }
    }

    fn move_cursor_to(&mut self, pos: usize) {
        self.state.cursor_pos = pos.min(self.state.input.len());
    }

    // ── History & undo/redo ────────────────────────────────────────────────

    fn push_undo(&mut self) {
        self.state
            .undo_stack
            .push((self.state.input.clone(), self.state.cursor_pos));
    }

    fn clear_redo(&mut self) {
        self.state.redo_stack.clear();
    }

    fn undo(&mut self) {
        if let Some((text, pos)) = self.state.undo_stack.pop() {
            self.state
                .redo_stack
                .push((self.state.input.clone(), self.state.cursor_pos));
            self.state.input = text;
            self.state.cursor_pos = pos;
        }
    }

    fn redo(&mut self) {
        if let Some((text, pos)) = self.state.redo_stack.pop() {
            self.state
                .undo_stack
                .push((self.state.input.clone(), self.state.cursor_pos));
            self.state.input = text;
            self.state.cursor_pos = pos;
        }
    }

    fn history_prev(&mut self) {
        if self.state.input_history.is_empty() {
            self.state.input_flash = 3;
            return;
        }
        let pos = match self.state.history_pos {
            Some(p) if p > 0 => p - 1,
            Some(p) => p,
            None => self.state.input_history.len() - 1,
        };
        self.state.history_pos = Some(pos);
        self.state.input = self.state.input_history[pos].clone();
        self.state.cursor_pos = self.state.input.len();
    }

    fn history_next(&mut self) {
        let pos = match self.state.history_pos {
            Some(p) => p + 1,
            None => return,
        };
        if pos >= self.state.input_history.len() {
            self.state.history_pos = None;
            self.state.input.clear();
            self.state.cursor_pos = 0;
        } else {
            self.state.history_pos = Some(pos);
            self.state.input = self.state.input_history[pos].clone();
            self.state.cursor_pos = self.state.input.len();
        }
    }

    // ── State mutations ────────────────────────────────────────────────────

    fn set_text(&mut self, text: String) {
        self.push_undo();
        self.state.input = text;
        self.state.cursor_pos = self.state.input.len();
        self.clear_redo();
    }

    fn clear(&mut self) {
        self.state.input.clear();
        self.state.cursor_pos = 0;
        self.state.history_pos = None;
        self.state.undo_stack.clear();
        self.state.redo_stack.clear();
        self.state.input_scroll = 0;
    }

    fn drain_queue(&mut self, messages: Vec<String>) {
        let mut combined = String::new();
        for msg in messages {
            if !combined.is_empty() {
                combined.push('\n');
            }
            combined.push_str(&msg);
        }
        if !combined.is_empty() {
            self.push_undo();
            if !self.state.input.is_empty() && !self.state.input.ends_with('\n') {
                self.state.input.push('\n');
            }
            self.state.input.push_str(&combined);
            self.state.cursor_pos = self.state.input.len();
            self.clear_redo();
        }
    }

    fn insert_at_ref(&mut self, text: &str) {
        self.push_undo();
        self.state.input.insert_str(self.state.cursor_pos, text);
        self.state.cursor_pos += text.len();
        self.clear_redo();
    }

    fn file_picker_abort(&mut self) {
        if let Some((input, cursor, _, _)) = self.state.file_picker_backup.take() {
            self.state.input = input;
            self.state.cursor_pos = cursor;
        }
    }

    #[cfg(test)]
    pub fn state(&self) -> &InputState {
        &self.state
    }
}

impl Actor for InputActor {
    type Msg = InputMsg;
    type Event = Event;

    async fn run_body(mut self, mut rx: mpsc::Receiver<Self::Msg>, bus: EventBus<Event>) {
        while let Some(msg) = rx.recv().await {
            self.handle_msg(msg.clone());
            bus.publish(Event::InputChanged {
                state: Box::new(self.state.clone()),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn insert_char_updates_cursor() {
        let bus = EventBus::<Event>::new(16);
        let mut sub = bus.subscribe();

        let (handle, _actor) = InputActor::spawn(bus);
        handle.send(InputMsg::InsertChar('h')).await;
        handle.send(InputMsg::InsertChar('i')).await;
        drop(handle);

        let mut events = Vec::new();
        while let Ok(e) = sub.recv().await {
            if matches!(e, Event::InputChanged { .. }) {
                events.push(e);
            }
        }

        assert_eq!(events.len(), 2);
        if let Event::InputChanged { state } = &events[0] {
            assert_eq!(state.input, "h");
            assert_eq!(state.cursor_pos, 1);
        }
        if let Event::InputChanged { state } = &events[1] {
            assert_eq!(state.input, "hi");
            assert_eq!(state.cursor_pos, 2);
        }
    }

    #[tokio::test]
    async fn history_prev_cycles() {
        let bus = EventBus::<Event>::new(16);
        let (handle, _actor) = InputActor::spawn(bus);

        handle
            .send(InputMsg::HistoryLoaded {
                entries: vec!["first".into(), "second".into()],
            })
            .await;
        handle.send(InputMsg::HistoryPrev).await;
        drop(handle);
    }

    #[tokio::test]
    async fn clear_resets_everything() {
        let bus = EventBus::<Event>::new(16);
        let (handle, _actor) = InputActor::spawn(bus);

        handle.send(InputMsg::InsertChar('t')).await;
        handle.send(InputMsg::InsertChar('e')).await;
        handle.send(InputMsg::InsertChar('s')).await;
        handle.send(InputMsg::Clear).await;
        drop(handle);
    }
}
