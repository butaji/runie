//! Renderer-independent UI actor messages.

use runie_core::types::AgentEvent;

runie_core::typed_action_registry! {
    pub enum PaletteAction {
        NewSession => "New Session",
        KeyboardShortcuts => "Keyboard Shortcuts",
        Quit => "Quit",
    }
}

/// Pure intent emitted by the UI actor for an effect-owning consumer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiCommand {
    ActivatePaletteEntry(PaletteAction),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiMsg {
    HideWelcome,
    ToggleShortcuts,
    ToggleCommandPalette,
    CommandPaletteChar(char),
    CommandPaletteBackspace,
    CommandPaletteMove(isize),
    CommandPaletteEscape,
    ActivateCommandPalette,
    ToggleModelSelector,
    ModelSelectorChar(char),
    ModelSelectorBackspace,
    ModelSelectorMove(isize),
    ModelSelectorEscape,
    ModelSelectorToggleScope,
    ActivateModelSelector,
    SetModelSelectorResultCount(usize),
    SetModelSelectorRows(Vec<String>),
    ToggleSessionInfo,
    Reset,
}

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
    pub model_selector_open: bool,
    pub model_selector_query: String,
    pub model_selector_index: usize,
    pub model_selector_scoped_only: bool,
    pub model_selector_result_count: usize,
    pub model_selector_rows: Vec<String>,
    pub session_info_open: bool,
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

    #[allow(clippy::too_many_lines)]
    pub fn update(mut self, msg: UiMsg) -> Self {
        if matches!(
            msg,
            UiMsg::CommandPaletteChar(_)
                | UiMsg::CommandPaletteBackspace
                | UiMsg::CommandPaletteMove(_)
                | UiMsg::CommandPaletteEscape
                | UiMsg::ActivateCommandPalette
                | UiMsg::ModelSelectorChar(_)
                | UiMsg::ModelSelectorBackspace
                | UiMsg::ModelSelectorMove(_)
                | UiMsg::ModelSelectorEscape
                | UiMsg::ModelSelectorToggleScope
                | UiMsg::ActivateModelSelector
        ) {
            return if matches!(
                msg,
                UiMsg::ModelSelectorChar(_)
                    | UiMsg::ModelSelectorBackspace
                    | UiMsg::ModelSelectorMove(_)
                    | UiMsg::ModelSelectorEscape
                    | UiMsg::ModelSelectorToggleScope
                    | UiMsg::ActivateModelSelector
            ) {
                self.update_model_selector(msg)
                    .expect("selector message is handled by selector reducer")
            } else {
                self.update_palette(msg)
                    .expect("palette message is handled by palette reducer")
            };
        }
        match msg {
            UiMsg::HideWelcome => self.show_welcome = false,
            UiMsg::ToggleShortcuts => self.shortcuts_open = !self.shortcuts_open,
            UiMsg::ToggleCommandPalette => {
                self.command_palette_open = !self.command_palette_open;
                if !self.command_palette_open {
                    self.command_palette_query.clear();
                    self.command_palette_index = 0;
                }
            }
            UiMsg::CommandPaletteChar(_)
            | UiMsg::CommandPaletteBackspace
            | UiMsg::CommandPaletteMove(_)
            | UiMsg::CommandPaletteEscape
            | UiMsg::ActivateCommandPalette => unreachable!("palette messages handled above"),
            UiMsg::ToggleModelSelector => {
                self.model_selector_open = !self.model_selector_open;
                if !self.model_selector_open {
                    self.model_selector_query.clear();
                    self.model_selector_index = 0;
                }
            }
            UiMsg::ToggleSessionInfo => self.session_info_open = !self.session_info_open,
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
            UiMsg::ModelSelectorChar(_)
            | UiMsg::ModelSelectorBackspace
            | UiMsg::ModelSelectorMove(_)
            | UiMsg::ModelSelectorEscape
            | UiMsg::ModelSelectorToggleScope
            | UiMsg::ActivateModelSelector => unreachable!("selector messages handled above"),
            UiMsg::Reset => self = Self::new(),
        }
        self
    }

    fn update_palette(mut self, msg: UiMsg) -> Option<Self> {
        match msg {
            UiMsg::CommandPaletteChar(ch) => self.command_palette_query.push(ch),
            UiMsg::CommandPaletteBackspace => {
                self.command_palette_query.pop();
            }
            UiMsg::CommandPaletteMove(delta) => {
                let count = PaletteAction::entry_count(&self.command_palette_query);
                self.command_palette_index = if count == 0 {
                    0
                } else {
                    self.command_palette_index
                        .saturating_add_signed(delta)
                        .min(count - 1)
                };
            }
            UiMsg::CommandPaletteEscape => {
                if self.command_palette_query.is_empty() {
                    self.command_palette_open = false;
                } else {
                    self.command_palette_query.clear();
                }
                self.command_palette_index = 0;
            }
            UiMsg::ActivateCommandPalette => {
                self.last_palette_command = PaletteAction::selected_label(
                    &self.command_palette_query,
                    self.command_palette_index,
                )
                .map(str::to_owned);
                self.command_palette_open = false;
                self.command_palette_query.clear();
                self.command_palette_index = 0;
            }
            _ => return None,
        }
        Some(self)
    }

    fn update_model_selector(mut self, msg: UiMsg) -> Option<Self> {
        match msg {
            UiMsg::ModelSelectorChar(ch) => self.model_selector_query.push(ch),
            UiMsg::ModelSelectorBackspace => {
                self.model_selector_query.pop();
            }
            UiMsg::ModelSelectorMove(delta) => {
                let count = self.model_selector_result_count;
                self.model_selector_index = if count == 0 {
                    0
                } else {
                    self.model_selector_index
                        .saturating_add_signed(delta)
                        .min(count - 1)
                };
            }
            UiMsg::ModelSelectorEscape => {
                if self.model_selector_query.is_empty() {
                    self.model_selector_open = false;
                } else {
                    self.model_selector_query.clear();
                }
                self.model_selector_index = 0;
            }
            UiMsg::ModelSelectorToggleScope => {
                self.model_selector_scoped_only = !self.model_selector_scoped_only;
                self.model_selector_index = 0;
            }
            UiMsg::ActivateModelSelector => {
                self.model_selector_open = false;
                self.model_selector_query.clear();
                self.model_selector_index = 0;
            }
            _ => return None,
        }
        Some(self)
    }
}
