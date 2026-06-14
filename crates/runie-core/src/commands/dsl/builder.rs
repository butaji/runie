//! Command Builder

use super::{CommandCategory, CommandFlow, CommandResult, DialogType};
use crate::dialog::dsl::{form, FormPanel};
use crate::dialog::PanelStack as CoreStack;
use crate::model::AppState;

/// A single command definition
#[derive(Clone)]
pub struct CommandDef {
    pub name: String,
    pub desc: String,
    pub aliases: Vec<String>,
    pub category: CommandCategory,
    pub flow: CommandFlow,
    /// If true, this command opens a sub-dialog: the current dialog
    /// (e.g. the command palette = Main Menu) is pushed onto the
    /// global back stack before the command runs. Android-like.
    pub is_sub: bool,
}

impl CommandDef {
    /// Create a new command
    pub fn new(name: &'static str) -> Self {
        Self {
            name: name.into(),
            desc: String::new(),
            aliases: Vec::new(),
            category: CommandCategory::System,
            flow: CommandFlow::None,
            is_sub: false,
        }
    }

    /// Set description
    pub fn desc(mut self, desc: &'static str) -> Self {
        self.desc = desc.into();
        self
    }

    /// Add an alias
    pub fn alias(mut self, alias: &'static str) -> Self {
        self.aliases.push(alias.into());
        self
    }

    /// Add multiple aliases
    pub fn aliases(mut self, aliases: &'static [&'static str]) -> Self {
        self.aliases.extend(aliases.iter().map(|s| s.to_string()));
        self
    }

    /// Set category
    pub fn category(mut self, cat: CommandCategory) -> Self {
        self.category = cat;
        self
    }

    /// Show a static message
    pub fn msg(self, msg: &'static str) -> Self {
        self.with_flow(CommandFlow::Message(msg))
    }

    /// Show a dynamic message
    pub fn msgf(self, f: fn(&AppState, &str) -> String) -> Self {
        self.with_flow(CommandFlow::Dynamic(f))
    }

    /// Show message or fallback if result is None
    pub fn or_msg(self, f: fn(&AppState, &str) -> CommandResult, fallback: &'static str) -> Self {
        self.with_flow(CommandFlow::OrMessage(f, fallback))
    }

    /// Open a dialog
    pub fn dialog(self, d: DialogType) -> Self {
        self.with_flow(CommandFlow::Dialog(d)).apply_sub()
    }

    /// Open a panel stack produced at runtime
    pub fn panel<F>(self, f: F) -> Self
    where
        F: Fn(&mut AppState, &str) -> CoreStack + Send + Sync + 'static,
    {
        self.with_flow(CommandFlow::PanelStack(std::sync::Arc::new(f)))
            .apply_sub()
    }

    /// Mark this command as opening a sub-dialog. The current dialog
    /// (typically the command palette = Main Menu) is automatically
    /// pushed onto the global back stack before the command runs.
    /// Esc returns to the previous dialog; only at the absolute root
    /// does Esc close the bar. Android-like navigation for every
    /// menu bar item.
    ///
    /// # Example
    /// ```ignore
    /// crate::cmd!("settings")
    ///     .desc("Open settings")
    ///     .sub()
    ///     .dialog(DialogType::Settings)
    ///
    /// crate::cmd!("login")
    ///     .desc("Login to a provider")
    ///     .sub()
    ///     .panel(|state, _| build_login_root(state))
    /// ```
    pub fn sub(mut self) -> Self {
        self.is_sub = true;
        self
    }

    /// Apply the sub-dialog wrapping if `.sub()` was called. Wraps
    /// the flow in `CommandFlow::Sub` so the executor pushes the
    /// current dialog onto the back stack before running.
    fn apply_sub(mut self) -> Self {
        if self.is_sub && !matches!(self.flow, CommandFlow::None) {
            let inner = std::mem::replace(&mut self.flow, CommandFlow::None);
            self.flow = CommandFlow::Sub(Box::new(inner));
        }
        self
    }

    /// Show a form dialog. Args are split on whitespace and used to
    /// pre-fill fields in order. The closure must call `.on_submit`.
    pub fn form<F>(self, title: &'static str, build: F) -> Self
    where
        F: FnOnce(FormPanel) -> FormPanel + Send + Sync + 'static,
    {
        let id = self.name.clone();
        let template = build(form(id, title));
        self.panel(move |_state, args| build_form_stack_from_template(template.clone(), args))
    }

    /// Custom handler
    pub fn handler(self, f: fn(&mut AppState, &str) -> CommandResult) -> Self {
        self.with_flow(CommandFlow::Handler(f)).apply_sub()
    }

    /// Chain multiple flows
    pub fn chain(self, flows: Vec<CommandFlow>) -> Self {
        self.with_flow(CommandFlow::Chain(flows))
    }

    /// Conditional flow
    pub fn when(self, predicate: fn(&AppState) -> bool, flow: CommandFlow) -> Self {
        self.with_flow(CommandFlow::When(predicate, Box::new(flow)))
    }

    fn with_flow(mut self, flow: CommandFlow) -> Self {
        self.flow = flow;
        self
    }
}

/// Shorthand constructor
pub fn cmd(name: &'static str) -> CommandDef {
    CommandDef::new(name)
}

fn build_form_stack_from_template(template: FormPanel, args: &str) -> CoreStack {
    let args_list: Vec<&str> = args.split_whitespace().collect();
    let built = template.build();
    let mut panel = crate::dialog::Panel::new(built.id, built.title).form();
    panel.submit_factory = built.submit_factory;
    let mut arg_idx = 0;
    for item in built.items {
        match item {
            crate::dialog::PanelItem::FormField {
                label,
                placeholder,
                key,
                value,
            } => {
                let val = if arg_idx < args_list.len() {
                    args_list[arg_idx].to_string()
                } else {
                    value
                };
                panel = panel.form_field_value(label, placeholder, key, val);
                arg_idx += 1;
            }
            crate::dialog::PanelItem::FormSubmit => panel = panel.form_submit(),
            _ => {}
        }
    }
    CoreStack::new(panel)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Event;

    #[test]
    fn test_cmd_macro() {
        let cmd = crate::cmd!("hello", "Hello!");
        assert_eq!(cmd.name, "hello");
        assert!(matches!(cmd.flow, CommandFlow::Message(_)));
    }

    #[test]
    fn test_cmd_builder_chain() {
        let cmd = crate::cmd!("test")
            .desc("Test command")
            .alias("t")
            .aliases(&["tt", "ttt"])
            .category(CommandCategory::System)
            .msg("Test message");

        assert_eq!(cmd.name, "test");
        assert_eq!(cmd.desc, "Test command");
        assert_eq!(cmd.aliases, vec!["t", "tt", "ttt"]);
        assert_eq!(cmd.category, CommandCategory::System);
    }

    #[test]
    fn test_sub_wraps_flow() {
        use super::super::{CommandFlow, DialogType};
        let cmd = crate::cmd!("settings")
            .desc("Open settings")
            .category(CommandCategory::System)
            .sub()
            .dialog(DialogType::Settings);
        assert!(matches!(cmd.flow, CommandFlow::Sub(_)));
    }

    #[test]
    fn test_sub_is_noop_for_empty_flow() {
        let cmd = crate::cmd!("nothing").sub();
        assert!(matches!(cmd.flow, CommandFlow::None));
    }

    #[test]
    fn test_sub_wraps_handler() {
        fn my_handler(_: &mut crate::model::AppState, _: &str) -> super::super::CommandResult {
            super::super::CommandResult::None
        }
        let cmd = crate::cmd!("custom")
            .desc("Custom sub command")
            .sub()
            .handler(my_handler);
        assert!(matches!(cmd.flow, super::super::CommandFlow::Sub(_)));
    }

    fn save_submit(values: &std::collections::HashMap<String, String>) -> Event {
        Event::RunSaveCommand {
            name: values.get("name").cloned().unwrap_or_default(),
        }
    }

    #[test]
    fn command_form_builds_panel_stack() {
        let cmd = crate::cmd!("save").desc("Save session").form("save", |f| {
            f.field("Name", "session", "name").on_submit(save_submit)
        });
        assert!(matches!(cmd.flow, CommandFlow::PanelStack(_)));
    }

    #[test]
    fn form_prefills_args() {
        let cmd = crate::cmd!("save").form("save", |f| {
            f.field("Name", "session", "name").on_submit(save_submit)
        });
        let mut state = AppState::default();
        let result = cmd.flow.exec(&mut state, "save", "mysession");
        if let CommandResult::OpenPanelStack(stack) = result {
            let panel = stack.current().unwrap();
            assert_eq!(
                panel.form_values.get("name"),
                Some(&"mysession".to_string())
            );
        } else {
            panic!("expected panel stack");
        }
    }
}
