# Pi Agent Harness - Implementation Analysis

## Overview

**Repository**: github.com/earendil-works/pi  
**Stars**: 52.1k | **Forks**: 6.2k | **Commits**: 4,225  
**License**: MIT  
**Node.js**: >=22.19.0

AI agent toolkit with coding agent CLI, unified LLM API, TUI library, and Slack bot support.

---

## Tech Stack

### Core Language
- **TypeScript 93.5%** - Primary language
- **JavaScript 5.8%** - Build scripts, examples
- **CSS 0.4%**, Shell 0.3%, HTML/C 0%

### Runtime & Build
- **Node.js**: >=22.19.0
- **TypeScript**: 5.9.3 (native, no transpiler needed for emit)
- **esbuild**: 0.28.0 (fast bundling)
- **tsx**: 4.22.1 (TypeScript executor)
- **Biome**: 2.3.5 (linting/formatting - replaces ESLint+Prettier)
- **tsgo**: Internal build tool wrapping TypeScript

### Testing
- **Vitest**: 3.2.4 (agent, ai, coding-agent packages)
- **Node.js built-in test**:  (tui package only)
- **@xterm/xterm**: 5.5.0 (terminal emulation for tests)

### Monorepo
- **npm workspaces** - manages package dependencies
- **Husky**: 9.1.7 (git hooks)
- **.npmrc**: `save-exact=true`, `min-release-age=2` (supply chain hardening)

---

## Package Architecture

### 4 Core Packages (lockstep versioning)

| Package | Purpose | Key Dependencies |
|---------|---------|-----------------|
| `@earendil-works/pi-ai` | Unified LLM API | openai, @anthropic-ai/sdk, @google/genai, @mistralai/mistralai, @aws-sdk/client-bedrock-runtime |
| `@earendil-works/pi-agent-core` | Agent runtime | pi-ai, typebox, yaml |
| `@earendil-works/pi-coding-agent` | CLI application | all above + pi-tui, glob, diff, highlight.js, chalk |
| `@earendil-works/pi-tui` | Terminal UI library | marked, get-east-asian-width, koffi (optional) |

### Additional
- `@earendil-works/pi-chat` (separate repo) - Slack/chat automation

---

## Key Dependencies Detail

### pi-ai (LLM Abstraction)
```
@anthropic-ai/sdk: 0.91.1
@aws-sdk/client-bedrock-runtime: 3.1048.0
@google/genai: 1.52.0
@mistralai/mistralai: 2.2.1
openai: 6.26.0
http-proxy-agent: 7.0.2
https-proxy-agent: 7.0.6
partial-json: 0.1.7
typebox: 1.1.38
```

### pi-coding-agent (CLI)
```
@silvia-odwyer/photon-node: 0.3.4 (image processing)
chalk: 5.6.2 (colors)
cross-spawn: 7.0.6 (process spawning)
diff: 8.0.4 (text diffing)
glob: 13.0.6 (file matching)
highlight.js: 10.7.3 (syntax highlighting)
proper-lockfile: 4.1.2 (session locking)
undici: 8.3.0 (HTTP client)
yaml: 2.9.0
@mariozechner/clipboard: 0.3.6 (optional, clipboard)
```

### pi-tui (Terminal UI)
```
marked: 15.0.12 (markdown rendering)
get-east-asian-width: 1.6.0 (CJK width calculation)
koffi: 2.16.2 (optional, native interop)
@xterm/xterm: 5.5.0 (dev, terminal emulator)
```

---

## Code Organization Patterns

### Project Structure
```
pi-mono/
├── packages/
│   ├── ai/              # LLM provider abstraction layer
│   │   ├── src/
│   │   │   ├── providers/    # Provider implementations
│   │   │   ├── index.ts      # Main exports
│   │   │   └── types.ts      # Core types
│   │   ├── scripts/          # Code generation (models)
│   │   └── test/
│   ├── agent/           # Agent runtime core
│   │   ├── src/
│   │   └── test/
│   ├── coding-agent/    # CLI application
│   │   ├── src/
│   │   │   ├── cli/          # CLI entry
│   │   │   ├── core/         # Agent implementation
│   │   │   ├── modes/        # Interactive/other modes
│   │   │   └── tools/         # Built-in tools
│   │   ├── examples/         # Extension examples
│   │   └── test/
│   └── tui/             # Terminal UI components
│       └── src/
├── scripts/             # Build/release scripts
├── .husky/              # Git hooks
└── tsconfig.base.json   # Shared TypeScript config
```

### TypeScript Configuration
- Target: ES2022
- Module: Node16
- Strict mode enabled
- Erasable syntax only (no constructor parameter properties, enums, namespaces)
- Paths alias for workspace packages
- `inlineSources: true` for source maps

### Import Patterns
- **Explicit subpath exports** in package.json
- **No dynamic imports** for types
- **No inline imports** - all top-level
- Provider modules lazy-loaded via register-builtins.ts

### Build System
- `tsgo` - internal wrapper around tsc
- Separate `tsconfig.build.json` per package (noEmit: false)
- Root `tsconfig.json` for IDE/type checking with paths
- Asset copying via `shx` (shell commands)
- Binary builds via `bun build --compile`

---

## Configuration Approaches

### .npmrc
```ini
save-exact=true
min-release-age=2
```
- Exact versions for direct deps
- 2-day buffer before accepting new dep releases

### biome.json
- Linter with recommended rules
- Tab indentation, 120 char line width
- `noNonNullAssertion: off`, `noExplicitAny: off`
- Scoped to `packages/*/src/**/*.ts`

### Supply Chain Hardening
- `package-lock.json` is ground truth
- Pre-commit hook blocks accidental lockfile changes (unless `PI_ALLOW_LOCKFILE_CHANGE=1`)
- npm-shrinkwrap.json generated for coding-agent (pins transitive deps)
- Lifecycle script allowlist for shrinkwrap
- CI uses `npm ci --ignore-scripts`
- Audit: `npm audit --omit=dev` + `npm audit signatures`

---

## Build & Test Setup

### Build Commands
```bash
npm run build        # Sequential: tui → ai → agent → coding-agent
npm run check         # biome + pinned-deps + ts-imports + shrinkwrap + typecheck
npm run clean         # Clean all packages
```

### Test Commands
```bash
npm test              # Run tests in all packages
./test.sh             # Skip LLM tests if no API keys (unSets all provider keys)
```

### Test Approach
- **pi-ai**: Vitest with mocking for provider APIs
- **pi-agent-core**: Vitest with harness config
- **pi-coding-agent**: Vitest suite + regression tests
- **pi-tui**: Node.js built-in test runner
- LLM tests skipped when `PI_NO_LOCAL_LLM=1` or no API keys

### Release Process
```bash
npm run release:local          # Smoke test in isolated env
npm run release:patch/minor    # Bumps all packages together
# Publishing requires maintainer 2FA
```

---

## Notable Code Patterns

### 1. Provider Pattern (pi-ai)
- Each provider in `src/providers/<name>.ts`
- Exports: `stream()`, `streamSimple()`, options interface, message conversion
- Standardized event types: `text`, `tool_call`, `thinking`, `usage`, `stop`
- Lazy registration in `register-builtins.ts`

### 2. Extension System (coding-agent)
- Examples in `packages/coding-agent/examples/extensions/`
- Custom providers, sandbox execution
- Hooks export system at `@earendil-works/pi-coding-agent/hooks`

### 3. Session Management
- `proper-lockfile` for concurrent access prevention
- Auth stored in `~/.pi/agent/auth.json`
- Session state in `.pi` config directory

### 4. Model Resolution
- `src/core/model-resolver.ts` maps provider → default model
- Auto-generated models in `packages/ai/src/models.generated.ts`

### 5. State Management (agent-core)
- YAML-based state files
- Attachment support
- Transport abstraction layer

---

## Development Rules (from AGENTS.md)

### TypeScript Restrictions
- No `any` unless absolutely necessary
- No constructor parameter properties
- No `enum`, `namespace`, `import =`, `export =`
- No inline/dynamic imports for types
- Must use erasable syntax compatible with strip-only mode

### Git Workflow
- Feature branches, then merge to main
- Never `git add -A` (stage specific files only)
- Always include `fixes #<number>` in commit
- Parallel agents work in same worktree

### Code Quality
- Read files fully before editing
- `npm run check` after code changes (not build/test)
- No emojis in commits/issues
- Short, technical prose

---

## Entry Points

### CLI (pi-coding-agent)
```
dist/cli.js
```

### Library APIs
```
@earendil-works/pi-ai           # Main LLM API
@earendil-works/pi-ai/<provider> # Subpath exports (anthropic, openai-responses, etc.)
@earendil-works/pi-ai/oauth      # OAuth utilities
@earendil-works/pi-agent-core    # Agent runtime
@earendil-works/pi-tui           # TUI components
@earendil-works/pi-coding-agent/hooks  # Extension hooks
```

---

## Summary

**pi** is a well-engineered TypeScript monorepo for AI agent development:
- Clean separation: LLM API abstraction → Agent runtime → CLI → TUI
- Strong supply-chain security (exact versions, shrinkwrap, audit)
- Strict TypeScript (strict mode, erasable syntax only)
- Professional tooling: Biome, Vitest, npm workspaces, Husky
- Extensive documentation in AGENTS.md with contribution guidelines
- Multi-provider LLM support (OpenAI, Anthropic, Google, Mistral, Bedrock)
- Extension system for custom providers and sandboxed execution
