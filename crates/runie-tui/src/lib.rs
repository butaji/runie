//! `runie-tui` — minimal ratatui/crossterm TUI that subscribes to
//! `runie-core`'s event stream and renders a 3-region chat interface
//! (scrollback transcript + prompt input + status bar).
//!
//! See `tasks/` for the implementation plan; the e2e test
//! (`tests/e2e_test.rs`) is the behavioural contract.

pub mod app;
pub mod event_renderer;
pub mod key;
pub mod layout;
pub mod widgets;
pub mod yaml_runner;

pub use app::{App, AppExit};
pub use event_renderer::EventRenderer;
pub use key::{Action, map_key};
pub use layout::{chat_layout, ChatLayout, PROMPT_HEIGHT, STATUS_HEIGHT};
pub use widgets::{PromptOutcome, PromptWidget, Scrollback, Status, StatusBar};