**Summary:** Expanded to 8 groups covering ~90% of 2026 efficiency patterns for coding agents (Claude Code, Cursor, etc.). Prioritizes lexical→structural→semantic flow.

**Key Points:**

**1. Search & Retrieval**
`rg` (lexical, default). `ast-grep` (`sg`) for structure (e.g., `sg "fn $NAME($$$) { $$$ }"` in Rust). Tree-sitter indexing. LSP/symbols next. Semantic last. Rule: "rg → ast-grep → semantic."

**2. Project Conventions & Memory**
Root `CLAUDE.md` / `AGENTS.md` (<200 lines): search order, style, build cmds, constraints. `PROJECT.md` for architecture (Mermaid diagrams). Persistent via worktrees/index.

**3. Architecture & Sub-agents**
Orchestrator spawns focused sub-agents (`.claude/agents/*.md` with own prompt/tools/permissions). Parallel execution. Nest 2-3 levels max. Return distilled summaries.

**4. Context & Workflow Discipline**
Explicit plan → approve → execute → test/review loop. Frequent compaction. Small diffs. Reflection: "Generate → run → critique → iterate." Layered tools only.

**5. Tooling & Safety**
Core: rg, fd, ast-grep, tree-sitter, LSP. MCP servers for external (DBs, APIs). Bash/git. Structured output + runtime approvals/guards. Permissions per sub-agent.

**6. Skills & Reusable Patterns**
`SKILLS/` or `SKILL.md` files: on-demand (e.g., "PR review checklist", "performance rules"). Load via agent trigger. Portable across tools.

**7. Hooks & Automation**
Lifecycle hooks (pre/post edit, PR events). Background/async agents for long tasks. GitHub Actions integration for CI fixes/reviews.

**8. Evaluation & Scaling**
Eval loops (SWE-bench style). Token tracking. Hybrid setups (Claude terminal + Cursor IDE). Agent teams for very large work. Skills/plugins for sharing.

**Optional Deeper Layer:** Start with CLAUDE.md + rg/ast-grep + sub-agents. These yield 3-5x token/efficiency gains per benchmarks. Coverage strong but async/MCP still evolving fast (Medium confidence). Sources: Anthropic docs, arXiv, 2026 dev reports. Test small.

---

## Runie Architecture (rig-core Integration)

### Crate Structure

```
runie/
├── runie-core/     # Core types: Message, Event, Tool, Session, Context
├── runie-ai/      # AI integration: Provider trait, RigProvider, ModelRegistry
├── runie-agent/    # Agent loop, hooks, permissions
├── runie-tools/    # Tool implementations + rig adapter
├── runie-cli/      # CLI entry point
└── runie-tui/      # Terminal UI
```

### Rig Core Integration

**Provider Layer** (`runie-ai/src/providers/rig.rs`)
- `RigProvider` enum wraps rig's 22 provider clients (OpenAI, Anthropic, Gemini, etc.)
- Enum required because rig's `CompletionClient` trait returns `impl CompletionModel` with lifetimes tied to self - trait objects not feasible
- `stream_from_client()` helper avoids repeating the same 4-line streaming pattern

**Tool Integration** (`runie-tools/src/rig_adapter.rs`)
- `RunieToolAdapter` implements rig's `ToolDyn` trait
- Wraps `Box<dyn Tool + Send + Sync>` and delegates to runie's Tool trait
- Conversion: `ToolSchema` ↔ `ToolDefinition`

**Memory Integration** (`runie-ai/src/session_adapter.rs`)
- `RigSessionAdapter` wraps rig's `InMemoryConversationMemory`
- Converts runie `Message` ↔ rig `Message` including thinking/reasoning
- Enables rig memory features while maintaining runie API compatibility

### Key Design Decisions

**Why Enum Over Trait Object?**
Rig's `CompletionClient::completion_model()` returns `impl CompletionModel` with an associated `Client` type. This ties the model's lifetime to the client, making `Box<dyn CompletionClient>` impractical. The enum dispatch is the idiomatic rig approach.

**Hardcoded Metadata**
`supports_tools()`, `supports_vision()`, `max_context_tokens()` use model name heuristics. Rig's `ModelLister` can fetch actual metadata at runtime for supported providers (OpenAI, Anthropic, Gemini, DeepSeek, OpenRouter, Ollama).

### Adding a New Provider

1. Add client type to `RigProvider` enum
2. Add variant to `new()` match and `define_provider_accessors!` macro
3. Add streaming case to `chat()` match (or rely on VoyageAI exception)
4. Update `ModelRegistry::register_defaults()` if hardcoding model info

### Adding a New Tool

1. Implement `runie_core::Tool` trait in `runie-tools/src/`
2. Register in `ToolRegistry`
3. Adapter (`RunieToolAdapter`) automatically exposes it as `dyn ToolDyn` for rig

### Running Tests

```bash
cargo test --workspace --lib   # Unit tests
cargo check --workspace        # Compilation check
```