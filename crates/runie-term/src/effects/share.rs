//! Share session effect handler.

use runie_core::{ChatMessage, Event as CoreEvent};
use tokio::sync::mpsc;

/// Upload the session to a gist and emit a `SystemMessage` with the result.
pub fn run(messages: Vec<ChatMessage>, display_name: Option<String>, tx: mpsc::Sender<CoreEvent>) {
    tokio::spawn(async move {
        match crate::share::share_session(&messages, display_name.as_deref()).await {
            Ok(url) => {
                let _ = tx
                    .send(CoreEvent::SystemMessage {
                        content: format!("Shared session: {}", url),
                    })
                    .await;
            }
            Err(e) => {
                let _ = tx
                    .send(CoreEvent::SystemMessage {
                        content: format!("Could not share session: {}", e),
                    })
                    .await;
            }
        }
    });
}
