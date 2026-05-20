use ratatui::{
    widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    style::{Style, Color, Modifier},
    text::{Line, Span},
};

#[derive(Clone)]
pub enum EntryType {
    Thought,
    Edit,
    Plan,
    Question,
}

pub struct StreamEntry {
    pub entry_type: EntryType,
    pub file: Option<String>,
    pub content: Vec<String>,
    pub elapsed_secs: Option<f64>,
    pub expanded: bool,
}

pub struct Stream {
    pub entries: Vec<StreamEntry>,
    pub selected: usize,
    pub auto_scroll: bool,
    pub scroll_offset: usize,
}

impl Stream {
    pub fn new() -> Self {
        let entries = Self::generate_mock_entries(50);
        Self {
            entries,
            selected: 0,
            auto_scroll: true,
            scroll_offset: 0,
        }
    }

    fn entry_line_count(entry: &StreamEntry, idx: usize) -> usize {
        match &entry.entry_type {
            EntryType::Thought => 2,
            EntryType::Edit => 1 + entry.content.len(),
            EntryType::Plan => {
                if entry.expanded || idx < 10 { 2 + entry.content.len().saturating_sub(1) } else { 1 }
            }
            EntryType::Question => {
                if entry.expanded || idx < 5 { 4 } else { 1 }
            }
        }
    }

    /// Total rendered line count (for scrollbar max)
    pub fn line_count(&self) -> usize {
        self.entries.iter()
            .enumerate()
            .map(|(i, e)| Self::entry_line_count(e, i))
            .sum()
    }

    fn ensure_selection_visible(&mut self, viewport_height: usize) {
        let vis_h = viewport_height.saturating_sub(2);
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        } else if self.selected >= self.scroll_offset + vis_h {
            self.scroll_offset = self.selected.saturating_sub(vis_h - 1);
        }
    }

    pub fn render(&self, visible_height: u16) -> (Paragraph<'_>, Scrollbar<'_>, ScrollbarState) {
        let visible_h = visible_height as usize;
        let mut lines: Vec<Line> = Vec::new();
        let mut visible_line_count = 0;
        let mut skipped = 0;

        for (i, entry) in self.entries.iter().enumerate() {
            let entry_h = Self::entry_line_count(entry, i);

            if skipped < self.scroll_offset {
                skipped += entry_h;
                continue;
            }
            if visible_line_count + entry_h > visible_h {
                break;
            }

            let selected = i == self.selected;
            let prefix = if selected { "▶" } else { " " };

            let (marker, color) = match &entry.entry_type {
                EntryType::Thought => ("◇", Color::DarkGray),
                EntryType::Edit => ("◆", Color::Yellow),
                EntryType::Plan => ("┌", Color::Cyan),
                EntryType::Question => ("┌", Color::Magenta),
            };

            let elapsed = entry.elapsed_secs
                .map(|s| format!(" [{:.1}s]", s))
                .unwrap_or_default();

            match &entry.entry_type {
                EntryType::Thought => {
                    lines.push(Line::from(vec![
                        Span::raw(format!("{} {} ", prefix, marker)),
                        Span::styled("Thought", Style::new().fg(color).add_modifier(Modifier::BOLD)),
                        Span::raw(elapsed),
                    ]));
                    lines.push(Line::from(format!("  {} {}", prefix, entry.content[0])));
                }
                EntryType::Edit => {
                    lines.push(Line::from(vec![
                        Span::raw(format!("{} {} ", prefix, marker)),
                        Span::styled("Edit ", Style::new().fg(color).add_modifier(Modifier::BOLD)),
                        Span::raw(entry.file.as_deref().unwrap_or("")),
                    ]));
                    for line in &entry.content {
                        let style = if line.trim().starts_with('+') {
                            Style::new().fg(Color::Green)
                        } else if line.trim().starts_with('-') {
                            Style::new().fg(Color::Red)
                        } else {
                            Style::new().fg(Color::White)
                        };
                        lines.push(Line::from(Span::styled(format!("  {} {}", prefix, line), style)));
                    }
                }
                EntryType::Plan => {
                    let title = entry.content.first().map(|s| s.as_str()).unwrap_or("Plan");
                    lines.push(Line::from(vec![
                        Span::raw(format!("{} ┌─ ", prefix)),
                        Span::styled("Plan: ", Style::new().fg(color).add_modifier(Modifier::BOLD)),
                        Span::raw(title),
                        Span::raw(" ─────────────────────────────"),
                    ]));
                    if entry.expanded || i < 10 {
                        for line in &entry.content[1..] {
                            lines.push(Line::from(vec![
                                Span::raw(format!("{} │  ", prefix)),
                                Span::raw(line),
                            ]));
                        }
                        lines.push(Line::from(format!("{} └───────────────────────────────────────────", prefix)));
                    }
                }
                EntryType::Question => {
                    lines.push(Line::from(vec![
                        Span::raw(format!("{} ┌─ ", prefix)),
                        Span::styled("Question", Style::new().fg(color).add_modifier(Modifier::BOLD)),
                        Span::raw(" ─────────────────────────────"),
                    ]));
                    if entry.expanded || i < 5 {
                        lines.push(Line::from(vec![
                            Span::raw(format!("{} │  ", prefix)),
                            Span::raw(&entry.content[0]),
                        ]));
                        lines.push(Line::from(vec![
                            Span::raw(format!("{} │  [1/3] ↑/↓ navigate · Enter: select", prefix)),
                        ]));
                        lines.push(Line::from(format!("{} └───────────────────────────────────────────", prefix)));
                    }
                }
            }
            visible_line_count += entry_h;
        }

        let para = Paragraph::new(lines).style(Style::new().fg(Color::White));
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .style(Style::new().fg(Color::DarkGray));
        let sb_state = ScrollbarState::new(self.line_count()).position(self.scroll_offset);
        (para, scrollbar, sb_state)
    }

    pub fn handle_key(&mut self, key: crossterm::event::KeyCode, viewport_height: usize) {
        let vis_h = viewport_height.saturating_sub(4);
        match key {
            crossterm::event::KeyCode::Char('j') | crossterm::event::KeyCode::Down => {
                if self.selected < self.entries.len().saturating_sub(1) {
                    self.selected += 1;
                    self.ensure_selection_visible(vis_h);
                }
            }
            crossterm::event::KeyCode::Char('k') | crossterm::event::KeyCode::Up => {
                if self.selected > 0 {
                    self.selected -= 1;
                    self.ensure_selection_visible(vis_h);
                }
            }
            crossterm::event::KeyCode::Char(' ') => {
                if self.selected < self.entries.len() {
                    self.entries[self.selected].expanded = !self.entries[self.selected].expanded;
                }
            }
            crossterm::event::KeyCode::PageDown => {
                self.selected = (self.selected + vis_h).min(self.entries.len().saturating_sub(1));
                self.ensure_selection_visible(vis_h);
            }
            crossterm::event::KeyCode::PageUp => {
                self.selected = self.selected.saturating_sub(vis_h);
                self.ensure_selection_visible(vis_h);
            }
            _ => {}
        }
    }

    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Push a new thought entry from user input
    pub fn push_input_entry(&mut self, text: &str) {
        self.entries.push(StreamEntry {
            entry_type: EntryType::Thought,
            file: None,
            content: vec![text.to_string()],
            elapsed_secs: Some(0.0),
            expanded: true,
        });
        self.selected = self.entries.len().saturating_sub(1);
        // Auto-scroll: set offset so last entry is visible
        if self.entries.len() > 10 {
            self.scroll_offset = self.entries.len().saturating_sub(5);
        }
    }

    pub fn scroll_to_bottom(&mut self, viewport_height: usize) {
        self.scroll_offset = self.line_count().saturating_sub(viewport_height as usize);
        self.selected = self.entries.len().saturating_sub(1);
    }

    fn generate_mock_entries(count: usize) -> Vec<StreamEntry> {
        let thoughts = [
            "Analyzing current codebase structure for refactoring opportunities",
            "Checking dependencies and version compatibility",
            "Reviewing existing test coverage",
            "Identifying potential performance bottlenecks",
            "Planning incremental migration strategy",
            "Comparing approaches for optimal solution",
            "Validating input parameters and edge cases",
            "Checking for existing patterns in the codebase",
            "Evaluating security implications",
            "Reviewing documentation requirements",
        ];
        let files = [
            "src/main.rs", "src/tui/app.rs", "src/tui/stream.rs",
            "src/core/dag.rs", "src/router/ooda.rs", "tests/integration.rs",
            "Cargo.toml", "README.md",
        ];
        let plans = [
            "Refactor auth module to use OAuth2",
            "Optimize database queries for scaling",
            "Add comprehensive test coverage",
            "Migrate to new API version",
            "Implement caching layer",
            "Add observability metrics",
        ];
        let questions = [
            "Which authentication provider should we prioritize?",
            "How should we handle backwards compatibility?",
            "What's the preferred error handling strategy?",
            "Should we use sync or async for this operation?",
            "What's the memory budget for this feature?",
        ];

        (0..count).map(|i| {
            let entry_type = match i % 4 {
                0 => EntryType::Thought,
                1 => EntryType::Edit,
                2 => EntryType::Plan,
                _ => EntryType::Question,
            };
            match entry_type {
                EntryType::Thought => StreamEntry {
                    entry_type,
                    file: None,
                    content: vec![thoughts[i % thoughts.len()].to_string()],
                    elapsed_secs: Some(0.5 + (i % 10) as f64 * 0.3),
                    expanded: true,
                },
                EntryType::Edit => {
                    let line_num = 100 + i * 5;
                    StreamEntry {
                        entry_type,
                        file: Some(files[i % files.len()].to_string()),
                        content: vec![
                            format!("{}  -   <div className=\"old\">old content</div>", line_num),
                            format!("{}  +   <div className=\"new\">new content</div>", line_num + 1),
                            format!("{}  +     More detailed implementation", line_num + 2),
                            format!("{}  +       with proper formatting", line_num + 3),
                        ],
                        elapsed_secs: None,
                        expanded: true,
                    }
                }
                EntryType::Plan => StreamEntry {
                    entry_type: EntryType::Plan,
                    file: Some("plan.md".to_string()),
                    content: vec![
                        format!("{}. {}", i / 4 + 1, plans[i % plans.len()]),
                        "   • Identify scope and dependencies".to_string(),
                        "   • Implement core functionality".to_string(),
                        "   • Add tests and documentation".to_string(),
                        "   • Review and iterate".to_string(),
                        "[Pending approval]".to_string(),
                    ],
                    elapsed_secs: None,
                    expanded: i < 10,
                },
                EntryType::Question => StreamEntry {
                    entry_type,
                    file: None,
                    content: vec![questions[i % questions.len()].to_string()],
                    elapsed_secs: None,
                    expanded: i < 5,
                },
            }
        }).collect()
    }
}

impl Default for Stream {
    fn default() -> Self {
        Self::new()
    }
}
