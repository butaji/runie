//! Renderer-independent UI actor messages.

use runie_core::types::AgentEvent;

use crate::{
    DialogSpec, DialogStack, CHANGELOG_DIALOG, COMMAND_DIALOG, COMMAND_RESULT_DIALOG,
    FILE_SELECTOR_DIALOG, MODEL_SELECTOR_DIALOG, PALETTE_PARAMETERS_DIALOG, SESSION_DIALOG,
    SHORTCUTS_DIALOG, USER_QUESTION_DIALOG,
};

pub use crate::palette_meta::palette_display_rows;

runie_core::typed_action_registry! {
    pub enum PaletteAction {
        NewSession => "New Session",
        KeyboardShortcuts => "Keyboard Shortcuts",
        Quit => "Quit",
        Changelog => "Changelog",
        CopyLastResponse => "Copy Last Response",
        SessionInfo => "Session Info",
        SessionHistory => "Session History",
        UndoSession => "Undo Session",
        SelectModel => "Select Model",
        SelectTheme => "Select Theme",
        ManageProviders => "Manage Providers",
        ScopedModels => "Scoped Models",
        SetSessionName => "Set Session Name",
        CompactContext => "Compact Context",
        ForkSession => "Fork Session",
        SelectTreeEntry => "Select Tree Entry",
        ExportSession => "Export Session",
        ImportSession => "Import Session",
        CloneSession => "Clone Session",
        ResumeSession => "Resume Session",
        ShareSession => "Share Session",
        Help => "Help",
        ContextInfo => "Context Info",
        Settings => "Settings",
        Doctor => "Doctor",
        RewindSession => "Rewind Session",
        PromptHistory => "Prompt History",
        FindTranscript => "Find Transcript",
        JumpTranscript => "Jump Transcript",
        Recap => "Recap",
        SetEffort => "Set Reasoning Effort",
        AlwaysApprove => "Always Approve",
        AutoApprove => "Automatic Approval",
        PlanMode => "Plan Mode",
        ViewPlan => "View Plan",
        Login => "Login",
        Logout => "Logout",
        Reload => "Reload",
        TrustProject => "Trust Project",
        Skills => "Skills",
        Hooks => "Hooks",
        Plugins => "Plugins",
        Mcps => "MCP Servers",
        McpReady => "Ready MCP Servers",
        McpFailed => "Failed MCP Servers",
        Memory => "Memory",
        Remember => "Remember",
        Goal => "Goal",
        Workflow => "Workflow",
        Workflows => "Workflows",
        Loop => "Loop",
        DeepResearch => "Deep Research",
        Feedback => "Feedback",
        Usage => "Usage",
        Jobs => "Background Jobs",
        ActiveJobs => "Active Jobs",
        CancelAllJobs => "Cancel All Jobs",
        ClearFinishedJobs => "Clear Finished Jobs",
        JobOutput => "Job Output",
        GitStatus => "Git Status",
        GitDiff => "Git Diff",
        GitConflicts => "Git Conflicts",
        Questions => "User Questions",
    }
}

include!("ui_messages.rs");

/// Translate core lifecycle events into UI-owned reducer messages.
/// Unsupported core events intentionally produce no UI transition.
pub fn ui_messages_for_event(event: &AgentEvent) -> Vec<UiMsg> {
    match event {
        AgentEvent::Reset => vec![UiMsg::Reset],
        AgentEvent::AgentStart
        | AgentEvent::AgentEnd { .. }
        | AgentEvent::Error { .. }
        | AgentEvent::ThinkingLevelChanged { .. }
        | AgentEvent::TurnStart
        | AgentEvent::Waiting { .. }
        | AgentEvent::ThemeChanged { .. }
        | AgentEvent::ModelChanged { .. }
        | AgentEvent::ActiveToolsChanged { .. }
        | AgentEvent::SessionLabelChanged { .. }
        | AgentEvent::SessionNameChanged { .. }
        | AgentEvent::SessionLaneChanged { .. }
        | AgentEvent::SessionEntryAppended { .. }
        | AgentEvent::BranchSummaryCreated { .. }
        | AgentEvent::CustomSessionEntryCreated { .. }
        | AgentEvent::CompactionCreated { .. }
        | AgentEvent::OperationRecordCreated { .. }
        | AgentEvent::TypedOperationRecordCreated { .. }
        | AgentEvent::ToolDisplayModeChanged { .. }
        | AgentEvent::TurnEnd { .. }
        | AgentEvent::MessageStart { .. }
        | AgentEvent::MessageUpdate { .. }
        | AgentEvent::MessageEnd { .. }
        | AgentEvent::ToolExecutionStart { .. }
        | AgentEvent::ToolExecutionUpdate { .. }
        | AgentEvent::ToolExecutionEnd { .. }
        | AgentEvent::BackgroundWorkStarted { .. }
        | AgentEvent::BackgroundWorkProgress { .. }
        | AgentEvent::BackgroundWorkFinished { .. }
        | AgentEvent::BackgroundWorkCancelled { .. }
        | AgentEvent::WorkflowStarted { .. }
        | AgentEvent::WorkflowProgress { .. }
        | AgentEvent::WorkflowFinished { .. } => Vec::new(),
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UiState {
    pub show_welcome: bool,
    pub shortcuts_open: bool,
    pub command_palette_open: bool,
    pub command_palette_query: String,
    pub command_palette_index: usize,
    pub last_palette_command: Option<String>,
    pub last_skill_command: Option<String>,
    pub skill_rows: Vec<String>,
    pub model_selector_open: bool,
    pub model_selector_query: String,
    pub model_selector_index: usize,
    pub model_selector_scoped_only: bool,
    pub model_selector_result_count: usize,
    pub model_selector_rows: Vec<String>,
    pub session_info_open: bool,
    pub changelog_open: bool,
    pub dialog_stack: DialogStack,
    pub palette_parameter_action: Option<PaletteAction>,
    pub palette_parameter_options: Vec<String>,
    pub command_result: Option<String>,
    pub user_question: Option<runie_core::tools::PendingUserQuestion>,
    pub user_question_selected: Vec<usize>,
}

impl UiState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_welcome() -> Self {
        Self {
            show_welcome: true,
            ..Self::default()
        }
    }

    pub fn update(mut self, msg: UiMsg) -> Self {
        if is_overlay_message(&msg) {
            return self
                .update_overlay(msg)
                .expect("overlay message is handled by an overlay reducer");
        }
        self.update_general(msg);
        self
    }

    fn update_overlay(mut self, msg: UiMsg) -> Option<Self> {
        match msg {
            UiMsg::CommandPaletteChar(_)
            | UiMsg::CommandPaletteBackspace
            | UiMsg::CommandPaletteMove(_)
            | UiMsg::CommandPaletteEscape
            | UiMsg::ActivateCommandPalette => self.update_palette(msg),
            UiMsg::ModelSelectorChar(_)
            | UiMsg::ModelSelectorBackspace
            | UiMsg::ModelSelectorMove(_)
            | UiMsg::ModelSelectorEscape
            | UiMsg::ModelSelectorToggleScope
            | UiMsg::ActivateModelSelector => self.update_model_selector(msg),
            UiMsg::PaletteParameterChar(_)
            | UiMsg::PaletteParameterBackspace
            | UiMsg::PaletteParameterMove(_)
            | UiMsg::PaletteParameterPreview
            | UiMsg::PaletteParameterSubmit => self.update_parameters(msg),
            UiMsg::UserQuestionMove(_)
            | UiMsg::ToggleUserQuestionSelection
            | UiMsg::SubmitUserQuestion => self.update_user_question(msg),
            _ => None,
        }
    }

    fn update_general(&mut self, msg: UiMsg) {
        if self.update_toggle(&msg) {
            return;
        }
        self.update_data(msg);
    }

    fn update_toggle(&mut self, msg: &UiMsg) -> bool {
        match msg {
            UiMsg::HideWelcome => self.show_welcome = false,
            UiMsg::ToggleShortcuts => {
                self.shortcuts_open = !self.shortcuts_open;
                self.open_dialog_for_toggle(SHORTCUTS_DIALOG);
            }
            UiMsg::ToggleCommandPalette => {
                self.toggle_command_palette();
            }
            UiMsg::ToggleModelSelector => {
                self.toggle_model_selector();
            }
            UiMsg::ToggleSessionInfo => {
                self.session_info_open = !self.session_info_open;
                self.open_dialog_for_toggle(SESSION_DIALOG);
            }
            UiMsg::ToggleChangelog => {
                self.changelog_open = !self.changelog_open;
                self.open_dialog_for_toggle(CHANGELOG_DIALOG);
            }
            UiMsg::DialogEscape => self.escape_dialog(),
            UiMsg::CloseDialogs => self.close_dialogs(),
            _ => return false,
        };
        true
    }

    fn toggle_command_palette(&mut self) {
        self.command_palette_open = !self.command_palette_open;
        self.last_palette_command = None;
        self.last_skill_command = None;
        if !self.command_palette_open {
            self.command_palette_query.clear();
            self.command_palette_index = 0;
            self.dialog_stack.pop();
        } else if self.dialog_stack.is_empty() {
            self.dialog_stack.push(COMMAND_DIALOG);
        }
    }

    fn toggle_model_selector(&mut self) {
        self.model_selector_open = !self.model_selector_open;
        self.open_dialog_for_toggle(MODEL_SELECTOR_DIALOG);
        if !self.model_selector_open {
            self.model_selector_query.clear();
            self.model_selector_index = 0;
        }
    }

    fn update_data(&mut self, msg: UiMsg) {
        match msg {
            UiMsg::OpenUserQuestion(question) => {
                self.user_question = Some(question);
                self.user_question_selected.clear();
                self.dialog_stack.push(USER_QUESTION_DIALOG);
            }
            UiMsg::OpenFileDialog => self.dialog_stack.push(FILE_SELECTOR_DIALOG),
            UiMsg::OpenPaletteParameters(action) => {
                self.palette_parameter_action = Some(action);
                if self.command_palette_open && self.dialog_stack.top_id() != Some("commands") {
                    self.dialog_stack.push(COMMAND_DIALOG);
                }
                self.dialog_stack.push(PALETTE_PARAMETERS_DIALOG);
            }
            UiMsg::SetModelSelectorResultCount(count) => {
                self.model_selector_result_count = count;
                self.model_selector_index = self.model_selector_index.min(count.saturating_sub(1));
            }
            UiMsg::SetModelSelectorRows(rows) => {
                self.model_selector_result_count = rows.len();
                self.model_selector_rows = rows;
                self.model_selector_index = self
                    .model_selector_index
                    .min(self.model_selector_result_count.saturating_sub(1));
            }
            UiMsg::SetSkillRows(rows) => self.skill_rows = rows,
            UiMsg::SetPaletteParameterOptions(options) => self.palette_parameter_options = options,
            UiMsg::ShowCommandResult(result) => {
                self.command_result = Some(result);
                self.dialog_stack.push(COMMAND_RESULT_DIALOG);
            }
            UiMsg::Reset => *self = Self::new(),
            UiMsg::UserQuestionMove(_)
            | UiMsg::ToggleUserQuestionSelection
            | UiMsg::SubmitUserQuestion => unreachable!(),
            UiMsg::CopyText(_) => {}
            _ => unreachable!("overlay or toggle message handled above"),
        }
    }

    fn update_parameters(&mut self, msg: UiMsg) -> Option<Self> {
        if matches!(msg, UiMsg::PaletteParameterPreview) {
            return Some(self.clone());
        }
        if matches!(msg, UiMsg::PaletteParameterSubmit) {
            self.dialog_stack.pop();
            self.palette_parameter_action = None;
            if self.dialog_stack.top_id() == Some("commands") {
                self.dialog_stack.pop();
            }
            self.command_palette_open = false;
            self.command_palette_query.clear();
            self.command_palette_index = 0;
            return Some(self.clone());
        }
        let frame = self.dialog_stack.top_mut()?;
        match msg {
            UiMsg::PaletteParameterChar(ch) => frame.query.push(ch),
            UiMsg::PaletteParameterBackspace => {
                frame.query.pop();
            }
            UiMsg::PaletteParameterMove(delta) => {
                let count = if self.palette_parameter_options.is_empty() {
                    crate::theme_labels().len()
                } else {
                    self.palette_parameter_options.len()
                };
                frame.selected = crate::wrap_dialog_selection(frame.selected, delta, count);
            }
            _ => return None,
        }
        Some(self.clone())
    }

    fn open_dialog_for_toggle(&mut self, spec: DialogSpec) {
        if self.dialog_stack.top_id() == Some(spec.id) {
            self.dialog_stack.pop();
        } else {
            self.dialog_stack.push(spec);
        }
    }

    fn escape_dialog(&mut self) {
        let Some(id) = self.dialog_stack.top_id() else {
            return;
        };
        if id == "commands" && !self.command_palette_query.is_empty() {
            self.command_palette_query.clear();
            self.command_palette_index = 0;
            return;
        }
        self.dialog_stack.pop();
        match id {
            "commands" => {
                self.command_palette_open = false;
                self.command_palette_query.clear();
                self.command_palette_index = 0;
            }
            "model" => {
                self.model_selector_open = false;
                self.model_selector_query.clear();
                self.model_selector_index = 0;
            }
            "shortcuts" => self.shortcuts_open = false,
            "session" => self.session_info_open = false,
            "changelog" => self.changelog_open = false,
            "palette-parameters" => {
                self.palette_parameter_action = None;
                if self.dialog_stack.top_id() == Some("commands") {
                    self.command_palette_open = true;
                }
            }
            "command-result" => self.command_result = None,
            _ => {}
        }
    }

    fn close_dialogs(&mut self) {
        self.dialog_stack.clear();
        self.shortcuts_open = false;
        self.command_palette_open = false;
        self.command_palette_query.clear();
        self.command_palette_index = 0;
        self.model_selector_open = false;
        self.model_selector_query.clear();
        self.model_selector_index = 0;
        self.session_info_open = false;
        self.changelog_open = false;
        self.palette_parameter_action = None;
        self.command_result = None;
        self.user_question = None;
        self.user_question_selected.clear();
    }

    fn update_palette(mut self, msg: UiMsg) -> Option<Self> {
        match msg {
            UiMsg::CommandPaletteChar(ch) => self.command_palette_query.push(ch),
            UiMsg::CommandPaletteBackspace => {
                self.command_palette_query.pop();
            }
            UiMsg::CommandPaletteMove(delta) => self.move_palette(delta),
            UiMsg::CommandPaletteEscape => self.escape_palette(),
            UiMsg::ActivateCommandPalette => self.activate_palette(),
            _ => return None,
        }
        Some(self)
    }

    fn move_palette(&mut self, delta: isize) {
        let count = palette_labels(&self.command_palette_query, &self.skill_rows).len();
        self.command_palette_index =
            crate::wrap_dialog_selection(self.command_palette_index, delta, count);
    }

    fn escape_palette(&mut self) {
        if self.command_palette_query.is_empty() {
            self.command_palette_open = false;
            self.dialog_stack.pop();
        } else {
            self.command_palette_query.clear();
        }
        self.command_palette_index = 0;
    }

    fn activate_palette(&mut self) {
        let labels = palette_labels(&self.command_palette_query, &self.skill_rows);
        let selected = labels.get(self.command_palette_index).cloned();
        self.last_palette_command = selected
            .clone()
            .filter(|label| !label.starts_with("/skills:"));
        self.last_skill_command = selected
            .clone()
            .filter(|label| label.starts_with("/skills:"));
        let keep_palette_as_parent = selected
            .as_deref()
            .and_then(PaletteAction::from_label)
            .is_some_and(|action| {
                action.requires_parameters() || action == PaletteAction::SelectModel
            });
        self.command_palette_open = keep_palette_as_parent;
        if !keep_palette_as_parent {
            self.command_palette_query.clear();
            self.command_palette_index = 0;
            self.dialog_stack.pop();
        }
    }

    fn update_model_selector(mut self, msg: UiMsg) -> Option<Self> {
        match msg {
            UiMsg::ModelSelectorChar(ch) => self.model_selector_query.push(ch),
            UiMsg::ModelSelectorBackspace => {
                self.model_selector_query.pop();
            }
            UiMsg::ModelSelectorMove(delta) => self.move_model_selector(delta),
            UiMsg::ModelSelectorEscape => self.escape_model_selector(),
            UiMsg::ModelSelectorToggleScope => self.toggle_model_scope(),
            UiMsg::ActivateModelSelector => self.activate_model_selector(),
            _ => return None,
        }
        Some(self)
    }

    fn move_model_selector(&mut self, delta: isize) {
        self.model_selector_index = crate::wrap_dialog_selection(
            self.model_selector_index,
            delta,
            self.model_selector_result_count,
        );
    }

    fn escape_model_selector(&mut self) {
        if self.model_selector_query.is_empty() {
            self.model_selector_open = false;
            if self.dialog_stack.top_id() == Some("model") {
                self.dialog_stack.pop();
            }
        } else {
            self.model_selector_query.clear();
        }
        self.model_selector_index = 0;
    }

    fn toggle_model_scope(&mut self) {
        self.model_selector_scoped_only = !self.model_selector_scoped_only;
        self.model_selector_index = 0;
    }
    fn activate_model_selector(&mut self) {
        self.model_selector_open = false;
        self.model_selector_query.clear();
        self.model_selector_index = 0;
        if self.dialog_stack.top_id() == Some("model") {
            self.dialog_stack.pop();
        }
        if self.dialog_stack.top_id() == Some("commands") {
            self.dialog_stack.pop();
            self.command_palette_open = false;
            self.command_palette_query.clear();
            self.command_palette_index = 0;
        }
    }
}
include!("ui_palette.rs");
include!("ui_question.rs");
#[cfg(test)]
#[path = "ui_tests.rs"]
mod dialog_palette_tests;
