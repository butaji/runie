use crate::tui::view_models::OverlayViewModel;

pub(crate) struct OverlayBuilder {
    visible: bool,
}

impl OverlayBuilder {
    pub(crate) fn new() -> Self {
        Self {
            visible: true,
        }
    }

    pub(crate) fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    pub(crate) fn build(self) -> OverlayViewModel {
        OverlayViewModel {
            visible: self.visible,
        }
    }
}

impl Default for OverlayBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_default_overlay() {
        let vm = OverlayBuilder::new().build();
        assert_eq!(vm.title, "");
        assert!(vm.content.is_empty());
        assert!(vm.tabs.is_empty());
        assert_eq!(vm.active_tab, 0);
        assert!(vm.show_close);
    }

    #[test]
    fn test_build_with_title() {
        let vm = OverlayBuilder::new().title("Test Title").build();
        assert_eq!(vm.title, "Test Title");
    }

    #[test]
    fn test_build_with_content() {
        let vm = OverlayBuilder::new()
            .content(&["Line 1", "Line 2"])
            .build();
        assert_eq!(vm.content.len(), 2);
        assert_eq!(vm.content[0], "Line 1");
    }

    #[test]
    fn test_build_with_tabs() {
        let vm = OverlayBuilder::new()
            .tab("Tab 1")
            .tab("Tab 2")
            .active_tab(1)
            .build();
        assert_eq!(vm.tabs.len(), 2);
        assert_eq!(vm.active_tab, 1);
    }
}
