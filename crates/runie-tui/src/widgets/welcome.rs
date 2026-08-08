//! Deterministic full-mode welcome surface.

use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line as RatLine, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

use crate::appearance;
use crate::widgets::scrollback::{Line, LineKind};
use runie_core::types::ThemeKind;

/// The full-mode welcome surface shown before the first prompt is submitted.
#[derive(Debug, Clone, Copy, Default)]
pub struct WelcomeWidget;

pub use runie_tui_model::{version_badge, VersionBadgeVariant};

impl Widget for WelcomeWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.render_with_theme(area, buf, ThemeKind::GrokNight);
    }
}

impl WelcomeWidget {
    pub fn render_with_theme(self, area: Rect, buf: &mut Buffer, theme: ThemeKind) {
        if area.width < 24 || area.height < 8 {
            return;
        }
        if area.height < 16 {
            self.render_compact(area, buf, theme);
            return;
        }
        if area.width >= 100 && area.height >= 22 {
            self.render_wide_hero(area, buf);
            return;
        }
        self.render_full(area, buf);
    }
    pub fn render_hero_footer_badge(area: Rect, buf: &mut Buffer) {
        if area.height == 0 || area.width == 0 {
            return;
        }
        Paragraph::new(version_badge(VersionBadgeVariant::HeroFooter))
            .alignment(Alignment::Right)
            .render(area, buf);
    }

    fn render_compact(self, area: Rect, buf: &mut Buffer, theme: ThemeKind) {
        let surface = Rect {
            x: area.x
                + area
                    .width
                    .saturating_sub(42.min(area.width.saturating_sub(4)))
                    / 2,
            y: area.y + area.height.saturating_sub(10) / 2,
            width: 42.min(area.width.saturating_sub(4)),
            height: 10,
        };
        let lines = vec![
            RatLine::from(Span::styled(
                "Runie",
                appearance::accent_style_for(theme).add_modifier(Modifier::BOLD),
            )),
            RatLine::from(version_badge(VersionBadgeVariant::HeroInline)),
            RatLine::from("Model · runie-core"),
            RatLine::from(""),
            RatLine::from("New session"),
            RatLine::from("/help for commands"),
            RatLine::from("Ctrl+D / Ctrl+Q · quit"),
            RatLine::from("◆ session_start"),
        ];
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title(" welcome "))
            .render(surface, buf);
    }

    fn render_wide_hero(self, area: Rect, buf: &mut Buffer) {
        let box_area = Rect::new(
            area.x.saturating_add(2),
            area.y.saturating_add(8),
            area.width.saturating_sub(2),
            13.min(area.height.saturating_sub(8)),
        );
        Block::default().borders(Borders::ALL).render(box_area, buf);
        render_wide_hero_copy(box_area, buf);
        render_wide_hero_actions(box_area, buf);
        Paragraph::new(version_badge(VersionBadgeVariant::Full))
            .alignment(Alignment::Right)
            .render(
                Rect::new(
                    area.x,
                    area.y + area.height.saturating_sub(1),
                    area.width,
                    1,
                ),
                buf,
            );
    }

    fn render_full(self, area: Rect, buf: &mut Buffer) {
        let bold = Style::default().add_modifier(Modifier::BOLD);
        let x = area.x.saturating_add(13);
        let action = |label: &str, shortcut: &str| {
            RatLine::from(vec![
                Span::styled(label.to_owned(), bold),
                Span::raw(" ".repeat(45usize.saturating_sub(label.len()))),
                Span::raw(shortcut.to_owned()),
            ])
        };
        Paragraph::new(vec![
            action("New worktree", "ctrl+w"),
            action("Resume session", "ctrl+s"),
            RatLine::from(Span::styled("Changelog", bold)),
            // Match Grok's full-mode welcome copy; Ctrl+D remains supported
            // by the input handler but is intentionally not duplicated here.
            action("Quit", "ctrl+q"),
        ])
        .render(
            Rect::new(x, area.y + 3, area.width.saturating_sub(13), 4),
            buf,
        );
        Paragraph::new(vec![
            RatLine::from(Span::styled("Workflows are here!", bold)),
            RatLine::from("Try them out using /workflows."),
        ])
        .render(
            Rect::new(x, area.y + 8, area.width.saturating_sub(13), 2),
            buf,
        );
        Paragraph::new(RatLine::from(vec![
            Span::styled("Tip: ", bold),
            Span::raw("Use Ctrl+Enter to interject messages. Or just Enter to queue messages."),
        ]))
        .render(Rect::new(area.x, area.y + 14, area.width, 1), buf);
        if area.width >= 100 {
            Paragraph::new(version_badge(VersionBadgeVariant::Full))
                .alignment(Alignment::Right)
                .render(Rect::new(area.x, area.y + 1, area.width, 1), buf);
        }
    }
}

fn render_wide_hero_copy(area: Rect, buf: &mut Buffer) {
    let logo = [
        "⠀⠀⠀⠀⠀⠀⣀⣀⡀⠀⠀⠀⢀⠄",
        "⠀⠀⠀⣠⣾⠿⠛⠛⠛⠛⢀⡴⠁⠀",
        "⠀⠀⣼⡟⠁⠀⠀⠀⢀⡴⠻⣿⡀⠀",
        "⠀⠀⣿⡇⠀⠀⠀⠔⠁⠀⠀⣿⡇⠀",
        "⠀⠀⢹⣷⠀⠀⠀⠀⠀⢀⣴⡿⠀⠀",
        "⠀⢀⠞⠁⠠⢶⣶⣶⣶⠿⠋⠀⠀⠀",
        "⠐⠁⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀",
    ];
    Paragraph::new(logo.into_iter().map(RatLine::from).collect::<Vec<_>>())
        .render(Rect::new(area.x + 3, area.y + 2, 32, 7), buf);
    Paragraph::new(vec![
        RatLine::from(Span::styled(
            "Grok Build Beta  0.2.118",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        RatLine::from(""),
        RatLine::from(Span::styled(
            "Grok 4.5 is here!",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        RatLine::from("Select 'Grok 4.5' under /model."),
    ])
    .render(Rect::new(area.x + 20, area.y + 2, 46, 4), buf);
}

fn render_wide_hero_actions(area: Rect, buf: &mut Buffer) {
    for (offset, (label, shortcut)) in [
        ("New worktree", "ctrl+w"),
        ("Resume session", "ctrl+s"),
        ("Changelog", ""),
        ("Quit", "ctrl+q"),
    ]
    .into_iter()
    .enumerate()
    {
        let y = area.y + 7 + offset as u16;
        let text = format!("{label:>85}{shortcut:>10}");
        Paragraph::new(RatLine::from(Span::styled(
            text,
            Style::default().add_modifier(Modifier::BOLD),
        )))
        .render(
            Rect::new(area.x + 3, y, area.width.saturating_sub(6), 1),
            buf,
        );
    }
}

/// Pure function: returns the welcome-modal lines (matches grok-build's
/// minimal-mode chrome). Adopts grok's `insta::assert_snapshot!` pattern:
/// the function is a pure formatter, the test pins its output to a snapshot.
/// The widget owns its own idle prompt text so the formatter lives with the
/// surface that renders it.
pub fn welcome_modal_lines() -> Vec<Line> {
    let version = env!("CARGO_PKG_VERSION");
    vec![
        Line::new(LineKind::System, format!("╭─ Runie  v{version} ─")),
        Line::new(LineKind::System, String::from("│ main runie")),
        Line::new(LineKind::System, String::from("│ Model · runie-core")),
        Line::new(LineKind::System, String::from("│ /help for commands")),
        Line::new(LineKind::System, String::from("╰─")),
        Line::new(LineKind::System, String::from("◆ session_start")),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn welcome_surface_snapshot() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 80, 24));
        WelcomeWidget.render(Rect::new(0, 0, 80, 24), &mut buffer);
        let text = (0..24)
            .map(|y| {
                (0..80)
                    .map(|x| buffer.cell((x, y)).expect("cell").symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        insta::assert_snapshot!("welcome-full-mode", text);
    }

    #[test]
    fn compact_welcome_shows_both_quit_chords() {
        let area = Rect::new(0, 0, 40, 12);
        let mut buffer = Buffer::empty(area);
        WelcomeWidget.render(area, &mut buffer);
        let text = (0..area.height)
            .flat_map(|y| (0..area.width).map(move |x| (x, y)))
            .filter_map(|(x, y)| buffer.cell((x, y)))
            .map(|cell| cell.symbol().to_string())
            .collect::<String>();
        assert!(text.contains("Ctrl+D / Ctrl+Q"));
    }

    #[test]
    fn compact_welcome_projects_the_selected_theme_accent() {
        let area = Rect::new(0, 0, 40, 12);
        let mut buffer = Buffer::empty(area);
        WelcomeWidget.render_with_theme(area, &mut buffer, ThemeKind::GrokDay);
        let runie = (0..area.height)
            .flat_map(|y| (0..area.width).map(move |x| (x, y)))
            .find_map(|(x, y)| {
                buffer
                    .cell((x, y))
                    .filter(|cell| cell.symbol() == "R")
                    .map(|cell| cell.fg)
            })
            .expect("Runie welcome title");
        assert_eq!(
            runie,
            appearance::accent_style_for(ThemeKind::GrokDay)
                .fg
                .expect("day accent color")
        );
    }

    #[test]
    fn full_welcome_uses_grok_quit_chord() {
        let area = Rect::new(0, 0, 80, 24);
        let mut buffer = Buffer::empty(area);
        WelcomeWidget.render(area, &mut buffer);
        let text = (0..area.height)
            .flat_map(|y| (0..area.width).map(move |x| (x, y)))
            .filter_map(|(x, y)| buffer.cell((x, y)))
            .map(|cell| cell.symbol().to_string())
            .collect::<String>();
        assert!(text.contains("ctrl+q"));
    }

    #[test]
    fn wide_full_welcome_renders_full_version_badge() {
        let area = Rect::new(0, 0, 120, 26);
        let mut buffer = Buffer::empty(area);
        WelcomeWidget.render(area, &mut buffer);
        let text = (0..area.height)
            .flat_map(|y| (0..area.width).map(move |x| (x, y)))
            .filter_map(|(x, y)| buffer.cell((x, y)))
            .map(|cell| cell.symbol().to_string())
            .collect::<String>();
        assert!(text.contains(&version_badge(VersionBadgeVariant::Full)));
    }

    #[test]
    fn hero_footer_badge_renders_in_its_dedicated_row() {
        let area = Rect::new(0, 0, 80, 1);
        let mut buffer = Buffer::empty(area);
        WelcomeWidget::render_hero_footer_badge(area, &mut buffer);
        let text = (0..area.width)
            .filter_map(|x| buffer.cell((x, 0)))
            .map(|cell| cell.symbol().to_string())
            .collect::<String>();
        assert!(text.contains(&version_badge(VersionBadgeVariant::HeroFooter)));
    }

    #[test]
    fn version_badge_variants_are_explicit_and_distinct() {
        let full = version_badge(VersionBadgeVariant::Full);
        let footer = version_badge(VersionBadgeVariant::HeroFooter);
        let inline = version_badge(VersionBadgeVariant::HeroInline);
        assert!(full.contains("Beta"));
        assert!(footer.contains("Beta"));
        assert!(inline.contains(env!("CARGO_PKG_VERSION")));
        assert_ne!(full, footer);
        assert_ne!(footer, inline);
    }

    #[test]
    fn wide_welcome_matches_recorded_grok_hero_markers() {
        let area = Rect::new(0, 0, 120, 36);
        let mut buffer = Buffer::empty(area);
        WelcomeWidget.render(area, &mut buffer);
        let text = (0..area.height)
            .flat_map(|y| (0..area.width).map(move |x| (x, y)))
            .filter_map(|(x, y)| buffer.cell((x, y)))
            .map(|cell| cell.symbol().to_string())
            .collect::<String>();
        for marker in [
            "Grok Build Beta  0.2.118",
            "Grok 4.5 is here!",
            "New worktree",
            "ctrl+q",
        ] {
            assert!(text.contains(marker), "wide hero lacks {marker:?}");
        }
    }

    /// Pure-function snapshot (adopted from grok-build's `insta` pattern).
    /// The welcome modal is a deterministic formatter; the test pins its
    /// text to a saved snapshot so accidental layout drift gets caught.
    #[test]
    fn welcome_modal_snapshot() {
        let text: String = super::welcome_modal_lines()
            .iter()
            .map(|l| l.text.clone())
            .collect::<Vec<_>>()
            .join("\n");
        insta::assert_snapshot!("welcome_modal", text);
    }
}
