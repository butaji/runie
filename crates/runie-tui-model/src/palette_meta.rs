use super::PaletteAction;

macro_rules! declare_palette_metadata {
    ($(($variant:ident, $slash:literal, $description:literal),)+) => {
        macro_rules! palette_slash_command {
            ($value:expr) => {
                match $value {
                    $(Self::$variant => $slash,)+
                }
            };
        }

        macro_rules! palette_description {
            ($value:expr) => {
                match $value {
                    $(Self::$variant => $description,)+
                }
            };
        }

        #[allow(dead_code)]
        pub const fn palette_metadata_count() -> usize {
            0 $(+ { let _ = stringify!($variant); 1 })+
        }
    };
}

declare_palette_metadata! {
    (NewSession, "/new", "Start a fresh session"),
    (KeyboardShortcuts, "/hotkeys", "Show keyboard shortcuts"),
    (Quit, "/quit", "Exit Runie"),
    (Changelog, "/changelog", "Show recent changes"),
    (CopyLastResponse, "/copy", "Copy the latest response"),
    (SessionInfo, "/session", "Show session statistics"),
    (SessionHistory, "/sessions history", "Show session branch history"),
    (SessionHistoryQuery, "/sessions history query", "Search session branch history"),
    (UndoSession, "/undo", "Undo the latest session entry"),
    (SelectModel, "/model", "Switch the active model"),
    (SelectTheme, "/theme", "Change the interface theme"),
    (ManageProviders, "/providers", "Manage configured providers"),
    (ScopedModels, "/scoped-models", "Browse scoped models"),
    (SetSessionName, "/name", "Rename the current session"),
    (CompactContext, "/compact", "Compress conversation history"),
    (CompactContextNow, "/compact", "Compact immediately without instructions"),
    (ForkSession, "/fork", "Fork at an entry"),
    (SelectTreeEntry, "/tree", "Select a tree entry"),
    (ExportSession, "/export", "Export a JSONL session"),
    (ImportSession, "/import", "Import a JSONL session"),
    (CloneSession, "/clone", "Clone a JSONL session"),
    (ResumeSession, "/resume", "Resume a JSONL session"),
    (ResumeSessionPicker, "/resume", "Open the session picker"),
    (ShareSession, "/share", "Create a shareable session link"),
    (Help, "/help", "Show commands and usage"),
    (ContextInfo, "/context", "Show context-window usage"),
    (ClearContext, "/clear", "Clear conversation context"),
    (ResetContext, "/reset", "Reset conversation context"),
    (ContextPolicy, "/context policy", "Show compaction policy state"),
    (ContextPolicyOn, "/context policy on", "Enable automatic compaction"),
    (ContextPolicyOff, "/context policy off", "Disable automatic compaction"),
    (ContextPolicyReserve, "/context policy reserve", "Set reserved context tokens"),
    (ContextPolicyKeepRecent, "/context policy keep", "Set recent context tokens"),
    (Settings, "/settings", "Open persistent settings"),
    (Doctor, "/doctor", "Diagnose runtime integrations"),
    (DoctorInspect, "/doctor inspect", "Inspect runtime integrations"),
    (DoctorFix, "/doctor fix", "Apply diagnostic fixes"),
    (RewindSession, "/rewind", "Restore an earlier conversation state"),
    (PromptHistory, "/history", "Search previous prompts"),
    (FindTranscript, "/find", "Search the transcript"),
    (JumpTranscript, "/jump", "Jump to a transcript entry"),
    (Recap, "/recap", "Summarize the current task and state"),
    (SetEffort, "/effort", "Set reasoning effort"),
    (AlwaysApprove, "/always-approve", "Toggle approval-free execution"),
    (AutoApprove, "/auto", "Automatically approve safe tools"),
    (AskBeforeTools, "/ask", "Ask before executing tools"),
    (ApprovalHistory, "/approval history", "Show recent approval decisions"),
    (Deny, "/deny", "Block tool execution"),
    (PlanMode, "/plan", "Enter plan mode"),
    (ViewPlan, "/view-plan", "Show the current plan"),
    (Login, "/login", "Authenticate a provider"),
    (Logout, "/logout", "Remove provider credentials"),
    (Reload, "/reload", "Reload configuration and resources"),
    (TrustProject, "/trust", "Trust or revoke a project"),
    (Skills, "/skills", "List or invoke skills"),
    (Hooks, "/hooks", "Manage project hooks"),
    (Plugins, "/plugins", "Manage plugins"),
    (Mcps, "/mcps", "Manage MCP servers"),
    (CloseMcps, "/mcps close", "Close MCP servers"),
    (ReconnectMcps, "/mcps reconnect", "Reconnect MCP servers"),
    (McpNotifications, "/mcps notifications", "Inspect MCP notifications"),
    (ClearMcpNotifications, "/mcps notifications clear", "Clear MCP notifications"),
    (McpPopNotifications, "/mcps notifications pop", "Pop the oldest MCP notification"),
    (McpReady, "/mcps status=ready", "Show ready MCP servers"),
    (McpFailed, "/mcps status=failed", "Show failed MCP servers"),
    (McpBusy, "/mcps status=busy", "Show busy MCP servers"),
    (McpClosed, "/mcps status=closed", "Show closed MCP servers"),
    (McpStdio, "/mcps stdio", "Show stdio MCP servers"),
    (McpHttp, "/mcps http", "Show HTTP MCP servers"),
    (Memory, "/memory", "Browse or configure memory"),
    (Remember, "/remember", "Save a note immediately"),
    (Goal, "/goal", "Create or manage an autonomous goal"),
    (Workflow, "/workflow", "Launch or manage a workflow"),
    (Workflows, "/workflows", "Show active workflow runs"),
    (Loop, "/loop", "Schedule recurring work"),
    (DeepResearch, "/deep-research", "Run bounded research"),
    (Feedback, "/feedback", "Submit feedback"),
    (Usage, "/usage", "Show usage or billing information"),
    (UsageChart, "/usage chart", "Show usage series chart"),
    (UsageChartAll, "/usage chart", "Show all usage series"),
    (UsageChartInput, "/usage chart input", "Show input usage series"),
    (UsageChartOutput, "/usage chart output", "Show output usage series"),
    (UsageChartCost, "/usage chart cost", "Show cost usage series"),
    (Jobs, "/jobs", "Inspect owned background jobs"),
    (ActiveJobs, "/jobs scheduler active", "Show queued and running work"),
    (CancelAllJobs, "/jobs cancel all", "Cancel all running jobs"),
    (CancelRunningJobs, "/jobs cancel running", "Cancel currently running jobs"),
    (ClearFinishedJobs, "/jobs clear finished", "Clear finished job rows"),
    (CompletedJobs, "/jobs completed", "Show completed jobs"),
    (QueuedJobs, "/jobs scheduler queued", "Show queued scheduler work"),
    (FailedJobs, "/jobs failed", "Show failed jobs"),
    (RunningJobs, "/jobs running", "Show running jobs"),
    (CancelledJobs, "/jobs cancelled", "Show cancelled jobs"),
    (CancelledSchedulerJobs, "/jobs scheduler cancelled", "Show cancellation metrics"),
    (CancelQueuedJobs, "/jobs scheduler cancel queued", "Cancel queued tool batches"),
    (JobOutput, "/jobs output", "Inspect bounded job output"),
    (JobOutputFacts, "/jobs output", "Inspect bounded output facts"),
    (JobOutputPreview, "/jobs output", "Inspect the bounded output preview"),
    (JobOutputSearch, "/jobs output", "Search bounded job output"),
    (JobOutputHead, "/jobs output", "Inspect the first output lines"),
    (JobOutputTail, "/jobs output", "Inspect the last output lines"),
    (GitStatus, "/git status", "Inspect repository status"),
    (GitDiff, "/git diff", "Inspect the unstaged Git diff"),
    (GitReview, "/git review", "Review the unstaged Git patch"),
    (GitCommitPrepare, "/git commit prepare", "Preview a commit without mutating Git"),
    (GitCommit, "/git commit", "Create a Git commit (approval required)"),
    (GitPush, "/git push", "Push an explicit Git ref (approval required)"),
    (GitRevert, "/git revert", "Revert an explicit Git commit (approval required)"),
    (GitWorktrees, "/git worktrees", "Inspect Git worktrees"),
    (GitConflicts, "/git conflicts", "Inspect unresolved Git conflicts"),
    (GitConflictsInspect, "/git conflicts action inspect", "Inspect a conflict path"),
    (GitConflictsCancel, "/git conflicts cancel", "Cancel conflict review"),
    (Questions, "/questions", "Browse user-question history"),
    (AnsweredQuestions, "/questions outcome=answered", "Show answered questions"),
    (CancelledQuestions, "/questions outcome=cancelled", "Show cancelled questions"),
    (RejectedQuestions, "/questions outcome=rejected", "Show rejected questions"),
    (ClearQuestionHistory, "/questions clear", "Clear question history"),
    (PendingQuestions, "/questions pending", "Show pending user questions"),
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

macro_rules! palette_sections {
    ($(($section:literal => [$($action:ident),+ $(,)?])),+ $(,)?) => {
        fn palette_section(action: PaletteAction) -> &'static str {
            match action {
                $( $(PaletteAction::$action)|+ => $section, )+
                _ => "Model & Input",
            }
        }
    };
}

palette_sections! {
    ("Context" => [CopyLastResponse, SessionInfo, CompactContextNow, Questions, AnsweredQuestions, CancelledQuestions, RejectedQuestions, ClearQuestionHistory, PendingQuestions]),
    ("Session" => [NewSession, KeyboardShortcuts, Quit, Changelog, ShareSession, SessionHistory, SessionHistoryQuery, UndoSession, ResumeSessionPicker]),
    ("Information" => [Help, ContextInfo, ClearContext, ResetContext, ContextPolicy, ContextPolicyOn, ContextPolicyOff, ContextPolicyReserve, ContextPolicyKeepRecent, Doctor, DoctorInspect, DoctorFix, Feedback, Usage, UsageChart, UsageChartAll, UsageChartInput, UsageChartOutput, UsageChartCost, GitStatus, GitDiff, GitReview, GitCommitPrepare, GitCommit, GitPush, GitRevert, GitWorktrees, GitConflicts, GitConflictsInspect, GitConflictsCancel]),
    ("Extensions" => [Skills, Hooks, Plugins, Mcps, CloseMcps, ReconnectMcps, McpNotifications, ClearMcpNotifications, McpPopNotifications, McpReady, McpFailed, McpBusy, McpClosed, McpStdio, McpHttp, Memory]),
    ("Automation" => [Goal, Workflow, Workflows, Loop, DeepResearch, Jobs, ActiveJobs, CancelAllJobs, CancelRunningJobs, ClearFinishedJobs, CompletedJobs, QueuedJobs, FailedJobs, RunningJobs, CancelledJobs, CancelQueuedJobs, AskBeforeTools, Deny]),
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
    pub fn requires_parameters(&self) -> bool {
        PARAMETERIZED_ACTIONS.contains(self)
    }

    pub const fn slash_command(&self) -> &'static str {
        palette_slash_command!(self)
    }

    pub const fn parameter_hint(&self) -> &'static str {
        match self {
            Self::SetSessionName => "Session name",
            Self::SessionHistoryQuery => "Query",
            Self::UndoSession => "Count (optional)",
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
            Self::ContextPolicyReserve => "Token count",
            Self::ContextPolicyKeepRecent => "Token count",
            Self::JobOutput | Self::JobOutputFacts | Self::JobOutputPreview => "Job id",
            Self::JobOutputSearch => "Job id and search text",
            Self::GitCommitPrepare | Self::GitCommit => "Commit message",
            Self::GitPush => "Remote and reference",
            Self::GitRevert => "Commit id",
            Self::JobOutputHead | Self::JobOutputTail => "Job id and line count",
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

const PARAMETERIZED_ACTIONS: &[PaletteAction] = &[
    PaletteAction::SetSessionName,
    PaletteAction::UndoSession,
    PaletteAction::SessionHistoryQuery,
    PaletteAction::SelectTheme,
    PaletteAction::CompactContext,
    PaletteAction::ForkSession,
    PaletteAction::SelectTreeEntry,
    PaletteAction::ExportSession,
    PaletteAction::ImportSession,
    PaletteAction::CloneSession,
    PaletteAction::ResumeSession,
    PaletteAction::Help,
    PaletteAction::Settings,
    PaletteAction::Doctor,
    PaletteAction::RewindSession,
    PaletteAction::PromptHistory,
    PaletteAction::FindTranscript,
    PaletteAction::JumpTranscript,
    PaletteAction::SetEffort,
    PaletteAction::AlwaysApprove,
    PaletteAction::AutoApprove,
    PaletteAction::PlanMode,
    PaletteAction::Login,
    PaletteAction::Logout,
    PaletteAction::TrustProject,
    PaletteAction::Remember,
    PaletteAction::Goal,
    PaletteAction::Workflow,
    PaletteAction::Loop,
    PaletteAction::DeepResearch,
    PaletteAction::Feedback,
    PaletteAction::Usage,
    PaletteAction::ContextPolicyReserve,
    PaletteAction::ContextPolicyKeepRecent,
    PaletteAction::Jobs,
    PaletteAction::Questions,
    PaletteAction::GitCommitPrepare,
    PaletteAction::GitCommit,
    PaletteAction::GitPush,
    PaletteAction::GitRevert,
    PaletteAction::JobOutput,
    PaletteAction::JobOutputFacts,
    PaletteAction::JobOutputPreview,
    PaletteAction::JobOutputSearch,
    PaletteAction::JobOutputHead,
    PaletteAction::JobOutputTail,
];

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
