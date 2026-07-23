//! Help commands.

use crate::commands::CommandResult;
use crate::dialog::{ItemAction, Panel, PanelStack};
use crate::model::AppState;

crate::handlers! {
    registry,
    "help" => handle_help,
    "quit" => quit,
}

pub fn handle_help(state: &mut AppState, _: &str) -> CommandResult {
    CommandResult::OpenPanelStack(Box::new(PanelStack::new(build_help_panel(state))))
}

pub fn quit(_: &mut AppState, _: &str) -> CommandResult {
    CommandResult::Event(crate::Event::Quit)
}

fn build_help_panel(state: &AppState) -> Panel {
    let mut panel = Panel::new("help", " Commands ")
        .with_filter()
        .header(crate::ui_strings::help::FILTER_HEADER);

    let mut last_category: Option<crate::commands::CommandCategory> = None;
    let mut items: Vec<_> = state.registry().list();
    items.sort_by_key(|d| (d.category, &d.name));

    for spec in items {
        if last_category != Some(spec.category) {
            if last_category.is_some() {
                panel = panel.separator();
            }
            panel = panel.header(spec.category.label());
            last_category = Some(spec.category);
        }
        let aliases = if spec.aliases.is_empty() {
            String::new()
        } else {
            format!(", {}", spec.aliases.join(", "))
        };
        let label = format!("/{}{}  {}", spec.name, aliases, spec.desc);
        panel = panel.item(label, ItemAction::Close);
    }
    panel
}
