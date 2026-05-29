use crate::tui::view_models::PermissionModalViewModel;

pub struct PermissionBuilder {
    tool: String,
    args: String,
    desc: String,
    selected: usize,
    timeout_secs: Option<u64>,
}

impl PermissionBuilder {
    pub fn new() -> Self {
        Self {
            tool: String::new(),
            args: String::new(),
            desc: String::new(),
            selected: 0,
            timeout_secs: None,
        }
    }

    pub fn tool(mut self, name: &str, args: &str) -> Self {
        self.tool = name.to_string();
        self.args = args.to_string();
        self
    }

    pub fn description(mut self, desc: &str) -> Self {
        self.desc = desc.to_string();
        self
    }

    pub fn allow_all(mut self) -> Self {
        self.selected = 2;
        self
    }

    pub fn timeout_secs(mut self, secs: u64) -> Self {
        self.timeout_secs = Some(secs);
        self
    }

    pub fn build(self) -> PermissionModalViewModel {
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
