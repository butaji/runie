use ratatui::{
    buffer::Buffer,
    layout::Rect,
};
use crate::theme::ThemeWrapper;
use crate::tui::state::Msg;

pub trait Modal {
    fn is_open(&self) -> bool;
    fn open(&mut self);
    fn close(&mut self);
    fn toggle(&mut self);
    fn render_ref(&self, area: Rect, buf: &mut Buffer, theme: &ThemeWrapper);
    fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> Option<Msg>;
}

impl Modal for crate::components::DiffViewer {
    fn is_open(&self) -> bool {
        self.visible
    }

    fn open(&mut self) {
        self.visible = true;
        self.scroll_offset = 0;
    }

    fn close(&mut self) {
        self.visible = false;
    }

    fn toggle(&mut self) {
        self.visible = !self.visible;
        if self.visible {
            self.scroll_offset = 0;
        }
    }

    fn render_ref(&self, area: Rect, buf: &mut Buffer, theme: &ThemeWrapper) {
        crate::components::DiffViewer::render_ref(self, area, buf, theme);
    }

    fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> Option<Msg> {
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.scroll_down();
                None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.scroll_up();
                None
            }
            KeyCode::Char('q') | KeyCode::Esc => {
                self.close();
                Some(Msg::CloseModal)
            }
            _ => None,
        }
    }
}
