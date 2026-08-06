//! Renderer-independent UI actor messages.

runie_core::typed_action_registry! {
    pub enum PaletteAction {
        NewSession => "New Session",
        NewSessionInWorktree => "New Session in Worktree",
        AgentDashboard => "Agent Dashboard",
        BackToHome => "Back to Home",
        DeleteThisSession => "Delete This Session",
        ResumeSession => "Resume Session",
        ShareSession => "Share Session",
        RenameSession => "Rename Session",
        SessionInfo => "Session Info",
        CompactHistory => "Compact History",
        ContextUsage => "Context Usage",
        ViewPlan => "View Plan",
        Memory => "Memory",
        SwitchModel => "Switch Model",
        KeyboardShortcuts => "Keyboard Shortcuts",
        Quit => "Quit",
    }
}

impl PaletteAction {
    pub fn filtered_labels(query: &str) -> Vec<&'static str> {
        let query = query.to_ascii_lowercase();
        Self::labels()
            .iter()
            .copied()
            .filter(|entry| query.is_empty() || entry.to_ascii_lowercase().contains(&query))
            .collect()
    }

    pub fn selected_label(query: &str, selected: usize) -> Option<&'static str> {
        Self::filtered_labels(query).into_iter().nth(selected)
    }

    pub fn entry_count(query: &str) -> usize {
        Self::filtered_labels(query).len()
    }
}

/// Pure intent emitted by the UI actor for an effect-owning consumer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiCommand {
    ActivatePaletteEntry(PaletteAction),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiMsg {
    HideWelcome,
    ToggleShortcuts,
    ToggleCommandPalette,
    CommandPaletteChar(char),
    CommandPaletteBackspace,
    CommandPaletteMove(isize),
    ActivateCommandPalette,
    Reset,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UiState {
    pub show_welcome: bool,
    pub shortcuts_open: bool,
    pub command_palette_open: bool,
    pub command_palette_query: String,
    pub command_palette_index: usize,
    pub last_palette_command: Option<String>,
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
            UiMsg::CommandPaletteChar(ch) => self.command_palette_query.push(ch),
            UiMsg::CommandPaletteBackspace => {
                self.command_palette_query.pop();
            }
            UiMsg::CommandPaletteMove(delta) => {
                let count = PaletteAction::entry_count(&self.command_palette_query);
                if count > 0 {
                    self.command_palette_index = self
                        .command_palette_index
                        .saturating_add_signed(delta)
                        .min(count - 1);
                } else {
                    self.command_palette_index = 0;
                }
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
            UiMsg::Reset => self = Self::new(),
        }
        self
    }
}
