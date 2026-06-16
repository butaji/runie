//! Slash-command execution event variants.

use std::fmt;
use strum::IntoStaticStr;

/// Events emitted when running slash commands.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, IntoStaticStr)]
#[strum(serialize_all = "PascalCase")]
pub enum CommandEvent {
    RunSaveCommand { name: String },
    RunLoadCommand { name: String },
    RunDeleteCommand { name: String },
    RunImportCommand { path: String },
    RunExportCommand { path: String },
    RunSkillCommand { name: String },
    RunLoginCommand { provider: String, token: String },
    RunLogoutCommand { provider: String },
    RunNameCommand { name: String },
    RunForkCommand { message_index: String },
    RunCompactCommand { keep: String, focus: String },
    RunPromptCommand { name: String },
    RunThinkingCommand { level: crate::model::ThinkingLevel },
    RunPaletteCommand { name: String, args: String },
}

impl fmt::Display for CommandEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandEvent::RunSaveCommand { .. } => write!(f, "RunSaveCommand"),
            CommandEvent::RunLoadCommand { .. } => write!(f, "RunLoadCommand"),
            CommandEvent::RunDeleteCommand { .. } => write!(f, "RunDeleteCommand"),
            CommandEvent::RunImportCommand { .. } => write!(f, "RunImportCommand"),
            CommandEvent::RunExportCommand { .. } => write!(f, "RunExportCommand"),
            CommandEvent::RunSkillCommand { .. } => write!(f, "RunSkillCommand"),
            CommandEvent::RunLoginCommand { .. } => write!(f, "RunLoginCommand"),
            CommandEvent::RunLogoutCommand { .. } => write!(f, "RunLogoutCommand"),
            CommandEvent::RunNameCommand { .. } => write!(f, "RunNameCommand"),
            CommandEvent::RunForkCommand { .. } => write!(f, "RunForkCommand"),
            CommandEvent::RunCompactCommand { .. } => write!(f, "RunCompactCommand"),
            CommandEvent::RunPromptCommand { .. } => write!(f, "RunPromptCommand"),
            CommandEvent::RunThinkingCommand { .. } => write!(f, "RunThinkingCommand"),
            CommandEvent::RunPaletteCommand { .. } => write!(f, "RunPaletteCommand"),
        }
    }
}
