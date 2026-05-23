//! Host binary - loads and runs compiled TSX apps.

use protocol::App;
use renderer::{Renderer, RatatuiRenderer, VNode, VElement, VText, VProps, VStyle, FlexDirection};
use ratatui::{Terminal, backend::CrosstermBackend, Frame};
use std::cell::RefCell;
use std::io::stdout;
use std::rc::Rc;
use crossterm::{event::{self, Event, KeyEvent}, execute};
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    execute!(stdout(), crossterm::terminal::EnterAlternateScreen)?;
    crossterm::terminal::enable_raw_mode()?;

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    // Create renderer and app state wrapped in RefCell for interior mutability
    let app = Rc::new(RefCell::new(TuiApp {
        count: 0,
        renderer: RatatuiRenderer::new(),
    }));

    // Event loop from spec
    loop {
        let app_clone = Rc::clone(&app);
        terminal.draw(|frame| {
            app_clone.borrow_mut().render(frame);
        })?;

        if event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                app.borrow_mut().handle_key(key);
            }
        }
    }
}

struct TuiApp {
    count: u32,
    renderer: RatatuiRenderer,
}

impl App for TuiApp {
    fn update(&mut self) {
        // State update logic
    }

    fn render(&mut self, frame: &mut Frame) {
        let area = frame.size();

        // Build VNode tree matching app.tsx
        let vnode = VNode::Element(VElement {
            tag: "View".to_string(),
            props: VProps {
                style: VStyle {
                    flex_direction: FlexDirection::Column,
                    padding: 2,
                    ..Default::default()
                },
                ..Default::default()
            },
            children: vec![
                VNode::Element(VElement {
                    tag: "Text".to_string(),
                    props: VProps {
                        style: VStyle {
                            color: Some("#FF5733".to_string()),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    children: vec![VNode::Text(VText {
                        content: format!("Count: {}", self.count),
                        style: VStyle::default(),
                    })],
                }),
                VNode::Element(VElement {
                    tag: "Button".to_string(),
                    props: VProps {
                        style: VStyle::default(),
                        on_press: Some(Box::new(|| {})),
                        ..Default::default()
                    },
                    children: vec![VNode::Text(VText {
                        content: "Increment".to_string(),
                        style: VStyle::default(),
                    })],
                }),
            ],
        });

        // Mount and render
        self.renderer.mount(vnode, area);
        self.renderer.render(frame.buffer_mut());
    }

    fn handle_key(&mut self, key: KeyEvent) {
        match key.code {
            crossterm::event::KeyCode::Enter => {
                self.count += 1;
            }
            crossterm::event::KeyCode::Char('q') => {
                // Quit
                let _ = execute!(stdout(), crossterm::terminal::LeaveAlternateScreen);
                crossterm::terminal::disable_raw_mode().ok();
                std::process::exit(0);
            }
            _ => {}
        }
    }
}
