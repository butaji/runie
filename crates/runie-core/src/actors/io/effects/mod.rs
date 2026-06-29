//! Sync effect handlers for IoActor (bash, files, git, clipboard, editor, gist).

mod clipboard;
mod editor;
mod gist;

pub use clipboard::{read_clipboard_sync, write_clipboard_sync};
pub use editor::open_editor_sync;
pub use gist::share_session_sync;
