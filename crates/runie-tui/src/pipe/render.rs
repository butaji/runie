use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io;
use crate::tui::view_models::ViewModels;
use super::Pipe;

/// RenderPipe transforms ViewModels into terminal frames.
/// Phase 3 placeholder - actual rendering happens in Tui::render()
#[allow(dead_code)]
pub struct RenderPipe<'a> {
    terminal: &'a mut Terminal<CrosstermBackend<io::Stdout>>,
}

impl<'a> RenderPipe<'a> {
    pub fn new(terminal: &'a mut Terminal<CrosstermBackend<io::Stdout>>) -> Self {
        Self { terminal }
    }
}

impl<'a> Pipe<ViewModels> for RenderPipe<'a> {
    type Output = io::Result<()>;

    fn pipe(&self, _vms: ViewModels) -> io::Result<()> {
        // TODO: integrate with existing render in Phase 3
        Ok(())
    }
}