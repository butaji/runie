//! `App` — the top-level TUI controller.

use std::sync::Arc;

use parking_lot::Mutex;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use runie_core::events::EventBus;
use runie_core::r#loop::LoopActor;
use runie_core::types::AgentMessage;

use crate::event_renderer::EventRenderer;
use crate::layout::chat_layout;
use crate::widgets::{PromptOutcome, PromptWidget, Scrollback, Status, StatusBar};

#[derive(Debug)]
pub enum AppExit {
    Quit,
    Error(String),
}

pub struct App {
    pub scrollback: Arc<Mutex<Scrollback>>,
    pub prompt: PromptWidget,
    pub status: Arc<Mutex<StatusBar>>,
    pub loop_actor: LoopActor,
    pub bus: EventBus,
    pub show_welcome: bool,
}

impl App {
    pub fn new(loop_actor: LoopActor, bus: EventBus) -> Self {
        Self {
            scrollback: Arc::new(Mutex::new(Scrollback::new())),
            prompt: PromptWidget::new(),
            status: Arc::new(Mutex::new(StatusBar::new())),
            loop_actor,
            bus,
            show_welcome: true,
        }
    }

    /// Handle a prompt outcome. Returns Some(text) on submit.
    pub async fn handle_prompt_outcome(&mut self, outcome: PromptOutcome) -> Option<String> {
        match outcome {
            PromptOutcome::Submitted(text) => {
                self.status.lock().set(Status::Thinking);
                let user_msg = AgentMessage::User(runie_core::types::UserMessage {
                    content: vec![runie_core::types::UserContent::Text { text: text.clone() }],
                    timestamp: 0,
                });
                let _ = self.loop_actor.prompt(vec![user_msg], runie_core::types::AgentContext::default()).await;
                self.status.lock().set(Status::Ready);
                Some(text)
            }
            PromptOutcome::Edited | PromptOutcome::Ignored => None,
        }
    }

    /// Spawn the renderer task. Owns the spawned task via JoinHandle.
    pub fn spawn_renderer(&self) -> (tokio::task::JoinHandle<()>, tokio::sync::watch::Sender<bool>) {
        let renderer = EventRenderer::new(self.scrollback.clone(), self.status.clone());
        let rx = self.bus.subscribe();
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        // OWNER: App — drives the renderer to completion.
        let handle = tokio::spawn(async move { renderer.run(rx, shutdown_rx).await });
        (handle, shutdown_tx)
    }

    /// Lay out the widgets and render them into the given area using `f`.
    pub fn render<F: FnMut(Rect, &mut Buffer)>(
        &mut self,
        area: Rect,
        mut f: F,
    ) {
        let layout = chat_layout(area);
        let mut sb = self.scrollback.lock();
        let mut buf = Buffer::empty(area);
        sb.render(layout.scrollback, &mut buf);
        f(layout.prompt, &mut buf);
        f(layout.status, &mut buf);
    }
}