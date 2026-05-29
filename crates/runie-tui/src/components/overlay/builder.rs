use crate::tui::view_models::OverlayViewModel;

pub struct OverlayBuilder {
    title: String,
    content: Vec<String>,
    tabs: Vec<String>,
    active_tab: usize,
    show_close: bool,
}

impl OverlayBuilder {
    pub fn new() -> Self {
        Self {
            title: String::new(),
            content: Vec::new(),
            tabs: Vec::new(),
            active_tab: 0,
            show_close: true,
        }
    }

    pub fn title(mut self, title: &str) -> Self {
        self.title = title.to_string();
        self
    }

    pub fn content(mut self, content: &[&str]) -> Self {
        self.content = content.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn tab(mut self, tab: &str) -> Self {
        self.tabs.push(tab.to_string());
        self
    }

    pub fn active_tab(mut self, index: usize) -> Self {
        self.active_tab = index;
        self
    }

    pub fn show_close(mut self, show: bool) -> Self {
        self.show_close = show;
        self
    }

    pub fn build(self) -> OverlayViewModel {
        OverlayViewModel {
            title: self.title,
            content: self.content,
            tabs: self.tabs,
            active_tab: self.active_tab,
            show_close: self.show_close,
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
