//! Freeze reproduction test

#[test]
fn test_channel_based_rendering() {
    use std::{
        sync::mpsc::channel,
        thread,
        time::{Duration, Instant},
    };
    use ratatui::{backend::TestBackend, Terminal, widgets::Paragraph};
    use runie_core::AppState;

    let (render_tx, render_rx) = channel::<AppState>();

    // UI thread
    let ui_handle = thread::spawn(move || {
        let backend = TestBackend::new(80, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut frames = 0u64;
        let start = Instant::now();

        while let Ok(state) = render_rx.recv() {
            let _ = terminal.draw(|f| {
                let lines: Vec<_> = state.messages.iter()
                    .skip(state.messages.len().saturating_sub(20))
                    .map(|m| ratatui::text::Line::from(m.content.as_str()))
                    .collect();
                f.render_widget(Paragraph::new(lines), f.area());
            });
            frames += 1;

            if start.elapsed() > Duration::from_secs(5) {
                break;
            }
        }

        eprintln!("[UI] {} frames", frames);
        frames
    });

    // Rapid state updates
    let mut state = AppState::default();
    let start = Instant::now();

    for i in 0..500 {
        thread::sleep(Duration::from_millis(10));
        state.messages.push(runie_core::ChatMessage {
            role: "user".to_string(),
            content: format!("Line {} with some content here to make it realistic", i),
            timestamp: 0.0,
            id: format!("req.{}", i),
        });

        // Send snapshot every 50ms
        if i % 5 == 0 {
            let _ = render_tx.send(state.clone());
        }
    }

    drop(render_tx);
    let frames = ui_handle.join().unwrap();
    let elapsed = start.elapsed();

    eprintln!("Test: {} frames in {:?}", frames, elapsed);
    assert!(frames >= 50, "UI only rendered {} frames", frames);
}
