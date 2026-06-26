//! Sync effect handlers for IoActor (bash, files, git, clipboard, editor, gist).

mod gist;
mod editor;
mod clipboard;

pub use gist::share_session_sync;
pub use editor::open_editor_sync;
pub use clipboard::{write_clipboard_sync, read_clipboard_sync};
