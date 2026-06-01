use crate::tui::view_models::PermissionModalViewModel;

pub(crate) struct PermissionBuilder {
    tool: String,
    args: String,
    desc: String,
    selected: usize,
    timeout_secs: Option<u64>,
}

impl PermissionBuilder {
    pub(crate) fn new() -> Self {
        Self {
            tool: String::new(),
            args: String::new(),
            desc: String::new(),
            selected: 0,
            timeout_secs: None,
        }
    }

    pub(crate) fn tool(mut self, name: &str, args: &str) -> Self {
        self.tool = name.to_string();
        self.args = args.to_string();
        self
    }

    pub(crate) fn description(mut self, desc: &str) -> Self {
        self.desc = desc.to_string();
        self
    }

    pub(crate) fn allow_all(mut self) -> Self {
        self.selected = 2;
        self
    }

    pub(crate) fn timeout_secs(mut self, secs: u64) -> Self {
        self.timeout_secs = Some(secs);
        self
    }

    pub(crate) fn build(self) -> PermissionModalViewModel {
        PermissionModalViewModel {
            tool: self.tool,
            args: self.args,
            desc: self.desc,
            selected: self.selected,
            visible: true,
            timeout_secs: self.timeout_secs,
        }
    }
}

impl Default for PermissionBuilder {
    fn default() -> Self {
        Self::new()
    }
}
