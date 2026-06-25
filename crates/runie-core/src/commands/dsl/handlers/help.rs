//! Help commands.

use crate::commands::{CommandCategory, CommandRegistry, CommandResult};
use crate::dialog::{ItemAction, Panel, PanelStack};
use crate::model::AppState;

use super::spec::{CommandKind, CommandSpec};

fn quit(_: &mut AppState, _: &str) -> CommandResult {
    CommandResult::Event(crate::Event::Quit)
}

static CORE_COMMANDS: &[CommandSpec] = &[
    CommandSpec {
        name: "help",
        desc: "Show command reference",
        aliases: &["h", "?"],
        category: CommandCategory::Core,
        sub: true,
        kind: CommandKind::Handler(handle_help),
    },
    CommandSpec {
        name: "quit",
        desc: "Quit application",
        aliases: &["q", "exit"],
        category: CommandCategory::Core,
        sub: false,
        kind: CommandKind::Handler(quit),
    },
];

pub fn register(registry: &mut CommandRegistry) {
    super::spec::register_commands(registry, CORE_COMMANDS);
}

fn handle_help(state: &mut AppState, _: &str) -> CommandResult {
    CommandResult::OpenPanelStack(Box::new(PanelStack::new(build_help_panel(state))))
}

fn build_help_panel(state: &AppState) -> Panel {
    let mut panel = Panel::new("help", " Commands ")
        .with_filter()
        .header("Type to filter · Esc closes");

    let mut last_category: Option<CommandCategory> = None;
    let mut items: Vec<_> = state.registry.list();
    items.sort_by_key(|d| (d.category, &d.name));

    for cmd in items {
        if last_category != Some(cmd.category) {
            if last_category.is_some() {
                panel = panel.separator();
            }
            panel = panel.header(cmd.category.label());
            last_category = Some(cmd.category);
        }
        let aliases = if cmd.aliases.is_empty() {
            String::new()
        } else {
            format!(", {}", cmd.aliases.join(", "))
        };
        let label = format!("/{}{}  {}", cmd.name, aliases, cmd.desc);
        panel = panel.item(label, ItemAction::Close);
    }
    panel
}
