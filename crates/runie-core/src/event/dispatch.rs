//! Action → Dispatch → Effect pattern (from Grok Build)
//!
//! This module never touches the terminal, network, or filesystem.
//! All mutations are synchronous and deterministic.
//! Async work is described as Effect values, not executed.

use crate::Event;

/// User intent enum - all possible actions from the UI
#[derive(Debug, Clone)]
pub enum Action {
    Submit(String),
    Navigate(Direction),
    Select(Selection),
    OpenPalette,
    CloseDialog,
    ToggleCommandPalette,
    OpenSettings,
    Quit,
    Abort,
    ScrollUp,
    ScrollDown,
    FocusUp,
    FocusDown,
    ToggleSidebar,
    ToggleDiff,
    ToggleCodeblock,
    CopyMessage,
    EditMessage,
    DeleteMessage,
    RetryMessage,
    StopGeneration,
    RequestPermission,
    AllowPermission,
    DenyPermission,
    NextProvider,
    PrevProvider,
    NextModel,
    PrevModel,
    ToggleReasoning,
    SelectResponse(usize),
    ExpandCollapsed,
    CollapseExpanded,
    OpenInEditor,
    ShowContext,
    HideContext,
    ToggleAutoApprove,
}

/// Direction for navigation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction { Up, Down, Left, Right }

/// Selection for list/tree items
#[derive(Debug, Clone)]
pub enum Selection {
    Index(usize),
    Range(usize, usize),
    All,
    None,
}

impl Selection {
    pub fn index(&self) -> Option<usize> {
        match self {
            Selection::Index(i) => Some(*i),
            _ => None,
        }
    }
}

/// Effects describe async work without executing it
#[derive(Debug, Clone)]
pub enum Effect {
    /// Spawn an agent with the given content
    SpawnAgent { content: String },
    /// Emit an event to the state machine
    EmitEvent(Event),
    /// Async task that will be spawned
    AsyncTask { task: Box<dyn std::future::Future<Output = ()> + Send + 'static> },
    /// Network request
    HttpRequest { request: HttpRequest },
    /// File system operation
    FileOp { operation: FileOperation },
    /// Tool call execution
    ToolCall { tool: String, args: serde_json::Value },
    /// Notification
    Notify { message: String, severity: NotificationSeverity },
}

#[derive(Debug, Clone)]
pub enum HttpRequest {
    Get { url: String, headers: Vec<(String, String)> },
    Post { url: String, headers: Vec<(String, String)>, body: Vec<u8> },
}

#[derive(Debug, Clone)]
pub enum FileOperation {
    Read { path: std::path::PathBuf },
    Write { path: std::path::PathBuf, content: Vec<u8> },
    Delete { path: std::path::PathBuf },
    Mkdir { path: std::path::PathBuf },
}

#[derive(Debug, Clone, Copy)]
pub enum NotificationSeverity { Info, Warning, Error }

impl std::fmt::Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Direction::Up => write!(f, "Up"),
            Direction::Down => write!(f, "Down"),
            Direction::Left => write!(f, "Left"),
            Direction::Right => write!(f, "Right"),
        }
    }
}

/// Pure sync dispatcher - converts Action → (StateDelta, Vec<Effect>)
pub struct Dispatcher;

impl Dispatcher {
    /// Dispatch an action and return the effects to execute
    pub fn dispatch(action: Action, state: &mut crate::AppState) -> Vec<Effect> {
        match action {
            Action::Submit(content) => {
                if content.trim().is_empty() {
                    return vec![];
                }
                vec![Effect::SpawnAgent { content }]
            }
            Action::Quit => {
                state.update(Event::ForceQuit);
                vec![]
            }
            Action::Abort => {
                state.update(Event::Abort);
                vec![]
            }
            Action::Navigate(dir) => {
                state.update(Event::Navigate(dir.into()));
                vec![]
            }
            Action::Select(selection) => {
                state.update(Event::Select(selection.into()));
                vec![]
            }
            Action::OpenPalette => {
                state.update(Event::OpenCommandPalette);
                vec![]
            }
            Action::CloseDialog => {
                state.update(Event::CloseDialog);
                vec![]
            }
            Action::ToggleCommandPalette => {
                state.update(Event::ToggleCommandPalette);
                vec![]
            }
            Action::OpenSettings => {
                state.update(Event::OpenSettings);
                vec![]
            }
            Action::ScrollUp => {
                state.update(Event::ScrollUp);
                vec![]
            }
            Action::ScrollDown => {
                state.update(Event::ScrollDown);
                vec![]
            }
            Action::FocusUp => {
                state.update(Event::FocusUp);
                vec![]
            }
            Action::FocusDown => {
                state.update(Event::FocusDown);
                vec![]
            }
            Action::ToggleSidebar => {
                state.update(Event::ToggleSidebar);
                vec![]
            }
            Action::ToggleDiff => {
                state.update(Event::ToggleDiff);
                vec![]
            }
            Action::ToggleCodeblock => {
                state.update(Event::ToggleCodeblock);
                vec![]
            }
            Action::CopyMessage => {
                state.update(Event::CopyMessage);
                vec![]
            }
            Action::EditMessage => {
                state.update(Event::EditMessage);
                vec![]
            }
            Action::DeleteMessage => {
                state.update(Event::DeleteMessage);
                vec![]
            }
            Action::RetryMessage => {
                state.update(Event::RetryMessage);
                vec![]
            }
            Action::StopGeneration => {
                state.update(Event::StopGeneration);
                vec![]
            }
            Action::RequestPermission => {
                state.update(Event::RequestPermission);
                vec![]
            }
            Action::AllowPermission => {
                state.update(Event::AllowPermission);
                vec![]
            }
            Action::DenyPermission => {
                state.update(Event::DenyPermission);
                vec![]
            }
            Action::NextProvider => {
                state.update(Event::NextProvider);
                vec![]
            }
            Action::PrevProvider => {
                state.update(Event::PrevProvider);
                vec![]
            }
            Action::NextModel => {
                state.update(Event::NextModel);
                vec![]
            }
            Action::PrevModel => {
                state.update(Event::PrevModel);
                vec![]
            }
            Action::ToggleReasoning => {
                state.update(Event::ToggleReasoning);
                vec![]
            }
            Action::SelectResponse(idx) => {
                state.update(Event::SelectResponse(idx));
                vec![]
            }
            Action::ExpandCollapsed => {
                state.update(Event::ExpandCollapsed);
                vec![]
            }
            Action::CollapseExpanded => {
                state.update(Event::CollapseExpanded);
                vec![]
            }
            Action::OpenInEditor => {
                state.update(Event::OpenInEditor);
                vec![]
            }
            Action::ShowContext => {
                state.update(Event::ShowContext);
                vec![]
            }
            Action::HideContext => {
                state.update(Event::HideContext);
                vec![]
            }
            Action::ToggleAutoApprove => {
                state.update(Event::ToggleAutoApprove);
                vec![]
            }
        }
    }

    /// Check if an action requires confirmation before execution
    pub fn requires_confirmation(action: &Action) -> bool {
        matches!(
            action,
            Action::DeleteMessage | Action::Abort | Action::Quit
        )
    }

    /// Get the action description for display
    pub fn describe(action: &Action) -> &'static str {
        match action {
            Action::Submit(_) => "Submit message",
            Action::Navigate(d) => match d {
                Direction::Up => "Navigate up",
                Direction::Down => "Navigate down",
                Direction::Left => "Navigate left",
                Direction::Right => "Navigate right",
            },
            Action::Select(_) => "Select item",
            Action::OpenPalette => "Open command palette",
            Action::CloseDialog => "Close dialog",
            Action::ToggleCommandPalette => "Toggle command palette",
            Action::OpenSettings => "Open settings",
            Action::Quit => "Quit application",
            Action::Abort => "Abort operation",
            Action::ScrollUp => "Scroll up",
            Action::ScrollDown => "Scroll down",
            Action::FocusUp => "Focus up",
            Action::FocusDown => "Focus down",
            Action::ToggleSidebar => "Toggle sidebar",
            Action::ToggleDiff => "Toggle diff view",
            Action::ToggleCodeblock => "Toggle codeblock",
            Action::CopyMessage => "Copy message",
            Action::EditMessage => "Edit message",
            Action::DeleteMessage => "Delete message",
            Action::RetryMessage => "Retry message",
            Action::StopGeneration => "Stop generation",
            Action::RequestPermission => "Request permission",
            Action::AllowPermission => "Allow permission",
            Action::DenyPermission => "Deny permission",
            Action::NextProvider => "Next provider",
            Action::PrevProvider => "Previous provider",
            Action::NextModel => "Next model",
            Action::PrevModel => "Previous model",
            Action::ToggleReasoning => "Toggle reasoning",
            Action::SelectResponse(_) => "Select response",
            Action::ExpandCollapsed => "Expand collapsed",
            Action::CollapseExpanded => "Collapse expanded",
            Action::OpenInEditor => "Open in editor",
            Action::ShowContext => "Show context",
            Action::HideContext => "Hide context",
            Action::ToggleAutoApprove => "Toggle auto-approve",
        }
    }
}

impl From<Direction> for crate::event::NavDirection {
    fn from(dir: Direction) -> Self {
        match dir {
            Direction::Up => crate::event::NavDirection::Up,
            Direction::Down => crate::event::NavDirection::Down,
            Direction::Left => crate::event::NavDirection::Left,
            Direction::Right => crate::event::NavDirection::Right,
        }
    }
}

impl From<Selection> for crate::event::SelectTarget {
    fn from(sel: Selection) -> Self {
        match sel {
            Selection::Index(i) => crate::event::SelectTarget::Index(i),
            Selection::Range(start, end) => crate::event::SelectTarget::Range(start, end),
            Selection::All => crate::event::SelectTarget::All,
            Selection::None => crate::event::SelectTarget::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_serialization() {
        let action = Action::Submit("Hello".to_string());
        let debug = format!("{:?}", action);
        assert!(debug.contains("Submit"));
        assert!(debug.contains("Hello"));
    }

    #[test]
    fn test_direction_display() {
        assert_eq!(Direction::Up.to_string(), "Up");
        assert_eq!(Direction::Down.to_string(), "Down");
        assert_eq!(Direction::Left.to_string(), "Left");
        assert_eq!(Direction::Right.to_string(), "Right");
    }

    #[test]
    fn test_selection_index() {
        assert_eq!(Selection::Index(5).index(), Some(5));
        assert_eq!(Selection::Range(1, 5).index(), None);
        assert_eq!(Selection::All.index(), None);
        assert_eq!(Selection::None.index(), None);
    }

    #[test]
    fn test_requires_confirmation() {
        assert!(!Dispatcher::requires_confirmation(&Action::Submit("test".into())));
        assert!(Dispatcher::requires_confirmation(&Action::DeleteMessage));
        assert!(Dispatcher::requires_confirmation(&Action::Quit));
        assert!(Dispatcher::requires_confirmation(&Action::Abort));
    }

    #[test]
    fn test_describe() {
        assert_eq!(Dispatcher::describe(&Action::Submit("test".into())), "Submit message");
        assert_eq!(Dispatcher::describe(&Action::OpenPalette), "Open command palette");
        assert_eq!(Dispatcher::describe(&Action::Quit), "Quit application");
    }
}
