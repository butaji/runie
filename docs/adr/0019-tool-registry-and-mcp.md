# Tool Registry, Permissions, and MCP

## Context

Tools are currently a single `Tool` enum in `runie-agent` with monolithic
execution and schema generation. MCP servers are managed only by a stub UI.
Research from Goose (`ExtensionManager`), AutoGen (`Workbench`), OpenHarness
(`ToolRegistry`), and thClaws shows that tools should be registered behind a
common trait, with permissions and MCP as first-class layers.

## Decision

1. **Tool trait in `runie-core`.** All tools implement `Tool` with metadata
   (`name`, `description`, `input_schema`, `is_read_only`,
   `requires_approval`) and an async `call` method.
2. **`ToolRegistry` unifies built-ins and MCP tools.** MCP server tools are
   namespaced (`<server>__<tool>`) and registered the same way as built-ins.
3. **Permission rulesets.** Wildcard rules (`read_*`, `src/**`) evaluated
   last-match determine allow/ask/deny. Read-only tools auto-approve in
   read-only mode. Sensitive paths are hard-denied.
4. **`ApprovalSink` trait.** TUI, headless, and test modes provide their own
   approval sink.
5. **MCP client in-house.** A thin JSON-RPC client over stdio (SSE/HTTP
   stubbed) to avoid immature crate dependencies.

## Consequences

- **Positive:** Built-ins, MCP, and future user tools share one path.
- **Positive:** Permission behavior is declarative and testable.
- **Trade-off:** In-house MCP client requires maintenance; we will revisit
  available crates once the ecosystem stabilizes.
