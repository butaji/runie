# Refactor Tools Behind a `ToolRegistry` Trait

**Status**: todo
**Milestone**: R3
**Category**: Tools
**Priority**: P0

**Depends on**: event-bus-jsonl-persistence
**Blocks**: mcp-client-integration, permission-rulesets

## Description

Tools in Runie are currently a single `Tool` enum in
`crates/runie-agent/src/tools.rs` with a monolithic match for execution and
schema generation. Research from Goose (`ExtensionManager`), AutoGen
(`Workbench`), OpenHarness (`ToolRegistry`), and thClaws shows that tools
should be registered behind a trait so that built-ins, MCP servers, and future
user tools coexist without special-casing.

## Acceptance Criteria

- [ ] `crates/runie-core/src/tool.rs` defines a `Tool` trait:
  ```rust
  #[async_trait]
  pub trait Tool: Send + Sync {
      fn name(&self) -> &str;
      fn description(&self) -> &str;
      fn input_schema(&self) -> Value;
      fn is_read_only(&self) -> bool { false }
      fn requires_approval(&self, _input: &Value) -> bool { true }
      async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolOutput>;
  }
  ```
- [ ] `crates/runie-core/src/tool.rs` defines `ToolRegistry`:
  ```rust
  pub struct ToolRegistry {
      tools: HashMap<String, Arc<dyn Tool>>,
  }
  impl ToolRegistry {
      pub fn register(&mut self, tool: Arc<dyn Tool>);
      pub fn list(&self) -> Vec<&Arc<dyn Tool>>;
      pub fn get(&self, name: &str) -> Option<&Arc<dyn Tool>>;
      pub fn schemas(&self) -> Vec<Value>;
  }
  ```
- [ ] Each existing tool (`bash`, `read_file`, `write_file`, `edit_file`, `ls`,
  `grep`, `find`, `fetch_docs`) is moved to its own small module implementing
  `Tool`.
- [ ] `runie-agent/src/tools.rs` becomes a thin module that assembles the
  built-in registry.
- [ ] Tool execution emits `AgentEvent::ToolCallStart/End/Error` to the bus.
- [ ] Tool results are wrapped in `ToolOutput { content, bytes_transferred, duration, status }`.
- [ ] `cargo build --workspace` succeeds.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `registry_registers_and_retrieves_tool` — register a test tool, get it
  back by name.
- [ ] `registry_schemas_include_name_and_description` — `schemas()` returns
  valid JSON schemas.
- [ ] `read_only_tool_returns_true` — `ReadFile` is read-only, `WriteFile` is
  not.
- [ ] `tool_output_records_bytes_and_duration` — execution populates metadata.

### Layer 2 — Event Handling
- [ ] `tool_call_emits_start_and_end_events` — `AgentActor` calls a tool, bus
  receives start and end events.
- [ ] `tool_error_emits_error_event` — failing tool emits `ToolCallError`.

### Layer 3 — Rendering
- [ ] `tool_card_renders_from_event` — TUI renders a tool card from
  `AgentEvent::ToolCallStart/End`.

## Notes

**Why keep it in `runie-core`:**
- The `Tool` trait and registry are domain concepts, not agent implementation
  details. `runie-tui` may need tool metadata for rendering and the command
  palette.

**Read-only vs mutating:**
- Read-only tools can be auto-approved in read-only mode; mutating tools
  require explicit permission (see `permission-rulesets`).

**Files touched:**
- `crates/runie-core/src/tool.rs` (new)
- `crates/runie-core/src/tool/` (new modules for schemas/context/output)
- `crates/runie-agent/src/tools/` (one file per built-in tool)
- `crates/runie-agent/src/tools.rs` (registry assembly)
- `crates/runie-agent/src/turn.rs`

**Out of scope:**
- MCP tool discovery (covered by `mcp-client-integration`).
- User-defined tools from config (future task).
