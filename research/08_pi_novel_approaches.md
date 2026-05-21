# Pi Agent Harness - Novel Approaches & Innovations Analysis

## Project Overview
A multi-package monorepo (4,225 commits, 52k stars) containing:
- `@earendil-works/pi-ai` - Unified multi-provider LLM API
- `@earendil-works/pi-agent-core` - Stateful agent runtime
- `@earendil-works/pi-coding-agent` - Interactive coding agent CLI
- `@earendil-works/pi-tui` - Terminal UI framework

---

## 1. pi-ai: Unified Multi-Provider LLM API

### Cross-Provider Handoffs (Unique)
- Seamless model switching mid-conversation
- Automatic message transformation for compatibility (thinking blocks → tagged text)
- Preserves full context including tool calls, tool results, images
- Enables "start fast, switch to capable when needed" workflows

### Streaming Tool Calls with Partial JSON
- Progressive parsing of tool arguments during streaming
- Real-time UI updates before complete args available
- Defensive parsing (partial JSON always returns empty object, never undefined)
- Allows showing file paths being written before content arrives

### Faux Provider for Testing
- In-memory test provider with scripted responses
- Token-per-second pacing simulation
- Queue-based response consumption
- Supports multi-model registration for switching tests
- Tool call argument streaming simulation

### TypeBox Tool Schema Validation
- Type-safe tool definitions using TypeBox (serializable to JSON)
- `validateToolCall()` for runtime argument validation
- Validation errors returned to model as tool results (enables retry)
- Agent loop validates automatically before execution

### 25+ Provider Support Including Niche Ones
- MiniMax, Kimi Moonshot, Xiaomi MiMo (including token plan regions: cn/ams/sgp)
- Cloudflare AI Gateway, Cloudflare Workers AI
- OpenCode Zen/Go, GitHub Copilot
- OpenAI, Anthropic, Google, DeepSeek, Mistral, Groq, Cerebras, xAI, Together AI, etc.

### OpenAI Compatibility Layer
- Per-provider `compat` settings for OpenAI-compatible APIs
- URL-based auto-detection for known providers
- Custom headers, baseUrl override for proxies
- `thinkingFormat` mapping for reasoning params across providers

---

## 2. pi-agent-core: Stateful Agent Runtime

### Multi-Phased Event Streaming
Rich event taxonomy:
- `agent_start`/`agent_end`, `turn_start`/`turn_end`
- `message_start`/`message_update`/`message_end` (for user, assistant, toolResult)
- `tool_execution_start`/`tool_execution_update`/`tool_execution_end`
- `error` with partial content preservation

### Steering & Follow-up Queues
- **Steering**: Interrupt agent mid-tool-execution, injected after current turn completes
- **Follow-up**: Queue work to run after agent would otherwise stop
- Mode configuration: `"one-at-a-time"` or `"all"`
- Clear methods for dropping queued messages

### Parallel vs Sequential Tool Execution
- Global config: `toolExecution: "parallel" | "sequential"`
- Per-tool override via `executionMode` on `AgentTool`
- Parallel: concurrent execution, `tool_execution_end` as soon as each tool finalizes
- Sequential: one-by-one matching historical behavior

### Before/After Tool Hooks
```typescript
beforeToolCall: async ({ toolCall, args, context }) => {
  if (toolCall.name === "bash") return { block: true, reason: "disabled" };
}
afterToolCall: async ({ toolCall, result, isError, context }) => {
  return { details: { ...result.details, audited: true } };
}
```

### Tool-Requested Termination
- Tools return `terminate: true` to hint skip next LLM call
- Only takes effect when ALL finalized tool results in batch set it
- Runtime-only hint, transcript messages remain standard

### Transform Context Pipeline
```
AgentMessage[] → transformContext() → AgentMessage[] → convertToLlm() → Message[]
     (optional)                                         (required)
```
- Enables pruning, compaction, context injection
- Runs before each LLM call

### Custom Message Types via Declaration Merging
```typescript
declare module "@earendil-works/pi-agent-core" {
  interface CustomAgentMessages {
    notification: { role: "notification"; text: string; timestamp: number };
  }
}
```
- Filter in `convertToLlm` to exclude from LLM

---

## 3. pi-coding-agent: Interactive Coding Agent CLI

### Session Tree Structure (In-Place Branching)
- Single JSONL file per session with `id`/`parentId` tree
- `/tree` navigates tree in-place, continues from any point
- Branch switching preserves all history
- No file duplication on branching

### Context File Discovery (CWD-Up Walking)
- Loads `AGENTS.md`/`CLAUDE.md` from:
  - `~/.pi/agent/` (global)
  - Parent directories (walking up to root)
  - Current directory
- All matching files concatenated
- Project-specific overrides

### Philosophy: Minimal Core, Max Extensibility
Explicitly excludes:
- **No MCP built-in** → Build with extension
- **No sub-agents** → Use extensions or tmux
- **No permission popups** → Use container or custom extension
- **No plan mode** → Use files or extensions
- **No built-in TODOs** → Use TODO.md file
- **No background bash** → Use tmux

### Pi Packages System
```json
{
  "name": "my-pi-package",
  "keywords": ["pi-package"],
  "pi": {
    "extensions": ["./extensions"],
    "skills": ["./skills"],
    "prompts": ["./prompts"],
    "themes": ["./themes"]
  }
}
```
- Share via npm or git
- Auto-discovery from conventional directories
- Security note: arbitrary code execution

### Skills System (agentskills.io compatible)
- On-demand capability packages
- Invoke via `/skill:name` or auto-load
- SKILL.md format with steps
- Search paths: `~/.pi/agent/skills/`, `.pi/skills/`, `~/.agents/skills/`

### Session Compaction
- Automatic: triggers on context overflow (retry) or approaching limit (proactive)
- Manual: `/compact [instructions]`
- Lossy compaction, full history preserved in JSONL
- `/tree` revisits original history
- Customizable via extensions

### OSS Session Sharing
- Publish sessions to Hugging Face via `badlogic/pi-share-hf`
- Public data helps improve models with real-world workflows
- Regular publishing of `pi-mono` work sessions

### Message Queue in Interactive Mode
- **Enter**: queues steering message (delivered after current turn)
- **Alt+Enter**: queues follow-up message (delivered after agent stops)
- **Escape**: aborts, restores queued to editor
- **Alt+Up**: retrieves queued messages back to editor

---

## 4. pi-tui: Terminal UI Framework

### Three-Strategy Differential Rendering
1. **First render**: Output all lines, don't clear scrollback
2. **Width changed or change above viewport**: Clear screen, full re-render
3. **Normal update**: Move cursor to first changed line, clear to end, render changed lines

### Synchronized Output (CSI 2026)
- Atomic screen updates via `\x1b[?2026h` ... `\x1b[?2026l`
- No flicker during updates
- All updates wrapped in synchronized output

### Focusable Interface with IME Support
```typescript
class MyInput implements Component, Focusable {
  focused: boolean = false; // Set by TUI on focus change
  render(width: number): string[] {
    const marker = this.focused ? CURSOR_MARKER : "";
    return [`> ${beforeCursor}${marker}${atCursor}${afterCursor}`];
  }
}
```
- CURSOR_MARKER: zero-width APC escape sequence
- TUI positions hardware cursor at marker location
- IME candidate windows appear at correct position

### Bracketed Paste Mode
- Detects >10 line pastes with markers
- Creates `[paste #1 +50 lines]` marker
- Correct handling of large clipboard content

### Overlay System
```typescript
tui.showOverlay(component, {
  anchor: 'bottom-right',
  offsetX: 2, offsetY: -1,
  row: "25%",  // percentage positioning
  col: "50%",
  visible: (w, h) => w >= 100,  // responsive visibility
  nonCapturing: true  // don't auto-focus
});
```
- Anchor-based positioning (9 anchor points)
- Percentage-based positioning
- Margin from terminal edges
- Visibility callbacks

### Component Caching Pattern
```typescript
render(width: number): string[] {
  if (this.cachedLines && this.cachedWidth === width) {
    return this.cachedLines;
  }
  // ... compute
  this.cachedWidth = width;
  this.cachedLines = lines;
  return lines;
}
```

---

## 5. Supply-Chain Hardening (Notable)

- Exact versions for external deps, version-ranged for internal
- `min-release-age=2` avoids same-day npm releases
- `package-lock.json` is ground truth, pre-commit blocks changes
- `npm-shrinkwrap.json` pins transitive deps for published CLI
- `--ignore-scripts` on installs
- Explicit allowlist for dependency lifecycle scripts
- Release smoke tests with isolated installs

---

## Summary: What Makes This Project Different

| Aspect | Uniqueness |
|--------|------------|
| Cross-provider handoffs | Seamless model switching mid-conversation with automatic message transformation |
| Streaming partial tool args | Real-time UI updates during JSON streaming before complete arguments |
| Session tree structure | In-place branching in single JSONL file, no file duplication |
| Differential TUI rendering | CSI 2026 synchronized output, three-strategy update system |
| Philosophy | Minimal core, aggressive extensibility, explicit "no baked-in features" stance |
| IME support | Focusable interface with CURSOR_MARKER for proper input method positioning |
| Tool execution hooks | beforeToolCall/afterToolCall enable permission gates and audit trails |
| Supply-chain | 2-day npm release age, shrinkwrap for CLI, explicit lifecycle script allowlist |
| Session sharing | Public OSS session dataset for real-world training data |
