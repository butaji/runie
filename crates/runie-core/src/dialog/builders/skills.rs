//! Skills panel builder.
//!
//! Lists installed skills with actions: Create, Delete, Install, Show.
//! Opens the skills wizard or handles inline operations via events.

use super::{ItemAction, Panel, PanelStack};
use crate::Event;

/// Build the skills panel listing all installed skills.
pub fn skills(skill_rows: Vec<SkillRow>) -> PanelStack {
    let mut panel = Panel::new("skills", " Skills ")
        .header("Manage your skills")
        .keep_open();

    if skill_rows.is_empty() {
        panel = panel.item(
            "Create your first skill",
            ItemAction::Emit(Event::RunPaletteCommand {
                name: "skills".to_string(),
                args: "create".to_string(),
            }),
        );
    } else {
        panel = panel.header(&format!("{} skill(s) installed", skill_rows.len()));
        for skill in skill_rows {
            let invocable_icon = if skill.user_invocable { "★" } else { "○" };
            let label = format!("{invocable_icon} {}", skill.name);
            let desc = if skill.user_invocable {
                format!("{} — /{}", skill.description.chars().take(50).collect::<String>(), skill.name)
            } else {
                skill.description.chars().take(50).collect::<String>()
            };
            panel = panel.item(
                label,
                ItemAction::Emit(Event::SkillAction {
                    name: skill.name.clone(),
                    action: SkillActionKind::Select,
                }),
            );
        }
    }

    panel = panel.separator();
    panel = panel.item(
        "+ Install skill from URL",
        ItemAction::Emit(Event::RunPaletteCommand {
            name: "skills".to_string(),
            args: "install".to_string(),
        }),
    );
    panel = panel.item(
        "+ Create new skill",
        ItemAction::Emit(Event::RunPaletteCommand {
            name: "skills".to_string(),
            args: "create".to_string(),
        }),
    );

    PanelStack::new(panel)
}

/// A row in the skills panel.
#[derive(Debug, Clone)]
pub struct SkillRow {
    pub name: String,
    pub description: String,
    pub user_invocable: bool,
    pub file_path: String,
}

/// Actions that can be performed on a skill row.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SkillActionKind {
    Select,
    Show,
    Delete,
}
