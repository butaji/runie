# FFF Search Integration

## Context

Runie currently shells out for file and content search:
- `grep` tool spawns `rg` or `grep`.
- `find` tool spawns `fd` or `find`.
- `list_dir` tool walks the directory with `std::fs::read_dir`.
- The `@` file picker performs its own traversal/matching.

Each call pays the cost of process spawn, `.gitignore` re-parsing, directory re-stat, and text output re-parsing. For an AI agent that may run hundreds of searches per session, this dominates latency and token cost.

`fff` (`fff-search`) is a Rust-native file-search library that keeps an in-memory index + content cache, supports typo-resistant fuzzy matching, frecency scoring, git-status awareness, definition classification, and background filesystem watching. It is designed for long-running processes and AI agents.

## Decision

Runie will integrate `fff-search` as the primary search backend. Integration will be native (Rust crate), not via the FFF MCP server. A long-lived `FffIndexerActor` owns the shared `FilePicker`/`FrecencyTracker`/`QueryTracker` state, and both agent tools and the TUI query it through the event bus.

### Scope

In scope:
- Native `fff-search` crate dependency.
- One `FffIndexerActor` per workspace session.
- A unified `search` tool replacing `grep`, `find`, and `list_dir`.
- FFF-backed `@` file picker.
- `find_definitions` agent tool using FFF’s definition classifier.
- Frecency scoring fed by tool/UI file access.
- Git-status-aware filtering.
- Fast `glob` tool and `file:line:col` location parsing.
- Configuration and `rg`/`fd` fallbacks for memory-constrained or very large repos.

Out of scope:
- FFF MCP server integration. If desired later, it can be wired via the generic MCP client task (`mcp-client-integration`).

### Architecture

```text
+--------------------------------------------------+
| FffIndexerActor                                  |
|  - SharedFilePicker                              |
|  - SharedFrecency                                |
|  - SharedQueryTracker                            |
+--------------------------------------------------+
            ^                 ^
            | event bus       | event bus
     +------+------+   +------+------+
     | search tool |   | @ picker    |
     +-------------+   +-------------+
```

### Configuration

```toml
[search]
backend = "fff"          # "fff" | "rg_fd" (legacy shell-out)
index_on_startup = true
max_index_memory_mb = 512

[search.frecency]
enabled = true

[search.fallback]
when_memory_exceeded = "rg_fd"
```

## Consequences

- **Positive:** Sub-10 ms repeated searches, structured typed results, fewer tokens, frecency ranking, git awareness, and shared state across tools and UI.
- **Positive:** Replaces four separate traversal implementations with one maintained library.
- **Trade-off:** Higher baseline memory (~360 bytes per indexed file). Configurable fallback addresses this.
- **Trade-off:** Adds a native dependency and a long-lived actor that must be shut down cleanly on exit.
