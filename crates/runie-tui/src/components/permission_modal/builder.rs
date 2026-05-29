use super::PermissionModal;

pub struct PermissionBuilder {
    tool_name: String,
    tool_args: String,
    description: String,
    allow_all: bool,
}

impl PermissionBuilder {
    pub fn new() -> Self {
        Self {
            tool_name: String::new(),
            tool_args: String::new(),
            description: String::new(),
            allow_all: false,
        }
    }

    pub fn tool(mut self, name: &str, args: &str) -> Self {
        self.tool_name = name.to_string();
        self.tool_args = args.to_string();
        self
    }

    pub fn description(mut self, desc: &str) -> Self {
        self.description = desc.to_string();
        self
    }

    pub fn allow_all(mut self) -> Self {
        self.allow_all = true;
        self
    }

    pub fn build(self) -> PermissionModal {
        PermissionModal {
            title: "Permission Required".to_string(),
            tool_name: self.tool_name,
            tool_args: self.tool_args,
            description: self.description,
            selected: if self.allow_all { 2 } else { 0 },
            timeout_secs: None,
        }
    }
}

impl Default for PermissionBuilder {
    fn default() -> Self {
        Self::new()
    }
}
