use tui_pantry::{run, Ingredient, PropInfo};
use ratatui::{
    layout::Rect,
    buffer::Buffer,
    style::{Style, Color},
    widgets::{Widget, Block, Borders, Paragraph, Clear},
    text::{Line, Span},
};

/// TopBar widget as a Ratatui Ingredient
pub struct TopBarIngredient {
    branch: String,
    path: String,
}

impl TopBarIngredient {
    pub fn new() -> Self {
        Self {
            branch: "main".to_string(),
            path: "src/components".to_string(),
        }
    }
}

impl Ingredient for TopBarIngredient {
    fn group(&self) -> &str {
        "TopBar"
    }

    fn name(&self) -> &str {
        "Default"
    }

    fn source(&self) -> &str {
        "tidy_tui::components::top_bar::TopBar"
    }

    fn render(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::NONE)
            .style(Style::default().bg(Color::Rgb(26, 26, 26)));

        block.render(area, buf);

        // Branch/path text
        let branch_span = Span::styled(
            &self.branch,
            Style::default().fg(Color::Rgb(128, 128, 128))
        );
        let path_text = format!(" {}", self.path);
        let path_span = Span::styled(
            &path_text,
            Style::default().fg(Color::Rgb(128, 128, 128)).add_modifier(ratatui::style::Modifier::DIM)
        );

        let line = Line::from(vec![branch_span, path_span]);
        Paragraph::new(line).render(
            Rect::new(area.x + 1, area.y, area.width - 2, 1),
            buf
        );

        // Stats on right
        let stats = Span::styled(
            "4 ✓  4.56%",
            Style::default().fg(Color::Rgb(128, 128, 128))
        );
        let stats_line = Line::from(stats);
        let stats_width = 12u16;
        Paragraph::new(stats_line).render(
            Rect::new(area.x + area.width - stats_width - 1, area.y, stats_width, 1),
            buf
        );
    }

    fn props(&self) -> &[PropInfo] {
        &[
            PropInfo {
                name: "branch",
                ty: "String",
                description: "Git branch name displayed on the left",
            },
            PropInfo {
                name: "path",
                ty: "String",
                description: "Current directory path",
            },
            PropInfo {
                name: "checks_passed",
                ty: "Option<usize>",
                description: "Number of checks passed",
            },
            PropInfo {
                name: "percentage",
                ty: "Option<f32>",
                description: "Completion percentage",
            },
        ]
    }
}

/// MessageList widget as a Ratatui Ingredient
pub struct MessageListIngredient;

impl Ingredient for MessageListIngredient {
    fn group(&self) -> &str {
        "MessageList"
    }

    fn name(&self) -> &str {
        "With Messages"
    }

    fn source(&self) -> &str {
        "tidy_tui::components::message_list::MessageList"
    }

    fn render(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Messages")
            .style(Style::default().bg(Color::Rgb(26, 26, 26)));

        let inner = block.inner(area);
        block.render(area, buf);

        // User message
        let user_line = Line::from(vec![
            Span::styled("❯ ", Style::default().fg(Color::White).add_modifier(ratatui::style::Modifier::BOLD)),
            Span::styled("Edit the copy on this page", Style::default().fg(Color::Rgb(224, 224, 224))),
        ]);
        Paragraph::new(user_line).render(
            Rect::new(inner.x, inner.y, inner.width, 1),
            buf
        );

        // Thought
        let thought_line = Line::from(vec![
            Span::styled("◆ ", Style::default().fg(Color::Rgb(128, 128, 128)).add_modifier(ratatui::style::Modifier::DIM)),
            Span::styled("Thought for 2.5s", Style::default().fg(Color::Rgb(128, 128, 128))),
        ]);
        Paragraph::new(thought_line).render(
            Rect::new(inner.x, inner.y + 2, inner.width, 1),
            buf
        );

        // Edit action
        let edit_line = Line::from(vec![
            Span::styled("◆ Edit ", Style::default().fg(Color::Rgb(128, 128, 128))),
            Span::styled("frontend/apps/website/src/app/(main)/cli/page.tsx", Style::default().fg(Color::Rgb(247, 140, 108))),
        ]);
        Paragraph::new(edit_line).render(
            Rect::new(inner.x, inner.y + 4, inner.width, 1),
            buf
        );
    }
}

/// InputBar widget as a Ratatui Ingredient
pub struct InputBarIngredient;

impl Ingredient for InputBarIngredient {
    fn group(&self) -> &str {
        "InputBar"
    }

    fn name(&self) -> &str {
        "Default"
    }

    fn source(&self) -> &str {
        "tidy_tui::components::input_bar::InputBar"
    }

    fn render(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::Rgb(30, 30, 30)));

        let inner = block.inner(area);
        block.render(area, buf);

        // Prompt
        let prompt = Span::styled(
            "❯ ",
            Style::default().fg(Color::White).add_modifier(ratatui::style::Modifier::BOLD)
        );
        let input = Span::styled(
            "/btw",
            Style::default().fg(Color::Rgb(224, 224, 224))
        );
        let line = Line::from(vec![prompt, input]);
        Paragraph::new(line).render(
            Rect::new(inner.x, inner.y, inner.width, 1),
            buf
        );

        // Right info
        let info = Span::styled(
            "grok-build-latest · always-approve",
            Style::default().fg(Color::Rgb(128, 128, 128)).add_modifier(ratatui::style::Modifier::DIM)
        );
        let info_line = Line::from(info);
        Paragraph::new(info_line).render(
            Rect::new(inner.x + inner.width - 40, inner.y, 40, 1),
            buf
        );
    }
}

/// Overlay widget as a Ratatui Ingredient
pub struct OverlayIngredient;

impl Ingredient for OverlayIngredient {
    fn group(&self) -> &str {
        "Overlay"
    }

    fn name(&self) -> &str {
        "Skills"
    }

    fn source(&self) -> &str {
        "tidy_tui::components::overlay::Overlay"
    }

    fn render(&self, area: Rect, buf: &mut Buffer) {
        // Clear background
        Clear.render(area, buf);

        let block = Block::default()
            .borders(Borders::ALL)
            .title("Skills")
            .title_style(Style::default().fg(Color::Rgb(255, 203, 107)))
            .border_style(Style::default().fg(Color::Rgb(64, 64, 64)))
            .style(Style::default().bg(Color::Rgb(37, 37, 37)));

        let inner = block.inner(area);
        block.render(area, buf);

        // Tabs
        let tabs = Line::from(vec![
            Span::styled("Hooks   ", Style::default().fg(Color::Rgb(128, 128, 128))),
            Span::styled("Plugins   ", Style::default().fg(Color::Rgb(128, 128, 128))),
            Span::styled("Marketplace   ", Style::default().fg(Color::Rgb(128, 128, 128))),
            Span::styled("Skills   ", Style::default().fg(Color::White).add_modifier(ratatui::style::Modifier::BOLD)),
            Span::styled("MCP Servers   ", Style::default().fg(Color::Rgb(128, 128, 128))),
            Span::styled("[×]", Style::default().fg(Color::Rgb(128, 128, 128))),
        ]);
        Paragraph::new(tabs).render(
            Rect::new(inner.x, inner.y, inner.width, 1),
            buf
        );

        // Skills list
        let skills = vec![
            "▸ rust-check    (local)",
            "▸ gcloud-auth   (local)",
            "▸ pr-babysit    (local)",
            "▸ code-review   (local)",
            "▸ help          (user)",
        ];

        for (i, skill) in skills.iter().enumerate() {
            let line = Line::from(Span::styled(
                *skill,
                Style::default().fg(Color::Rgb(224, 224, 224))
            ));
            Paragraph::new(line).render(
                Rect::new(inner.x + 2, inner.y + 3 + i as u16, inner.width - 4, 1),
                buf
            );
        }
    }
}

/// StatusBar widget as a Ratatui Ingredient
pub struct StatusBarIngredient;

impl Ingredient for StatusBarIngredient {
    fn group(&self) -> &str {
        "StatusBar"
    }

    fn name(&self) -> &str {
        "Chat Mode"
    }

    fn source(&self) -> &str {
        "tidy_tui::components::status_bar::StatusBar"
    }

    fn render(&self, area: Rect, buf: &mut Buffer) {
        let items = vec![
            ("Enter", "send"),
            ("Shift-Tab", "normal"),
            ("^h", "home"),
            ("^q", "quit"),
        ];

        let mut x = area.x + 1;
        for (i, (key, desc)) in items.iter().enumerate() {
            if i > 0 {
                let sep = Span::styled(" | ", Style::default().fg(Color::Rgb(128, 128, 128)));
                Paragraph::new(Line::from(sep)).render(
                    Rect::new(x, area.y, 3, 1),
                    buf
                );
                x += 3;
            }

            let key_span = Span::styled(*key, Style::default().fg(Color::Rgb(128, 128, 128)));
            let desc_span = Span::styled(
                format!(" {}", desc),
                Style::default().fg(Color::Rgb(128, 128, 128)).add_modifier(ratatui::style::Modifier::DIM)
            );
            let line = Line::from(vec![key_span, desc_span]);
            let width = (key.len() + 1 + desc.len()) as u16;
            Paragraph::new(line).render(
                Rect::new(x, area.y, width, 1),
                buf
            );
            x += width;
        }
    }
}

// Ingredient module - exports all ingredients for tui-pantry discovery
pub mod ingredient {
    use super::*;

    pub fn ingredients() -> Vec<Box<dyn Ingredient>> {
        vec![
            Box::new(TopBarIngredient::new()),
            Box::new(MessageListIngredient),
            Box::new(InputBarIngredient),
            Box::new(OverlayIngredient),
            Box::new(StatusBarIngredient),
        ]
    }
}

fn main() {
    let ingredients = ingredient::ingredients();
    run!(ingredients).unwrap();
}
