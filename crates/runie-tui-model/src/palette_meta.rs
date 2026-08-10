use super::PaletteAction;

macro_rules! palette_slash_command {
    ($value:expr) => {
        match $value {
            Self::NewSession => "/new",
            Self::KeyboardShortcuts => "/hotkeys",
            Self::Quit => "/quit",
            Self::Changelog => "/changelog",
            Self::CopyLastResponse => "/copy",
            Self::SessionInfo => "/session",
            Self::SelectModel => "/model",
            Self::SelectTheme => "/theme",
            Self::ManageProviders => "/providers",
            Self::ScopedModels => "/scoped-models",
            Self::SetSessionName => "/name",
            Self::CompactContext => "/compact",
            Self::ForkSession => "/fork",
            Self::SelectTreeEntry => "/tree",
            Self::ExportSession => "/export",
            Self::ImportSession => "/import",
            Self::CloneSession => "/clone",
            Self::ResumeSession => "/resume",
            Self::ShareSession => "/share",
            Self::Help => "/help",
            Self::ContextInfo => "/context",
            Self::Settings => "/settings",
            Self::Doctor => "/doctor",
            Self::RewindSession => "/rewind",
            Self::PromptHistory => "/history",
            Self::FindTranscript => "/find",
            Self::JumpTranscript => "/jump",
            Self::Recap => "/recap",
            Self::SetEffort => "/effort",
            Self::AlwaysApprove => "/always-approve",
            Self::AutoApprove => "/auto",
            Self::PlanMode => "/plan",
            Self::ViewPlan => "/view-plan",
            Self::Login => "/login",
            Self::Logout => "/logout",
            Self::Reload => "/reload",
            Self::TrustProject => "/trust",
            Self::Skills => "/skills",
            Self::Hooks => "/hooks",
            Self::Plugins => "/plugins",
            Self::Mcps => "/mcps",
            Self::Memory => "/memory",
            Self::Remember => "/remember",
            Self::Goal => "/goal",
            Self::Workflow => "/workflow",
            Self::Workflows => "/workflows",
            Self::Loop => "/loop",
            Self::DeepResearch => "/deep-research",
            Self::Feedback => "/feedback",
            Self::Usage => "/usage",
            Self::Jobs => "/jobs",
            Self::Questions => "/questions",
        }
    };
}

macro_rules! palette_description {
    ($value:expr) => {
        match $value {
            Self::NewSession => "Start a fresh session",
            Self::KeyboardShortcuts => "Show keyboard shortcuts",
            Self::Quit => "Exit Runie",
            Self::Changelog => "Show recent changes",
            Self::CopyLastResponse => "Copy the latest response",
            Self::SessionInfo => "Show session statistics",
            Self::SelectModel => "Switch the active model",
            Self::SelectTheme => "Change the interface theme",
            Self::ManageProviders => "Manage configured providers",
            Self::ScopedModels => "Browse scoped models",
            Self::SetSessionName => "Rename the current session",
            Self::CompactContext => "Compress conversation history",
            Self::ForkSession => "Fork at an entry",
            Self::SelectTreeEntry => "Select a tree entry",
            Self::ExportSession => "Export a JSONL session",
            Self::ImportSession => "Import a JSONL session",
            Self::CloneSession => "Clone a JSONL session",
            Self::ResumeSession => "Resume a JSONL session",
            Self::ShareSession => "Create a shareable session link",
            Self::Help => "Show commands and usage",
            Self::ContextInfo => "Show context-window usage",
            Self::Settings => "Open persistent settings",
            Self::Doctor => "Diagnose runtime integrations",
            Self::RewindSession => "Restore an earlier conversation state",
            Self::PromptHistory => "Search previous prompts",
            Self::FindTranscript => "Search the transcript",
            Self::JumpTranscript => "Jump to a transcript entry",
            Self::Recap => "Summarize the current task and state",
            Self::SetEffort => "Set reasoning effort",
            Self::AlwaysApprove => "Toggle approval-free execution",
            Self::AutoApprove => "Automatically approve safe tools",
            Self::PlanMode => "Enter plan mode",
            Self::ViewPlan => "Show the current plan",
            Self::Login => "Authenticate a provider",
            Self::Logout => "Remove provider credentials",
            Self::Reload => "Reload configuration and resources",
            Self::TrustProject => "Trust or revoke a project",
            Self::Skills => "List or invoke skills",
            Self::Hooks => "Manage project hooks",
            Self::Plugins => "Manage plugins",
            Self::Mcps => "Manage MCP servers",
            Self::Memory => "Browse or configure memory",
            Self::Remember => "Save a note immediately",
            Self::Goal => "Create or manage an autonomous goal",
            Self::Workflow => "Launch or manage a workflow",
            Self::Workflows => "Show active workflow runs",
            Self::Loop => "Schedule recurring work",
            Self::DeepResearch => "Run bounded research",
            Self::Feedback => "Submit feedback",
            Self::Usage => "Show usage or billing information",
            Self::Jobs => "Inspect owned background jobs",
            Self::Questions => "Browse user-question history",
        }
    };
}

pub fn palette_display_rows(query: &str, skills: &[String]) -> Vec<String> {
    let labels = super::palette_labels(query, skills);
    let grouped = query.trim().is_empty();
    let mut section = None;
    labels
        .into_iter()
        .flat_map(|label| {
            let next = PaletteAction::from_label(&label).map(palette_section);
            let header = grouped
                .then_some(next)
                .flatten()
                .filter(|next| section.as_ref() != Some(next))
                .map(|next| {
                    section = Some(next);
                    format!("§{next}")
                });
            header
                .into_iter()
                .chain(std::iter::once(palette_row(&label)))
        })
        .collect()
}

fn palette_section(action: PaletteAction) -> &'static str {
    if is_context_action(&action) {
        "Context"
    } else if is_session_action(&action) {
        "Session"
    } else if is_information_action(&action) {
        "Information"
    } else if is_extension_action(&action) {
        "Extensions"
    } else if is_automation_action(&action) {
        "Automation"
    } else {
        "Model & Input"
    }
}

const fn is_context_action(action: &PaletteAction) -> bool {
    matches!(
        action,
        PaletteAction::CopyLastResponse | PaletteAction::SessionInfo
    )
}

const fn is_session_action(action: &PaletteAction) -> bool {
    matches!(
        action,
        PaletteAction::NewSession
            | PaletteAction::KeyboardShortcuts
            | PaletteAction::Quit
            | PaletteAction::Changelog
            | PaletteAction::ShareSession
    )
}

const fn is_information_action(action: &PaletteAction) -> bool {
    matches!(
        action,
        PaletteAction::Help
            | PaletteAction::ContextInfo
            | PaletteAction::Doctor
            | PaletteAction::Feedback
            | PaletteAction::Usage
    )
}

const fn is_extension_action(action: &PaletteAction) -> bool {
    matches!(
        action,
        PaletteAction::Skills
            | PaletteAction::Hooks
            | PaletteAction::Plugins
            | PaletteAction::Mcps
            | PaletteAction::Memory
    )
}

const fn is_automation_action(action: &PaletteAction) -> bool {
    matches!(
        action,
        PaletteAction::Goal
            | PaletteAction::Workflow
            | PaletteAction::Workflows
            | PaletteAction::Loop
            | PaletteAction::DeepResearch
    )
}

fn palette_row(label: &str) -> String {
    PaletteAction::from_label(label).map_or_else(
        || format!("{label}  · skill"),
        |action| {
            let argument = if action.requires_parameters() {
                format!(" <{}>", action.parameter_hint())
            } else {
                String::new()
            };
            format!(
                "{label}{argument}  · {} · {}",
                action.description(),
                action.source()
            )
        },
    )
}

impl PaletteAction {
    pub const fn requires_parameters(&self) -> bool {
        matches!(
            self,
            Self::SetSessionName
                | Self::SelectTheme
                | Self::CompactContext
                | Self::ForkSession
                | Self::SelectTreeEntry
                | Self::ExportSession
                | Self::ImportSession
                | Self::CloneSession
                | Self::ResumeSession
                | Self::Help
                | Self::Settings
                | Self::Doctor
                | Self::RewindSession
                | Self::PromptHistory
                | Self::FindTranscript
                | Self::JumpTranscript
                | Self::SetEffort
                | Self::AlwaysApprove
                | Self::AutoApprove
                | Self::PlanMode
                | Self::Login
                | Self::Logout
                | Self::TrustProject
                | Self::Remember
                | Self::Goal
                | Self::Workflow
                | Self::Loop
                | Self::DeepResearch
                | Self::Feedback
                | Self::Usage
                | Self::Jobs
                | Self::Questions
        )
    }

    pub const fn slash_command(&self) -> &'static str {
        palette_slash_command!(self)
    }

    pub const fn parameter_hint(&self) -> &'static str {
        match self {
            Self::SetSessionName => "Session name",
            Self::CompactContext => "Instructions (optional)",
            Self::ForkSession | Self::SelectTreeEntry => "Entry id",
            Self::SelectModel => "provider/model",
            Self::SelectTheme => "theme name",
            Self::ManageProviders => "connect|provider|base URL|API key|model",
            Self::ExportSession
            | Self::ImportSession
            | Self::CloneSession
            | Self::ResumeSession => "Path (.jsonl)",
            Self::Help => "Search (optional)",
            Self::Settings => "Key/value (optional)",
            Self::Doctor => "fix (optional)",
            Self::RewindSession | Self::JumpTranscript => "Entry id or query",
            Self::PromptHistory | Self::FindTranscript => "Query (optional)",
            Self::SetEffort => "effort declared by current model",
            Self::AlwaysApprove | Self::AutoApprove => "on|off",
            Self::PlanMode => "Description (optional)",
            Self::Login | Self::Logout => "Provider (optional)",
            Self::TrustProject => "Project path (optional)",
            Self::Remember | Self::Feedback => "Text",
            Self::Goal => "Objective or status|pause|resume|clear",
            Self::Workflow => "Name and arguments",
            Self::Loop => "Interval and prompt",
            Self::DeepResearch => "Query",
            Self::Usage => "manage (optional)",
            Self::Questions => "Query (optional)",
            _ => "Value",
        }
    }

    pub const fn description(&self) -> &'static str {
        palette_description!(self)
    }

    pub const fn source(&self) -> &'static str {
        "builtin"
    }
}

#[allow(unused_macros)]

const THEME_LABELS: &[&str] = &[
    "ayu-dark",
    "ayu-light",
    "ayu-mirage",
    "catppuccin-frappe",
    "catppuccin-latte",
    "catppuccin-macchiato",
    "catppuccin-mocha",
    "dracula",
    "everforest-dark",
    "everforest-light",
    "flexoki-dark",
    "flexoki-light",
    "github-dark-dimmed",
    "github-light",
    "gruvbox-dark",
    "gruvbox-light",
    "kanagawa-dragon",
    "kanagawa-lotus",
    "kanagawa-wave",
    "light-owl",
    "monokai-pro",
    "nord",
    "one-dark",
    "one-light",
    "palenight",
    "rose-pine",
    "rose-pine-dawn",
    "rose-pine-moon",
    "silkcircuit-dawn",
    "silkcircuit-glow",
    "silkcircuit-neon",
    "silkcircuit-soft",
    "silkcircuit-vibrant",
    "solarized-dark",
    "solarized-light",
    "tokyo-night",
    "tokyo-night-moon",
    "tokyo-night-storm",
    "night-owl",
];

pub fn theme_labels() -> Vec<&'static str> {
    THEME_LABELS.to_vec()
}
