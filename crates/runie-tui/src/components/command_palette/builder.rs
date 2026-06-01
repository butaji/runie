use crate::tui::view_models::CommandPaletteViewModel;

pub(crate) struct CommandPaletteBuilder {
    show: bool,
}

impl CommandPaletteBuilder {
    pub(crate) fn new() -> Self {
        Self { show: false }
    }

    pub(crate) fn visible(mut self, visible: bool) -> Self {
        self.show = visible;
        self
    }

    pub(crate) fn build(self) -> CommandPaletteViewModel {
        CommandPaletteViewModel { show: self.show }
    }
}

impl Default for CommandPaletteBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_hidden_by_default() {
        let vm = CommandPaletteBuilder::new().build();
        assert!(!vm.show);
    }

    #[test]
    fn test_build_visible() {
        let vm = CommandPaletteBuilder::new().visible(true).build();
        assert!(vm.show);
    }
}
