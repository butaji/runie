//! Extensions Modal component - displays hooks, plugins, marketplace, skills, and MCP servers.

mod builder;
mod render;

pub use builder::*;
pub use render::*;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
};
use crate::theme::ThemeWrapper;

/// Extension types for the tabs
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExtensionTab {
    Hooks,
    Plugins,
    Marketplace,
    Skills,
    Mcps,
}

impl ExtensionTab {
    pub fn label(&self) -> &'static str {
        match self {
            ExtensionTab::Hooks => "Hooks",
            ExtensionTab::Plugins => "Plugins",
            ExtensionTab::Marketplace => "Marketplace",
            ExtensionTab::Skills => "Skills",
            ExtensionTab::Mcps => "MCP Servers",
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            ExtensionTab::Hooks,
            ExtensionTab::Plugins,
            ExtensionTab::Marketplace,
            ExtensionTab::Skills,
            ExtensionTab::Mcps,
        ]
    }
}

/// Scope of an extension item
#[derive(Debug, Clone, PartialEq)]
pub enum ExtensionScope {
    Project,
    Workspace,
}

/// Action status for an extension item
#[derive(Debug, Clone, PartialEq)]
pub enum ExtensionAction {
    Install,
    Installed,
    Update,
}

/// An extension item in the list
#[derive(Debug, Clone)]
pub struct ExtensionItem {
    pub name: String,
    pub version: Option<String>,
    pub scope: ExtensionScope,
    pub action: ExtensionAction,
}

impl ExtensionItem {
    pub fn new(name: &str, version: Option<&str>, scope: ExtensionScope, action: ExtensionAction) -> Self {
        Self {
            name: name.to_string(),
            version: version.map(|s| s.to_string()),
            scope,
            action,
        }
    }

    pub fn project_item(name: &str, version: Option<&str>, action: ExtensionAction) -> Self {
        Self::new(name, version, ExtensionScope::Project, action)
    }

    pub fn workspace_item(name: &str, version: Option<&str>, action: ExtensionAction) -> Self {
        Self::new(name, version, ExtensionScope::Workspace, action)
    }
}

/// ExtensionsModal state
#[derive(Clone)]
pub struct ExtensionsModal {
    pub active_tab: ExtensionTab,
    pub search_query: String,
    pub selected_index: usize,
    pub filter_scope: FilterScope,
    pub items: Vec<ExtensionItem>,
    pub theme: ThemeWrapper,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FilterScope {
    Workspace,
    Project,
}

impl ExtensionsModal {
    pub fn new() -> Self {
        Self {
            active_tab: ExtensionTab::Hooks,
            search_query: String::new(),
            selected_index: 0,
            filter_scope: FilterScope::Workspace,
            items: Self::mock_items_for_tab(ExtensionTab::Hooks),
            theme: ThemeWrapper::default(),
        }
    }

    pub fn with_tab(tab: ExtensionTab) -> Self {
        Self {
            active_tab: tab,
            search_query: String::new(),
            selected_index: 0,
            filter_scope: FilterScope::Workspace,
            items: Self::mock_items_for_tab(tab),
            theme: ThemeWrapper::default(),
        }
    }

    pub fn set_tab(&mut self, tab: ExtensionTab) {
        self.active_tab = tab;
        self.items = Self::mock_items_for_tab(tab);
        self.selected_index = 0;
        self.search_query.clear();
    }

    pub fn filtered_items(&self) -> Vec<&ExtensionItem> {
        self.items.iter().filter(|item| {
            // Filter by search query
            if !self.search_query.is_empty() {
                if !item.name.to_lowercase().contains(&self.search_query.to_lowercase()) {
                    return false;
                }
            }
            // Filter by scope if needed (for future use)
            true
        }).collect()
    }

    pub fn move_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.selected_index < self.items.len().saturating_sub(1) {
            self.selected_index += 1;
        }
    }

    pub fn render_ref(&self, area: Rect, buf: &mut Buffer, theme: &ThemeWrapper) {
        render::render(self, area, buf, theme);
    }

    /// Mock items for each tab
    fn mock_items_for_tab(tab: ExtensionTab) -> Vec<ExtensionItem> {
        match tab {
            ExtensionTab::Hooks => vec![
                ExtensionItem::workspace_item("pre-commit", Some("v3.0.0"), ExtensionAction::Installed),
                ExtensionItem::project_item("eslint-hook", Some("v1.2.3"), ExtensionAction::Install),
                ExtensionItem::workspace_item("test-runner", Some("v2.1.0"), ExtensionAction::Update),
            ],
            ExtensionTab::Plugins => vec![
                ExtensionItem::workspace_item("github-actions", Some("v1.5.0"), ExtensionAction::Installed),
                ExtensionItem::project_item("slack-notify", Some("v0.8.2"), ExtensionAction::Install),
            ],
            ExtensionTab::Marketplace => vec![
                ExtensionItem::workspace_item("browser-review", Some("v0.8.2"), ExtensionAction::Install),
                ExtensionItem::workspace_item("code-analysis", Some("v1.3.0"), ExtensionAction::Install),
                ExtensionItem::project_item("team-tool", None, ExtensionAction::Install),
            ],
            ExtensionTab::Skills => vec![
                ExtensionItem::workspace_item("web-dev", Some("v2.1.0"), ExtensionAction::Installed),
                ExtensionItem::project_item("data-analysis", Some("v0.9.5"), ExtensionAction::Install),
                ExtensionItem::workspace_item("api-design", Some("v1.0.0"), ExtensionAction::Update),
            ],
            ExtensionTab::Mcps => vec![
                ExtensionItem::workspace_item("filesystem", Some("v1.2.0"), ExtensionAction::Installed),
                ExtensionItem::project_item("browser-mcp", None, ExtensionAction::Install),
                ExtensionItem::workspace_item("database-mcp", Some("v0.5.0"), ExtensionAction::Update),
            ],
        }
    }
}

impl Default for ExtensionsModal {
    fn default() -> Self {
        Self::new()
    }
}
