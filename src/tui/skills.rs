//! Skills panel - skill/hook/plugin management
//! Tab cycles: Hooks | Plugins | Marketplace | Skills | MCP Servers

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    prelude::{Frame, Stylize},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

/// Tab types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillTab {
    Hooks,
    Plugins,
    Marketplace,
    Skills,
    McpServers,
}

impl SkillTab {
    pub fn next(self) -> Self {
        match self {
            SkillTab::Hooks => SkillTab::Plugins,
            SkillTab::Plugins => SkillTab::Marketplace,
            SkillTab::Marketplace => SkillTab::Skills,
            SkillTab::Skills => SkillTab::McpServers,
            SkillTab::McpServers => SkillTab::Hooks,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            SkillTab::Hooks => "Hooks",
            SkillTab::Plugins => "Plugins",
            SkillTab::Marketplace => "Marketplace",
            SkillTab::Skills => "Skills",
            SkillTab::McpServers => "MCP Servers",
        }
    }
}

/// A skill/hook/plugin entry
#[derive(Debug, Clone)]
pub struct SkillEntry {
    pub name: String,
    pub source: SkillSource,
    pub description: String,
    pub enabled: bool,
    pub trigger: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillSource {
    Local,
    User,
    Plugin,
}

impl SkillEntry {
    fn source_label(&self) -> &'static str {
        match self.source {
            SkillSource::Local => "local",
            SkillSource::User => "user",
            SkillSource::Plugin => "plugin",
        }
    }
}

/// Skills panel with tabs
pub struct SkillsPanel {
    pub visible: bool,
    pub active_tab: SkillTab,
    pub list_state: ListState,
    pub search_query: String,
    pub selected_entry: Option<SkillEntry>,
    pub expanded_entry: Option<usize>,
}

impl SkillsPanel {
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            visible: false,
            active_tab: SkillTab::Skills,
            list_state,
            search_query: String::new(),
            selected_entry: None,
            expanded_entry: None,
        }
    }

    /// Show the panel
    pub fn show(&mut self) {
        self.visible = true;
        self.list_state.select(Some(0));
    }

    /// Hide the panel
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Toggle visibility
    pub fn toggle(&mut self) {
        if self.visible {
            self.hide();
        } else {
            self.show();
        }
    }

    /// Cycle to next tab
    pub fn next_tab(&mut self) {
        self.active_tab = self.active_tab.next();
        self.list_state.select(Some(0));
        self.expanded_entry = None;
    }

    /// Get entries for current tab
    pub fn entries(&self) -> Vec<SkillEntry> {
        // Mock data based on current tab
        match self.active_tab {
            SkillTab::Hooks => vec![
                SkillEntry {
                    name: "pre-edit".to_string(),
                    source: SkillSource::Local,
                    description: "Validate file changes before edit".to_string(),
                    enabled: true,
                    trigger: Some("pre-edit".to_string()),
                },
                SkillEntry {
                    name: "post-edit".to_string(),
                    source: SkillSource::Local,
                    description: "Run after each file modification".to_string(),
                    enabled: true,
                    trigger: Some("post-edit".to_string()),
                },
                SkillEntry {
                    name: "pre-commit".to_string(),
                    source: SkillSource::User,
                    description: "Safety checks before commit".to_string(),
                    enabled: true,
                    trigger: Some("pre-commit".to_string()),
                },
                SkillEntry {
                    name: "on-cost-threshold".to_string(),
                    source: SkillSource::User,
                    description: "Downgrade model when approaching budget".to_string(),
                    enabled: false,
                    trigger: Some("on-cost-threshold".to_string()),
                },
            ],
            SkillTab::Plugins => vec![
                SkillEntry {
                    name: "eval-development".to_string(),
                    source: SkillSource::Plugin,
                    description: "Development evaluation toolkit".to_string(),
                    enabled: true,
                    trigger: None,
                },
            ],
            SkillTab::Marketplace => vec![
                SkillEntry {
                    name: "rust-expert".to_string(),
                    source: SkillSource::Local,
                    description: "Rust best practices and patterns".to_string(),
                    enabled: false,
                    trigger: None,
                },
                SkillEntry {
                    name: "react-master".to_string(),
                    source: SkillSource::Local,
                    description: "React patterns and performance".to_string(),
                    enabled: false,
                    trigger: None,
                },
            ],
            SkillTab::Skills => vec![
                SkillEntry {
                    name: "rust-check".to_string(),
                    source: SkillSource::Local,
                    description: "Run cargo check on Rust edits".to_string(),
                    enabled: true,
                    trigger: Some("*.rs".to_string()),
                },
                SkillEntry {
                    name: "gcloud-auth".to_string(),
                    source: SkillSource::Local,
                    description: "Authenticate with GCP".to_string(),
                    enabled: true,
                    trigger: None,
                },
                SkillEntry {
                    name: "pr-babysit".to_string(),
                    source: SkillSource::Local,
                    description: "Monitor PR status and feedback".to_string(),
                    enabled: true,
                    trigger: None,
                },
                SkillEntry {
                    name: "code-review".to_string(),
                    source: SkillSource::Local,
                    description: "Automated code review".to_string(),
                    enabled: true,
                    trigger: None,
                },
                SkillEntry {
                    name: "xai-grafana-mcp".to_string(),
                    source: SkillSource::Local,
                    description: "Grafana metrics integration".to_string(),
                    enabled: true,
                    trigger: None,
                },
                SkillEntry {
                    name: "help".to_string(),
                    source: SkillSource::User,
                    description: "User help and documentation".to_string(),
                    enabled: true,
                    trigger: None,
                },
                SkillEntry {
                    name: "oklch-skill".to_string(),
                    source: SkillSource::User,
                    description: "OKLCH color space utilities".to_string(),
                    enabled: true,
                    trigger: None,
                },
                SkillEntry {
                    name: "make-interfaces-feel-better".to_string(),
                    source: SkillSource::User,
                    description: "Design engineering principles".to_string(),
                    enabled: true,
                    trigger: None,
                },
            ],
            SkillTab::McpServers => vec![
                SkillEntry {
                    name: "filesystem".to_string(),
                    source: SkillSource::Local,
                    description: "File system operations".to_string(),
                    enabled: true,
                    trigger: None,
                },
                SkillEntry {
                    name: "git".to_string(),
                    source: SkillSource::Local,
                    description: "Git operations".to_string(),
                    enabled: true,
                    trigger: None,
                },
            ],
        }
    }

    /// Get filtered entries based on search
    pub fn filtered_entries(&self) -> Vec<SkillEntry> {
        let entries = self.entries();
        if self.search_query.is_empty() {
            return entries;
        }
        let query = self.search_query.to_lowercase();
        entries
            .into_iter()
            .filter(|e| {
                e.name.to_lowercase().contains(&query)
                    || e.description.to_lowercase().contains(&query)
            })
            .collect()
    }

    /// Handle keyboard input
    pub fn handle_key(&mut self, key: ratatui::crossterm::event::KeyEvent) {
        use ratatui::crossterm::event::{KeyCode, KeyModifiers};

        match key.code {
            KeyCode::Tab => {
                self.next_tab();
            }
            KeyCode::Char('/') if !key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.search_query = String::new();
                // Note: Search mode would be handled by input handler
            }
            KeyCode::Char('r') => {
                // Reload
            }
            KeyCode::Char(' ') => {
                // Toggle expansion
                if let Some(selected) = self.list_state.selected() {
                    if self.expanded_entry == Some(selected) {
                        self.expanded_entry = None;
                    } else {
                        self.expanded_entry = Some(selected);
                    }
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let len = self.filtered_entries().len();
                if len > 0 {
                    let current = self.list_state.selected().unwrap_or(0);
                    self.list_state.select(Some((current + len - 1) % len));
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let len = self.filtered_entries().len();
                if len > 0 {
                    let current = self.list_state.selected().unwrap_or(0);
                    self.list_state.select(Some((current + 1) % len));
                }
            }
            KeyCode::Esc => {
                self.hide();
            }
            _ => {}
        }
    }

    /// Get panel height based on content
    pub fn panel_height(&self) -> u16 {
        let entry_count = self.filtered_entries().len().min(10);
        (entry_count as u16 + 7).max(12)
    }

    /// Render the panel
    pub fn render(&self, f: &mut Frame, area: Rect) {
        let entries = self.filtered_entries();

        // Create layout: title bar + tab bar + list
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Title
                Constraint::Length(1), // Tabs
                Constraint::Min(1),    // List
                Constraint::Length(1), // Footer
            ])
            .split(area);

        // Title
        let title = Paragraph::new(Text::raw("Skills"))
            .style(Style::default().fg(Color::Yellow).bold());
        f.render_widget(title, chunks[0]);

        // Tab bar
        let tabs: Vec<Span> = [
            SkillTab::Hooks,
            SkillTab::Plugins,
            SkillTab::Marketplace,
            SkillTab::Skills,
            SkillTab::McpServers,
        ]
        .iter()
        .map(|tab| {
            if *tab == self.active_tab {
                Span::raw(format!("[{}] ", tab.label()))
            } else {
                Span::raw(format!("{} ", tab.label()))
            }
        })
        .collect();
        let tabs_line = Line::from(tabs);
        let tabs_para = Paragraph::new(Text::from(tabs_line))
            .style(Style::default().fg(Color::White));
        f.render_widget(tabs_para, chunks[1]);

        // List
        if entries.is_empty() {
            let empty = Paragraph::new("No skills in this category")
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(empty, chunks[2]);
        } else {
            let items: Vec<ListItem> = entries
                .iter()
                .enumerate()
                .map(|(i, entry)| {
                    let trigger = entry
                        .trigger
                        .as_ref()
                        .map(|t| format!(" ({})", t))
                        .unwrap_or_default();

                    let prefix = if entry.enabled { "▸" } else { " " };
                    let source = format!("({})", entry.source_label());

                    let style = if Some(i) == self.expanded_entry {
                        Style::default().fg(Color::Cyan)
                    } else if Some(i) == self.list_state.selected() {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default().fg(Color::White)
                    };

                    ListItem::new(
                        Text::raw(format!(
                            "{} {} {} {}{}",
                            prefix,
                            entry.name,
                            source,
                            entry.description,
                            trigger
                        )),
                    )
                    .style(style)
                })
                .collect();

            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Cyan)),
                )
                .highlight_style(Style::default().fg(Color::Yellow).bg(Color::DarkGray));

            f.render_stateful_widget(list, chunks[2], &mut self.list_state.clone());
        }

        // Footer
        let footer = if self.search_query.is_empty() {
            Text::raw("Tab next | / search | Space expand | Esc close")
        } else {
            Text::raw(format!("Search: {} | Esc clear", self.search_query))
        };
        let footer_para = Paragraph::new(footer)
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(footer_para, chunks[3]);
    }
}

impl Default for SkillsPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tab_cycle() {
        let tab = SkillTab::Hooks;
        assert_eq!(tab.next(), SkillTab::Plugins);
        assert_eq!(SkillTab::Plugins.next(), SkillTab::Marketplace);
        assert_eq!(SkillTab::McpServers.next(), SkillTab::Hooks);
    }

    #[test]
    fn test_filtered_entries() {
        let panel = SkillsPanel::new();
        let entries = panel.entries();
        assert!(!entries.is_empty());
    }

    #[test]
    fn test_skill_source_labels() {
        let entry = SkillEntry {
            name: "test".to_string(),
            source: SkillSource::Local,
            description: "desc".to_string(),
            enabled: true,
            trigger: None,
        };
        assert_eq!(entry.source_label(), "local");
    }
}
