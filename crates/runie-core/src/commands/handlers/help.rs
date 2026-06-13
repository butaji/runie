//! Help commands using the new DSL

use crate::commands::{CommandCategory, CommandRegistry, CommandResult};
use crate::dialog::{ItemAction, Panel, PanelStack};
use crate::model::AppState;

pub fn register(registry: &mut CommandRegistry) {
    registry.register(
        crate::cmd!("help")
            .desc("Show command reference")
            .aliases(&["h", "?"])
            .category(CommandCategory::Core)
            .sub()
            .handler(handle_help),
    );

    registry.register(
        crate::cmd!("quit")
            .desc("Quit application")
            .aliases(&["q", "exit"])
            .category(CommandCategory::Core)
            .handler(|_, _| CommandResult::Event(crate::Event::Quit)),
    );
}

fn handle_help(state: &mut AppState, _: &str) -> CommandResult {
    CommandResult::OpenPanelStack(PanelStack::new(build_help_panel(state)))
}

fn build_help_panel(state: &AppState) -> Panel {
    let mut panel = Panel::new("help", " Commands ")
        .with_filter()
        .header("Type to filter · Esc closes");

    let mut last_category: Option<CommandCategory> = None;
    let mut items: Vec<_> = state.registry.list();
    // The registry already sorts by category then name; this keeps categories stable.
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
