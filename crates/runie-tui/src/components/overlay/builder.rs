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
        assert!(vm.visible);
    }

    #[test]
    fn test_build_with_visible_false() {
        let vm = OverlayBuilder::new().visible(false).build();
        assert!(!vm.visible);
    }
}
