# MCP Server Support (Tool Configuration + Status)

**Status**: todo
**Milestone**: R3
**Category**: Tools
**Priority**: P2

## Description

The Model Context Protocol (MCP) allows Grok-like agents to connect
to external tool servers (Linear, Sentry, Grafana, filesystem, etc.).
Runie currently has only a stub `/mcps` slash command — no actual
server management, no config, no status indicators.

Grok Build TUI shows MCP servers in:
- `/mcps` — opens the Extensions modal pre-selected to "MCP Servers" tab
- Status indicator: `⛔ 6 MCP servers unavailable` at the top of the feed
- Per-server config in `~/.grok/config.toml`:
  ```toml
  [[mcp.servers]]
  name = "linear"
  command = "linear-mcp"
  args = ["--port", "8080"]
  env = { LINEAR_API_KEY = "..." }
  ```

## Acceptance Criteria

- [ ] `McpServerConfig` type: `name`, `command`, `args`, `env`
- [ ] Load MCP servers from `~/.runie/mcp.toml` (or `config.mcp` section)
- [ ] `McpStatus` enum: `Connected`, `Disconnected`, `Unavailable`
- [ ] `/mcps` slash command opens the MCP servers panel
- [ ] Each server shows: name, status badge, command preview
- [ ] Auto-injected managed MCP credentials (`RUNIE_MCP_<NAME>_TOKEN` env vars)
- [ ] Status indicator `⛔ N MCP servers unavailable` shown when servers are down
- [ ] Per-server actions: `[install]`, `[installed]`, `[update]`, `[remove]`
- [ ] `cargo build --workspace` succeeds
- [ ] `cargo test --workspace` succeeds

## Tests

### Layer 1 — State/Logic
- [ ] `parse_mcp_config_toml` — parses `[[mcp.servers]]` array
- [ ] `mcp_server_status_from_connection` — connection state → status enum
- [ ] `inject_mcp_env_vars` — adds `RUNIE_MCP_<NAME>_TOKEN` for each server
- [ ] `filter_available_servers` — returns only connected servers
- [ ] `count_unavailable_servers` — returns count of disconnected/unavailable

### Layer 2 — Event Handling
- [ ] `mcps_event_opens_panel` — `Event::OpenMcps` opens the MCP panel
- [ ] `mcp_refresh_event_runs_check` — `Event::McpRefresh` runs status check
- [ ] `mcp_server_install_event` — `Event::McpServerInstall { name }` runs install

### Layer 3 — Rendering
- [ ] `mcps_panel_shows_all_servers` — panel lists every configured server
- [ ] `mcps_panel_shows_status_badges` — each row has a `[installed]` or `[install]` badge
- [ ] `unavailable_badge_renders` — `⛔ N MCP servers unavailable` shows when count > 0

### Layer 4 — Smoke
- [ ] Adding a server to `~/.runie/mcp.toml` makes it appear in `/mcps`
- [ ] Removing a server from the config removes it from the panel

## Notes

**Related files:**
- `crates/runie-core/src/slash_command.rs` — has stub `/mcps` mapping
- `crates/runie-core/src/config.rs` — config loader (extend for `[mcp.servers]`)
- `crates/runie-tui/src/components/extensions_modal/` — modal with tab system

**Grok reference** (removed; original showed Extensions modal with MCP Servers
 tab and a status badge for unavailable servers):
> [Hooks] [Plugins] [Marketplace] [Skills] [MCP Servers]
> ⛔ 6 MCP servers unavailable

**Existing infrastructure:**
- `crates/runie-tui/src/components/extensions_modal/mod.rs` — has MCP tab already
- Just needs the underlying data plumbing

**Out of scope:**
- Implementing actual MCP protocol client (just management UI)
- OAuth flow for managed MCP servers
- Marketplace for discovering servers
- Plugin system (separate)
