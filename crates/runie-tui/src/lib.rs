#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used)]

pub mod theme;
pub mod components;
pub mod tui;
pub mod pipe;
pub mod actors;

pub use theme::{ThemeWrapper, ColorPalette, OpalineColor};
pub use tui::{Tui, TuiConfig, TuiMode, Msg, Cmd, update, event_to_msg, Onboarding, AppState};
pub use components::{
    MessageList,
    MessageItem,
    StatusBar,
    Overlay,
    CodeBlock,
    CodeLine,
    LineStatus,
    Collapsible,
    AgentList,
    AgentItem,
    AgentStatus,
    PermissionModal,
    PermissionAction,
    CommandPalette,
    PaletteCommand,
    ContextPanel,
    GitChange,
    GitStatus,
    SessionTreeNavigator,
    SessionTreeEntry,
};

// Re-export ratatui types we use
pub use ratatui::{
    layout::{Rect, Constraint, Layout, Direction},
    style::{Style, Color, Modifier},
    buffer::Buffer,
    text::{Line, Span, Text},
};