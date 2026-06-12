//! Command Builder

use super::{CommandCategory, CommandFlow, CommandResult, DialogType};
use crate::dialog::dsl::Panel as DslPanel;
use crate::dialog::{PanelItem, PanelStack as CoreStack};
use crate::model::AppState;
use crate::Event;

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
            category: CommandCategory::Help,
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

    /// Open a panel stack
    pub fn panel(self, f: fn(&mut AppState, &str) -> CoreStack) -> Self {
        self.with_flow(CommandFlow::PanelStack(f)).apply_sub()
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

    /// Show a form dialog
    pub fn form<F>(self, title: &'static str, build: F, submit: Event) -> Self
    where
        F: FnOnce(FormBuilder) -> FormBuilder,
    {
        let fields = build(FormBuilder::new()).into_fields();
        self.with_flow(CommandFlow::Form {
            title,
            fields,
            submit,
        })
        .apply_sub()
    }

    /// Show a form from a DSL panel
    pub fn dsl_form<F>(self, title: &'static str, panel_fn: F, submit: Event) -> Self
    where
        F: FnOnce() -> DslPanel,
    {
        let panel = panel_fn();
        let fields = FormBuilder::from_dsl_panel(&panel).into_fields();
        self.with_flow(CommandFlow::Form {
            title,
            fields,
            submit,
        })
        .apply_sub()
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

/// A form field definition
#[derive(Clone)]
pub struct FormField {
    pub label: String,
    pub placeholder: String,
    pub key: String,
    pub prefill: Option<String>,
}

impl FormField {
    pub fn new(
        label: impl Into<String>,
        placeholder: impl Into<String>,
        key: impl Into<String>,
    ) -> Self {
        Self {
            label: label.into(),
            placeholder: placeholder.into(),
            key: key.into(),
            prefill: None,
        }
    }

    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        self.prefill = Some(value.into());
        self
    }
}

/// Form builder for declarative field creation
#[derive(Default)]
pub struct FormBuilder {
    fields: Vec<FormField>,
}

impl FormBuilder {
    pub fn new() -> Self {
        Self { fields: Vec::new() }
    }

    /// Add a text field
    pub fn field(
        self,
        label: impl Into<String>,
        placeholder: impl Into<String>,
        key: impl Into<String>,
    ) -> Self {
        self.field_value(label, placeholder, key, "")
    }

    /// Add a field with pre-filled value
    pub fn field_value(
        self,
        label: impl Into<String>,
        placeholder: impl Into<String>,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        let mut this = self;
        this.fields
            .push(FormField::new(label, placeholder, key).with_value(value));
        this
    }

    /// Add multiple fields at once
    pub fn fields(self, fields: &[(String, String, String)]) -> Self {
        let mut this = self;
        for (label, placeholder, key) in fields {
            this = this.field(label, placeholder, key);
        }
        this
    }

    /// Add fields from DSL panel (converts to owned strings)
    pub fn from_dsl_panel(panel: &DslPanel) -> Self {
        let mut builder = Self::new();
        for item in &panel.items {
            if let PanelItem::FormField {
                label,
                placeholder,
                key,
                ..
            } = item
            {
                builder = builder.field(label, placeholder, key);
            }
        }
        builder
    }

    pub fn into_fields(self) -> Vec<FormField> {
        self.fields
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        // .sub() wraps the flow in CommandFlow::Sub so the current
        // dialog (e.g. the palette = Main Menu) is pushed onto the
        // back stack before the command runs.
        let cmd = crate::cmd!("settings")
            .desc("Open settings")
            .category(CommandCategory::System)
            .sub()
            .dialog(DialogType::Settings);
        assert!(matches!(cmd.flow, CommandFlow::Sub(_)));
    }

    fn test_sub_is_noop_for_empty_flow() {
        // .sub() on a command with no flow should be a no-op.
        let cmd = crate::cmd!("nothing").sub();
        assert!(matches!(cmd.flow, CommandFlow::None));
    }

    fn test_form_builder() {
        let fields = FormBuilder::new()
            .field("Name", "session", "name")
            .field("Path", "file.json", "path")
            .field_value("Default", "placeholder", "key", "value")
            .into_fields();

        assert_eq!(fields.len(), 3);
        assert_eq!(fields[2].prefill, Some("value".into()));
    }
}
