//! MCP Servers panel builder.
//!
//! Lists configured MCP servers with actions: Add, Remove, Test, Edit.
//! Opens the MCP wizard or handles inline add/remove via events.

use super::{ItemAction, Panel, PanelStack};
use crate::Event;

/// Build the MCP servers panel listing all configured servers.
pub fn mcp_servers(servers: Vec<McpServerRow>) -> PanelStack {
    let mut panel = Panel::new("mcp-servers", " MCP Servers ")
        .header("Manage your MCP server connections")
        .keep_open();

    if servers.is_empty() {
        panel = panel.item(
            "Add your first MCP server",
            ItemAction::Emit(Event::RunPaletteCommand { name: "mcp-servers".to_string(), args: "add".to_string() }),
        );
    } else {
        panel = panel.header(format!("{} server(s) configured", servers.len()));
        for srv in servers {
            let status_icon = if srv.connected { "●" } else { "○" };
            let transport_label = srv.transport.to_lowercase();
            let label = format!("{status_icon} {} [{}]", srv.name, transport_label);
            let _desc = if srv.connected {
                format!("{} tool(s) available", srv.tool_count)
            } else {
                "disconnected".to_string()
            };
            panel = panel.item(
                label,
                ItemAction::Emit(Event::McpServerAction {
                    name: srv.name.clone(),
                    action: McpServerActionKind::Select,
                }),
            );
        }
    }

    panel = panel.separator();
    panel = panel.item(
        "+ Add MCP Server (wizard)",
        ItemAction::Emit(Event::RunPaletteCommand { name: "mcp-servers".to_string(), args: "add".to_string() }),
    );

    PanelStack::new(panel)
}

/// A row in the MCP servers panel.
#[derive(Debug, Clone)]
pub struct McpServerRow {
    pub name: String,
    pub transport: String,
    pub connected: bool,
    pub tool_count: usize,
}

/// Actions that can be performed on an MCP server row.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum McpServerActionKind {
    Select,
    Test,
    Remove,
    Edit,
}
