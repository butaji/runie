# Pi Agent Harness - Project Research

## Project Overview

**Repository**: github.com/earendil-works/pi
**Stars**: 52.1k | **Forks**: 6.2k | **Language**: TypeScript (93.5%)

Pi is an **AI agent toolkit** organized as a monorepo containing multiple packages for building coding agents and LLM-powered applications.

## Problem Solved

Pi provides a **minimal, extensible coding agent harness** that lets developers build AI coding assistants without forking or modifying core internals. It emphasizes:

- **Adaptability**: Extensible via TypeScript extensions, skills (prompt packages), themes, and prompt templates
- **Minimalism**: Ships with essential tools (read, write, edit, bash) but no sub-agents, plan mode, or permission popups by default
- **No lock-in**: Can be shaped to fit workflows through packages shared via npm/git

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    pi-coding-agent (CLI)                    │
│  Interactive TUI, session management, tools, extensions      │
├─────────────────────────────────────────────────────────────┤
│                      pi-agent-core                         │
│  Agent runtime: tool execution, state mgmt, event streaming │
├─────────────────────────────────────────────────────────────┤
│                        pi-ai                               │
│  Unified LLM API: multi-provider, token tracking, tools      │
├─────────────────────────────────────────────────────────────┤
│                        pi-tui                              │
│  Terminal UI framework: diff rendering, async components    │
└─────────────────────────────────────────────────────────────┘
```

### Packages

| Package | Purpose |
|---------|---------|
| `packages/coding-agent` | Interactive CLI coding agent with TUI |
| `packages/agent` | Stateful agent runtime with tool calling |
| `packages/ai` | Unified multi-provider LLM API |
| `packages/tui` | Terminal UI component library |

## Key Design Decisions

### No MCP
MCP support can be built via extensions instead of being core. Rationale: keeps core minimal.

### No Sub-agents
Spawn via tmux or build custom with extensions.

### No Permission Popups
Use containers or build custom confirmation flows via extensions.

### No Plan Mode
Write plans to files or build as extension.

### No Built-in To-Dos
Confuse models. Use TODO.md or build custom.

### Cross-Provider Handoffs
Context serializable; seamlessly switch models mid-conversation.

## Directory Structure

```
pi/
├── packages/
│   ├── ai/                    # Unified LLM API
│   │   ├── src/
│   │   │   ├── providers/      # Provider implementations
│   │   │   │   ├── anthropic.ts
│   │   │   │   ├── openai-responses.ts
│   │   │   │   ├── google-generative-ai.ts
│   │   │   │   ├── bedrock-converse-stream.ts
│   │   │   │   └── ... (20+ providers)
│   │   │   ├── types.ts
│   │   │   ├── models.generated.ts
│   │   │   └── index.ts
│   │   ├── test/
│   │   └── README.md
│   │
│   ├── agent/                 # Agent runtime
│   │   ├── src/
│   │   │   ├── Agent.ts
│   │   │   ├── agentLoop.ts
│   │   │   ├── types.ts
│   │   │   └── index.ts
│   │   └── README.md
│   │
│   ├── coding-agent/          # CLI application
│   │   ├── src/
│   │   │   ├── cli/           # CLI entry points
│   │   │   ├── core/          # Core agent logic
│   │   │   ├── tui/           # TUI components
│   │   │   ├── tools/         # Built-in tools
│   │   │   └── extensions/    # Extension system
│   │   ├── examples/
│   │   ├── docs/
│   │   └── README.md
│   │
│   └── tui/                   # Terminal UI library
│       ├── src/
│       │   ├── TUI.ts
│       │   ├── components/     # Editor, Input, SelectList, etc.
│       │   └── index.ts
│       └── README.md
│
├── scripts/                   # Build/release scripts
├── .github/                   # GitHub workflows
├── AGENTS.md                  # Agent rules
└── README.md
```

## Core Technologies

- **Language**: TypeScript (Node.js)
- **Package Manager**: npm (monorepo via workspace)
- **Key Libraries**:
  - `@typebox` for schema validation
  - Custom TUI rendering engine
  - Streaming event-based architecture

## Notable Features

1. **Session Management**: JSONL-based with tree structure for branching
2. **Compaction**: Automatic context summarization for long sessions
3. **Tool Execution**: Parallel or sequential, with preflight hooks
4. **Provider Support**: 20+ LLM providers via unified API
5. **Extension System**: Full TypeScript-based extensibility
6. **Skills**: On-demand capability packages (agentskills.io standard)

## License

MIT
