//! Generated from TSX - DO NOT EDIT
//!
//! Source: export function App()

use renderer::{Renderer, RatatuiRenderer, VNode, VElement, VText, VProps, VStyle, FlexDirection};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::cell::RefCell;
use std::io::stdout;
use std::rc::Rc;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};

/// Component state derived from useState calls
struct AppState {
    count: i32,
}

struct App {
    state: AppState,
    renderer: RatatuiRenderer,
}

impl App {
    fn new() -> Self {
        Self {
            state: AppState {
                count: 0,
            },
            renderer: RatatuiRenderer::new(),
        }
    }

    fn render(&mut self, frame: &mut ratatui::Frame) {
        let area = frame.size();
        let buf = frame.buffer_mut();

        let vnode = VNode::Element(VElement {
            tag: "View".to_string(),
            props: VProps {
                style: VStyle { flex_direction: FlexDirection::Column, padding: 2, ..Default::default() },
                
                ..Default::default()
            },
            children: vec![VNode::Element(VElement {
            tag: "Text".to_string(),
            props: VProps {
                style: VStyle { color: Some("#FF5733".to_string()), ..Default::default() },
                
                ..Default::default()
            },
            children: vec![VNode::Text(VText { content: format!("Count: {}", self.state.count), style: VStyle { color: Some("#FF5733".to_string()), ..Default::default() } })],
        }), VNode::Element(VElement {
            tag: "Button".to_string(),
            props: VProps {
                style: VStyle::default(),
                on_press: Some(Box::new(|| {})), // Handled by dispatch_on_press
                ..Default::default()
            },
            children: vec![VNode::Text(VText { content: "Increment".to_string(), style: VStyle::default() })],
        })],
        });

        self.renderer.mount(vnode, area);
        self.renderer.render(buf);
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Enter => {
                self.dispatch_on_press();
                false
            }
            KeyCode::Char('q') => true,
            _ => false,
        }
    }

    /// Dispatches onPress handlers
    fn dispatch_on_press(&mut self) {
        self.state.count += 2;
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    execute!(stdout(), EnterAlternateScreen)?;
    enable_raw_mode()?;

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    let app = Rc::new(RefCell::new(App::new()));

    loop {
        let app_clone = Rc::clone(&app);
        terminal.draw(|frame| {
            app_clone.borrow_mut().render(frame);
        })?;

        if event::poll(std::time::Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                let should_quit = app.borrow_mut().handle_key(key);
                if should_quit {
                    execute!(stdout(), LeaveAlternateScreen)?;
                    disable_raw_mode()?;
                    return Ok(());
                }
            }
        }
    }
}
