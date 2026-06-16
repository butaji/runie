//! External editor effect handler.

use runie_core::event::ControlEvent;
use runie_core::Event as CoreEvent;
use std::io::{self, Write};
use tokio::sync::mpsc;

/// Spawn the user's `$EDITOR` with `text` and send the result back as an
/// `ExternalEditorDone` event.
pub fn run(text: String, tx: mpsc::Sender<CoreEvent>) {
    tokio::task::spawn_blocking(move || {
        let _ = spawn_external_editor_sync(text, tx);
    });
}

fn spawn_external_editor_sync(text: String, tx: mpsc::Sender<CoreEvent>) -> io::Result<()> {
    let editor = std::env::var("EDITOR")
        .unwrap_or_else(|_| if cfg!(windows) { "notepad" } else { "vi" }.to_string());

    let mut tmp = tempfile::NamedTempFile::new()?;
    tmp.write_all(text.as_bytes())?;
    tmp.flush()?;
    let path = tmp.into_temp_path();

    let status = std::process::Command::new(&editor).arg(&path).status()?;

    if status.success() {
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        let rt = tokio::runtime::Handle::try_current();
        if let Ok(handle) = rt {
            let _ = handle.block_on(tx.send(CoreEvent::Control(ControlEvent::ExternalEditorDone { content })));
        }
    }

    Ok(())
}
