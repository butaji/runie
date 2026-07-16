# Skills and MCP Full CRUD with Hot-Reload Implementation Plan

## Research Summary

Based on research from:
- **Claude Code** [docs](https://code.claude.com/docs/en/skills): SKILL.md format with YAML frontmatter, hot-reload via file watching
- **kimi-code** (packages/agent-core/src/skill/): Multi-source skill loading, scanner pattern
- **Grok Build**: Marketplace pattern with plugin manifest, MCP indicators

## Current State

| Component | Create | Read/List | Update | Delete | Hot-Reload |
|-----------|--------|-----------|--------|--------|------------|
| Skills | ❌ | ✅ `/skills` | ❌ | ❌ | ❌ |
| MCP | CLI ✅ | CLI ✅ | CLI ✅ | CLI ✅ | ❌ |

## Implementation Strategy

### Phase 1: Skills CRUD + Hot-Reload (Minimum Viable)

#### 1.1 Add Skill Events

```rust
// event/mod.rs
SkillsCreated { skill: Skill },
SkillsUpdated { skill: Skill },
SkillsDeleted { name: String },
ReloadSkills,
```

#### 1.2 Add Skill CRUD Commands

```rust
// commands/dsl/handlers/system.rs
"create-skill" -> handle_create_skill
"delete-skill" -> handle_delete_skill  
"reload-skills" -> handle_reload_skills
```

#### 1.3 Implement Skill File Operations

```rust
// skills/crud.rs (new file)
pub fn create_skill(name: &str, content: &str) -> Result<Skill>
pub fn update_skill(name: &str, content: &str) -> Result<Skill>
pub fn delete_skill(name: &str) -> Result<()>
```

#### 1.4 Add Hot-Reload via File Watching

```rust
// actors/io/ractor_io.rs - add watch skills directory
// Use notify crate for file watching
```

#### 1.5 TUI Commands

| Command | Action |
|---------|--------|
| `/skills` | List all skills |
| `/skill <name>` | Show skill details |
| `/create-skill <name>` | Create new skill (opens editor) |
| `/delete-skill <name>` | Delete skill |
| `/reload-skills` | Hot-reload skills |

---

### Phase 2: MCP CRUD in TUI

#### 2.1 Add MCP Events

```rust
// event/mod.rs
McpServerAdded { name: String },
McpServerRemoved { name: String },
McpServerReloaded,
ShowMcpServers,  // Show MCP panel
```

#### 2.2 Add MCP TUI Commands

```rust
// commands/dsl/handlers/system.rs
"mcp" -> handle_mcp
"mcp-add" -> handle_mcp_add
"mcp-remove" -> handle_mcp_remove
```

#### 2.3 MCP Panel UI

```rust
// dialog/mcp_panel.rs (new)
struct McpServersPanel {
    servers: Vec<(String, McpServer, ServerState)>,
}
```

#### 2.4 TUI Commands

| Command | Action |
|---------|--------|
| `/mcp` | Show MCP servers panel |
| `/mcp add <name> <command>` | Add MCP server |
| `/mcp remove <name>` | Remove MCP server |
| `/mcp restart <name>` | Restart MCP server |

---

### Phase 3: CLI Enhancement

Existing CLI commands already handle MCP. Add skills:

```bash
# Skills CLI
runie skill list          # List all skills
runie skill create <name> # Create skill
runie skill delete <name> # Delete skill
runie skill show <name>   # Show skill content

# MCP CLI (already exists)
runie mcp list
runie mcp add <name> <cmd>
runie mcp remove <name>
```

---

### Phase 4: Hot-Reload for MCP

```rust
// config watcher already exists, extend to MCP
// actors/config/handlers.rs - spawn_config_watcher()
// On config change, restart MCP servers
```

---

## File Changes Summary

### New Files

| File | Purpose |
|------|---------|
| `skills/crud.rs` | Skill CRUD operations |
| `dialog/mcp_panel.rs` | MCP server management UI |
| `commands/handlers/mcp.rs` | MCP command handlers |

### Modified Files

| File | Changes |
|------|---------|
| `event/mod.rs` | Add CRUD events |
| `event/taxonomy.json` | Add event taxonomy |
| `skills/mod.rs` | Export CRUD functions |
| `commands/dsl/handlers/system.rs` | Add CRUD commands |
| `actors/io/ractor_io.rs` | Add file watching |
| `actors/config/handlers.rs` | MCP server restart on config change |
| `update/dialog/open.rs` | MCP panel opener |
| `ui_strings.rs` | Add UI strings |

---

## Priority Order

1. **Skills CRUD** - Low effort, high impact
   - Add create/delete skill commands
   - Add `/reload-skills` command
   - Wire to existing `load_skills()` function

2. **MCP TUI** - Medium effort, high visibility
   - Add `/mcp` panel command
   - Show server status
   - Add/remove servers from UI

3. **Hot-Reload** - Medium effort, great UX
   - File watching for skills
   - Config watching for MCP

4. **CLI Enhancement** - Low effort
   - Add `runie skill` subcommand

---

## Verification

```bash
# Skills
cargo test -p runie-tests --test skills_commands
just tui --mock
/type /skills
/type /create-skill test
/type hello world
/press Enter
/type /reload-skills
/press Enter
/type /skills

# MCP
/type /mcp
/type /mcp add test "echo test"
/press Enter
/type /mcp
```
