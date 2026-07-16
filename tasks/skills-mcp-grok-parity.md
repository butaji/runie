# Skills and MCP Support - Grok Parity Implementation Plan

## Current State

Runie already has a solid foundation for Skills and MCP support. This document outlines gaps and implementation priorities.

## Skills System ✅ (Mostly Complete)

| Feature | Status | Notes |
|---------|--------|-------|
| Skill loading from `~/.agents/skills/` | ✅ | Includes `build_skills_context()` |
| Skill loading from `~/.runie/skills/` | ✅ | |
| Skill loading from `./.runie/skills/` | ✅ | |
| `/skills` command | ✅ | Lists all loaded skills |
| `/skill <name>` command | ✅ | Shows skill details |
| SKILL.md frontmatter parsing | ✅ | YAML frontmatter with fallback |
| User invocable detection | ✅ | `(invocable)` marker in summary |
| HarnessSkill trait | ✅ | Event hooks (on_turn_start, on_tool_call, on_turn_end) |

**Gap:** No black-box tests for skills commands (ID 4.12 in scenario catalog).

## MCP System ✅ (Mostly Complete)

| Feature | Status | Notes |
|---------|--------|-------|
| `McpConnectionManager` | ✅ | Server lifecycle management |
| Schema caching | ✅ | SHA-256 fingerprinting, disk persistence |
| Stdio transport | ✅ | Full implementation |
| HTTP/SSE transport | ⚠️ | Stub implementation (tools not loaded) |
| Server state tracking | ✅ | Starting, Running, Failed, Stopped |
| Tool list from servers | ✅ | Via `list_all_tools()` |
| MCP indicators in status | ⚠️ | Not visible in UI |

**Gaps:**
1. No black-box tests for MCP functionality
2. MCP status not visible in UI (no server indicators)

## Grok Reference (from GROK.md)

### Skills
- `/skills` — List available skills ✅ (already in runie)

### MCP
Grok shows these MCP indicators:
```
MCP stdio transport
WebSocket relay
ACP stdio transport
```

Grok also shows MCP scope indicators:
```
user — global scope
team — team scope  
organization — org scope
```

## Implementation Plan

### Phase 1: Black-box Tests for Skills (Priority: P1)

Create test file: `tests/skills_commands.rs`

```rust
// Test scenarios:
// 1. /skills with no skills loaded shows "No skills loaded"
// 2. /skills with skills shows skill list
// 3. /skill <name> shows skill details
// 4. /skill <unknown> shows "Skill not found"
```

### Phase 2: Black-box Tests for MCP (Priority: P2)

Create test file: `tests/mcp_status.rs`

```rust
// Test scenarios:
// 1. MCP servers start and show in status
// 2. MCP server errors show in UI
// 3. MCP tool count visible
```

### Phase 3: MCP Status Display (Priority: P2)

Add MCP server status to status bar or diagnostics.

```rust
// In diagnostics or status:
// MCP Servers: 2 running (15 tools)
// - filesystem: running (8 tools)
// - github: failed (connection refused)
```

## Files to Modify

1. `tests/skills_commands.rs` — New black-box test file
2. `tests/mcp_status.rs` — New black-box test file  
3. `src/ui_strings.rs` — Add MCP status strings if needed

## Verification

After implementation:
1. Run `cargo test` in runie-skills
2. Live test with `just tui --mock`
3. Check `/skills` and `/skill <name>` output
