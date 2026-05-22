use ratatui::{
    backend::TestBackend,
    layout::Rect,
    Terminal,
    widgets::{Widget, Block, Borders, Paragraph},
    style::{Style, Color},
    text::{Line, Span},
};

fn main() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    
    terminal.draw(|frame| {
        let area = frame.area();
        
        // Background
        let block = Block::default()
            .style(Style::default().bg(Color::Rgb(26, 26, 26)));
        frame.render_widget(block, area);
        
        // Top bar
        let top_bar = Block::default()
            .borders(Borders::NONE)
            .style(Style::default().bg(Color::Rgb(37, 37, 37)));
        frame.render_widget(top_bar, Rect::new(0, 0, 80, 1));
        
        let branch = Line::from(vec![
            Span::styled("main", Style::default().fg(Color::Rgb(128, 128, 128))),
            Span::styled(" src/components", Style::default().fg(Color::Rgb(128, 128, 128)).add_modifier(ratatui::style::Modifier::DIM)),
        ]);
        frame.render_widget(Paragraph::new(branch), Rect::new(1, 0, 40, 1));
        
        let stats = Line::from(vec![
            Span::styled("4 ✓  4.56%", Style::default().fg(Color::Rgb(128, 128, 128))),
        ]);
        frame.render_widget(Paragraph::new(stats), Rect::new(65, 0, 14, 1));
        
        // User message
        let user_msg = Line::from(vec![
            Span::styled("❯ ", Style::default().fg(Color::White).add_modifier(ratatui::style::Modifier::BOLD)),
            Span::styled("Edit the copy on this page", Style::default().fg(Color::Rgb(224, 224, 224))),
        ]);
        frame.render_widget(Paragraph::new(user_msg), Rect::new(2, 2, 76, 1));
        
        // Thought
        let thought = Line::from(vec![
            Span::styled("◆ ", Style::default().fg(Color::Rgb(128, 128, 128)).add_modifier(ratatui::style::Modifier::DIM)),
            Span::styled("Thought for 2.5s", Style::default().fg(Color::Rgb(128, 128, 128))),
        ]);
        frame.render_widget(Paragraph::new(thought), Rect::new(2, 4, 76, 1));
        
        // Edit action
        let edit = Line::from(vec![
            Span::styled("◆ Edit ", Style::default().fg(Color::Rgb(128, 128, 128))),
            Span::styled("frontend/apps/website/src/app/(main)/cli/page.tsx", Style::default().fg(Color::Rgb(247, 140, 108))),
        ]);
        frame.render_widget(Paragraph::new(edit), Rect::new(2, 6, 76, 1));
        
        // Code block
        let code_bg = Block::default()
            .style(Style::default().bg(Color::Rgb(30, 30, 30)));
        frame.render_widget(code_bg, Rect::new(4, 8, 72, 6));
        
        let code_line = Line::from(vec![
            Span::styled("  770  ", Style::default().fg(Color::Rgb(128, 128, 128)).add_modifier(ratatui::style::Modifier::DIM)),
            Span::styled("<h3 className=\"", Style::default().fg(Color::Rgb(92, 207, 230))),
            Span::styled("text-balance text-3xl font-semibold tracking-tight", Style::default().fg(Color::Rgb(126, 231, 135))),
            Span::styled("\">", Style::default().fg(Color::Rgb(92, 207, 230))),
        ]);
        frame.render_widget(Paragraph::new(code_line), Rect::new(4, 8, 72, 1));
        
        let code_line2 = Line::from(vec![
            Span::styled("  771  ", Style::default().fg(Color::Rgb(128, 128, 128)).add_modifier(ratatui::style::Modifier::DIM)),
            Span::styled("    Better title", Style::default().fg(Color::Rgb(224, 224, 224))),
        ]);
        frame.render_widget(Paragraph::new(code_line2), Rect::new(4, 9, 72, 1));
        
        let code_added = Line::from(vec![
            Span::styled("  773  ", Style::default().fg(Color::Rgb(128, 128, 128)).add_modifier(ratatui::style::Modifier::DIM)),
            Span::styled("    <p className=\"text-secondary mx-auto mt-4\">", Style::default().fg(Color::Rgb(126, 231, 135)).bg(Color::Rgb(30, 50, 30))),
        ]);
        frame.render_widget(Paragraph::new(code_added), Rect::new(4, 10, 72, 1));
        
        let code_added2 = Line::from(vec![
            Span::styled("  774  ", Style::default().fg(Color::Rgb(128, 128, 128)).add_modifier(ratatui::style::Modifier::DIM)),
            Span::styled("      A terminal-native experience...", Style::default().fg(Color::Rgb(126, 231, 135)).bg(Color::Rgb(30, 50, 30))),
        ]);
        frame.render_widget(Paragraph::new(code_added2), Rect::new(4, 11, 72, 1));
        
        // Input bar
        let input_bg = Block::default()
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::Rgb(30, 30, 30)));
        frame.render_widget(input_bg, Rect::new(0, 19, 80, 3));
        
        let input = Line::from(vec![
            Span::styled("❯ ", Style::default().fg(Color::White).add_modifier(ratatui::style::Modifier::BOLD)),
            Span::styled("/btw", Style::default().fg(Color::Rgb(224, 224, 224))),
        ]);
        frame.render_widget(Paragraph::new(input), Rect::new(2, 20, 40, 1));
        
        let info = Line::from(vec![
            Span::styled("grok-build-latest · always-approve", Style::default().fg(Color::Rgb(128, 128, 128)).add_modifier(ratatui::style::Modifier::DIM)),
        ]);
        frame.render_widget(Paragraph::new(info), Rect::new(45, 20, 32, 1));
        
        // Status bar
        let status = Line::from(vec![
            Span::styled("Enter", Style::default().fg(Color::Rgb(128, 128, 128))),
            Span::styled(" send  |  ", Style::default().fg(Color::Rgb(128, 128, 128)).add_modifier(ratatui::style::Modifier::DIM)),
            Span::styled("Shift-Tab", Style::default().fg(Color::Rgb(128, 128, 128))),
            Span::styled(" normal  |  ", Style::default().fg(Color::Rgb(128, 128, 128)).add_modifier(ratatui::style::Modifier::DIM)),
            Span::styled("^q", Style::default().fg(Color::Rgb(128, 128, 128))),
            Span::styled(" quit", Style::default().fg(Color::Rgb(128, 128, 128)).add_modifier(ratatui::style::Modifier::DIM)),
        ]);
        frame.render_widget(Paragraph::new(status), Rect::new(1, 23, 78, 1));
    }).unwrap();
    
    // Get the buffer and print it
    let buffer = terminal.backend().buffer().clone();
    
    // Print to stdout
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            let cell = buffer.get(x, y);
            print!("{}", cell.symbol());
        }
        println!();
    }
}
