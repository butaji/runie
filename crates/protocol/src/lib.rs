use ratatui::Frame;
use crossterm::event::KeyEvent;

pub trait App {
    fn update(&mut self);
    fn render(&mut self, frame: &mut Frame);
    fn handle_key(&mut self, key: KeyEvent);
}
