#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used)]

pub mod theme;
pub mod components;
pub mod tui;

pub use theme::{ThemeWrapper, ColorPalette, OpalineColor};
pub use tui::{Tui, TuiConfig, TuiMode, TuiAction, Msg, Cmd, update, event_to_msg};
pub use components::{
    TopBar,
    MessageList,
    MessageItem,
    InputBar,
    InputMode,
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
    PaletteItem,
    PaletteStep,
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