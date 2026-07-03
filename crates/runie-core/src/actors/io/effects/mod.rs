//! Sync effect handlers for IoActor (bash, files, git, clipboard, editor, gist).

#[cfg(feature = "clipboard")]
mod clipboard;
mod editor;
mod gist;

#[cfg(feature = "clipboard")]
pub use clipboard::{read_clipboard_sync, write_clipboard_sync};
pub use editor::open_editor_sync;
pub use gist::share_session_sync;
