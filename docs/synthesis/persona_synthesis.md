# Runie UI/UX Persona Synthesis

*Cross-persona analysis for the "cognitive" worktree*

---

## Executive Summary

This document synthesizes insights from 10 distinct developer personas to identify the **universal UI/UX principles** that Runie must embody to delight developers across all experience levels, work styles, and use cases. The synthesis reveals that despite diverse backgrounds, all personas share common fundamental needs: **transparency, control, speed, and cognitive simplicity**.

### Key Finding

> **The best interface is one that disappears.** When developers focus entirely on accomplishing their goals rather than figuring out how to use the tool, that's when Runie succeeds.

---

## Universal Principles (All 10 Personas Agree)

These principles emerged from every persona analysis and must guide every UI/UX decision.

### 1. Diff-First, Always

**What it means:** Show exactly what will change before any action is taken.

**Why it matters:**
- 66% of developers cite "almost right" solutions as #1 frustration
- Trust requires visibility
- Verification burden reduces productivity by 40%

**Implementation:**
```
┌─────────────────────────────────────────────────────────────┐
│  REVIEW CHANGES (3 files)                                   │
│                                                             │
│  src/auth/jwt.rs        │ +142 lines, -12 lines           │
│  src/config/mod.rs      │ +3 lines, -1 line               │
│  tests/auth_tests.rs    │ +28 lines, -0 lines             │
│                                                             │
│  [Show Diff] [Apply All] [Select] [Cancel]                 │
└─────────────────────────────────────────────────────────────┘
```

**Requirements:**
- Every file modification shows full diff before application
- Granular control: accept/reject individual changes
- Hunk-level editing capability
- Color-coded additions (green) and deletions (red)

---

### 2. Escape is Safety

**What it means:** Every operation must be abortable with a single key.

**Why it matters:**
- Flow state preservation is critical (23 min refocus cost)
- Users must feel in control at all times
- Anxiety about "stuck" states erodes trust

**Implementation:**
```
┌─────────────────────────────────────────────────────────────┐
│  NORMAL MODE │ Ctrl+C Always Cancels │ Esc Returns         │
└─────────────────────────────────────────────────────────────┘
```

**Requirements:**
- `Esc` cancels any in-progress operation
- `Ctrl+C` interrupts AI thinking
- Every modal has a clear exit path
- Never require multiple `Esc` presses
- Never close the application on `Esc`

---

### 3. Zero-Config Defaults That Work

**What it means:** Runie must work immediately without configuration.

**Why it matters:**
- 30-50% faster task completion with progressive disclosure
- Setup time is pure cost for all personas
- Defaults must serve the 80% use case

**Smart Defaults Checklist:**
```
□ Auto-detect project type (Cargo, npm, etc.)
□ Read existing code patterns for style matching
□ Respect .gitignore and project conventions
□ Conservative model selection by default
□ Mock mode enabled for exploration
□ Context preview before sending to model
□ Visible confidence indicators on suggestions
□ Audit logging enabled by default
```

---

### 4. Keyboard-First, Mouse-Optional

**What it means:** Every action accessible via keyboard.

**Why it matters:**
- 47 context switches per hour avoided by staying in terminal
- Power users live in tmux; mouse breaks flow
- TUI conventions (vim, lazygit) set expectations

**Core Shortcuts (vim-inspired):**
```
NAVIGATION
j/k           Down/Up (vim convention)
h/l           Left/Right
gg/G          Jump to top/bottom
/ ?           Search forward/backward
n/N           Next/previous match

ACTIONS
Enter         Select / Confirm
Space         Toggle selection
Esc           Cancel / Back (universal abort)
:             Command palette
?             Context-sensitive help
q             Quit panel / Close

EDITING (when in input mode)
Ctrl+A        Select all
Ctrl+Z        Undo
Ctrl+C        Copy / Cancel
Ctrl+V        Paste
```

---

### 5. Status Bar as Contract

**What it means:** Always visible, always informative.

**Why it matters:**
- Working memory constraints (4-7 chunks)
- Reduces "where am I?" cognitive load
- Trust through transparency

**The Four Questions (Answer in Status Bar):**
```
┌─────────────────────────────────────────────────────────────┐
│ [Chat] │ Model: claude-sonnet-4 │ Privacy: Standard        │
│ Context: 2,847 tokens │ Last sync: 2m ago │ [?] Help      │
└─────────────────────────────────────────────────────────────┘
```

**Status Bar Must Show:**
1. **Where am I?** — Current mode: `[Chat]`, `[Files]`, `[Settings]`
2. **What's selected?** — `Selected: src/auth/jwt.rs (142 lines)`
3. **What's happening?** — `Thinking...` or `Connected to Anthropic`
4. **What can I do?** — Mode hints: `↑↓ Navigate │ Enter Select │ Esc Back`

---

### 6. Command Palette as Power Feature

**What it means:** Fuzzy-searchable command interface.

**Why it matters:**
- Reduces memorization burden (recognition > recall)
- Discovery without memorization
- Power users can be fast; beginners can explore

**Trigger:** `:` opens command palette

**Prefix Conventions:**
```
:        Commands
/        Search
@        Symbols / File references
#        Content search
>        Actions / Pipelines
```

**Example Interface:**
```
┌─────────────────────────────────────────────────────────────┐
│ > ▌                                                      │
├─────────────────────────────────────────────────────────────┤
│ ► :switch-model                                          │
│   :switch-provider                                       │
│   :session new                                          │
│   :session list                                         │
│   :config edit                                          │
│   :privacy-set standard                                 │
│   :context-scan                                         │
└─────────────────────────────────────────────────────────────┘
```

---

### 7. Progressive Disclosure

**What it means:** Show essential first, reveal complexity on demand.

**Why it matters:**
- 30-50% faster task completion with progressive interfaces
- Beginners need guidance; experts need speed
- Cognitive overload kills productivity

**Three-Level Help System:**

**Level 1: Inline Hints (Always Visible)**
```
┌─────────────────────────────────────────────────────────────┐
│ ↑↓ Navigate │ Enter Select │ Esc Back │ ? Help │ : Cmd    │
└─────────────────────────────────────────────────────────────┘
```

**Level 2: On-Demand Help (Press `?`)**
```
┌─────────────────────────────────────────────────────────────┐
│ HELP                                                      │
├─────────────────────────────────────────────────────────────┤
│ NAVIGATION                                                │
│ j/k        Move down/up                                  │
│ gg/G       Jump to top/bottom                            │
│ /          Search forward                                 │
│                                                            │
│ ACTIONS                                                   │
│ Enter      Select / Confirm                               │
│ Space      Toggle selection                                │
│ d          Discard change                                  │
│                                                            │
│ [Press any key to close]                                  │
└─────────────────────────────────────────────────────────────┘
```

**Level 3: Full Documentation (`:help` or `F1`)**
```bash
runie help              # General help
runie help commands     # Command reference
runie help config       # Configuration guide
runie help keyboard     # Shortcut reference
```

---

### 8. Transparency by Default

**What it means:** Show reasoning, show context, show limitations.

**Why it matters:**
- 46% distrust AI accuracy (up from 31%)
- Trust requires understanding
- Confidence indicators reduce verification burden

**Context Preview (Before Any API Call):**
```
┌─────────────────────────────────────────────────────────────┐
│ Context Preview                                    [Edit] │
├─────────────────────────────────────────────────────────────┤
│ About to send 2,847 tokens to Claude (claude-3-5-sonnet) │
│                                                              │
│ Files (3):                                                  │
│ ✓ src/auth/jwt.rs (423 tokens)                             │
│ ✓ src/config/mod.rs (189 tokens)                            │
│ ✓ Cargo.toml (67 tokens)                                    │
│                                                              │
│ [Scan for Secrets] [Edit Context] [Send]                    │
└─────────────────────────────────────────────────────────────┘
```

**Reasoning Visibility:**
```
REASONING FOR SUGGESTION

I noticed this function:
1. Handles multiple responsibilities (validation, transformation, I/O)
2. Violates Single Responsibility Principle
3. Matches pattern used in src/utils/mod.rs

Suggested refactoring:
- validate_input() → validation only
- transform_data() → transformation only
- fetch_from_db() → I/O only

[Show Full Reasoning] [Apply] [Edit] [Dismiss]
```

---

### 9. Speed is Sacred

**What it means:** Instant response for local ops; progress for network.

**Why it matters:**
- Context switching costs 23 minutes to recover
- Flow state produces best work
- Terminal users expect <100ms response

**Performance Requirements:**
```
Local Operations:
- Key input response: <16ms (60fps)
- File operations: <50ms
- Context loading: <200ms
- Search results: <100ms

Network Operations:
- Show thinking indicator immediately
- Update status bar every 1s
- Batch updates (avoid flicker)
- Never block on network
```

**Graceful Degradation:**
```
┌─────────────────────────────────────────────────────────────┐
│ [●] Connected │ Model: claude-3-5-sonnet │ Latency: 234ms │
│ Context: 2,847 tokens │ Tokens remaining: ~47,153           │
└─────────────────────────────────────────────────────────────┘
```

---

### 10. Offline-First Architecture

**What it means:** Core functionality works without network.

**Why it matters:**
- Remote workers face spotty connectivity
- 3 AM incidents don't wait for connectivity
- Trust requires reliability

**Offline Capabilities:**
```
Always Available:
- Local file operations
- Context building and management
- Session history browsing
- Configuration editing
- Help and documentation

Requires Network:
- AI model communication
- Provider authentication
- Real-time collaboration

Graceful Fallback:
- Queue requests when offline
- Execute when connectivity returns
- Clear indicator of offline state
```

---

## Persona-Specific Insights

### The Vim Power User (Marcus)

**Must-Have for This Persona:**
- Full vim-style navigation (`j/k/h/l/gg/G`)
- Command palette with `:`
- Text-based config files (TOML/YAML)
- Pipe-compatible output
- Diff-first for all changes

**Wow Factors:**
- Ghost-text hints that don't interrupt
- Context tree showing AI's understanding
- Learn mode that improves over time
- Silent/headless mode for scripting

---

### The DevOps Engineer (Marcus)

**Must-Have for This Persona:**
- Headless/scriptable modes
- Exit codes as proper Unix contracts
- JSON output for piping
- Mock/recording mode for CI
- Transparent rate limiting

**Wow Factors:**
- Incident response mode
- Multi-environment diffs
- Pipeline self-documentation
- Kubernetes-native understanding

---

### The Junior Developer (Jamie)

**Must-Have for This Persona:**
- Progressive disclosure architecture
- Verification-friendly diff display
- Context visibility panel
- Learning mode toggle
- Confidence indicators

**Wow Factors:**
- Code mentor effect (explain "why")
- Confidence builder (socratic prompting)
- Time machine (cross-file analysis)
- Team voice (project-specific conventions)

---

### The Security Developer (Alex)

**Must-Have for This Persona:**
- Context preview before send
- Secret detection
- Privacy level selector
- Local audit log
- Ollama/local model support

**Wow Factors:**
- Secret Sentinel (continuous secret monitoring)
- Context Provenance (explain why each file in context)
- Compliance presets (HIPAA/SOC2/GDPR)
- Privacy score gamification

---

### The Remote Worker (Alex)

**Must-Have for This Persona:**
- Latency-sensitive design (<100ms response)
- SSH workflow optimizations
- Offline capability
- Session state persistence
- Clear connectivity indicators

**Wow Factors:**
- Async-first collaboration features
- Cross-timezone context preservation
- Composable Unix-style architecture
- Flow state protection

---

### The OSS Maintainer (Alex)

**Must-Have for This Persona:**
- Multi-repo context management
- Intelligent project switching
- AI pre-review mode
- Contributor onboarding acceleration
- CI visibility in terminal

**Wow Factors:**
- Maintainer Assistant mode
- Contributor reputation memory
- Time boxing features
- Merge confidence scoring

---

### The Platform Engineer (Jordan)

**Must-Have for This Persona:**
- First-class MCP support
- Plugin/extension API
- Internal platform integration
- Headless mode for CI/CD
- Composability with existing tools

**Wow Factors:**
- Internal Platform Awareness
- Self-Documenting Platform
- Platform Health Dashboard
- Architecture Validation

---

### The Team Lead (Sam)

**Must-Have for This Persona:**
- Team configuration management
- Convention encoding and enforcement
- Project initialization wizards
- Violation dashboards
- Knowledge transfer documentation

**Wow Factors:**
- Senior Shadow (proxy for senior dev)
- Code Review Multiplier
- Institutional Memory
- Onboarding Time Machine

---

### The Startup Founder (Morgan)

**Must-Have for This Persona:**
- Zero-config onboarding
- Full-stack context awareness
- Speed-optimized interface
- Cost transparency
- Multi-stack assistance

**Wow Factors:**
- Time machine (2x productivity)
- Stack Whisperer (cross-stack intelligence)
- Cost Optimizer (ROI focus)
- Multi-Tasking Genius

---

### The Academic Researcher (Dr. Sarah)

**Must-Have for This Persona:**
- Reproducibility guarantees
- Publication-ready code generation
- Git-first workflow
- Experiment versioning
- Citation generation

**Wow Factors:**
- Methods section generator
- Reproducibility score
- Co-pilot Review Mode
- Grant progress dashboard

---

## Critical UI/UX Patterns

### Pattern 1: The "Wait, Let Me Show You" Flow

Every AI action follows this pattern:

```
1. OBSERVE: "I noticed [observation]"
2. EXPLAIN: "I'm suggesting [action] because [reasoning]"
3. PREVIEW: [Show exact diff/changes]
4. CONFIRM: [Wait for user approval]
5. EXECUTE: [Apply changes]
6. VERIFY: [Show result and what changed]
```

**Why:** Addresses the #1 frustration: "almost right" solutions. User sees reasoning before action, reducing verification burden.

---

### Pattern 2: The Confidence Indicator

Every AI suggestion shows its confidence level:

```
┌─────────────────────────────────────────────────────────────┐
│ HIGH CONFIDENCE (green)                                      │
│ "This is idiomatic Rust. Well-tested pattern in 50K crates." │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│ MEDIUM CONFIDENCE (yellow)                                   │
│ "This SQL should work, but edge cases around NULL depend    │
│ on your database config."                                   │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│ LOW CONFIDENCE (red)                                        │
│ "I'm uncertain about the React hook order here.             │
│ Please review carefully."                                   │
└─────────────────────────────────────────────────────────────┘
```

**Why:** 46% distrust AI accuracy. Visible confidence helps users prioritize verification effort.

---

### Pattern 3: The "Are You Sure?" for Destructive Actions

Before any destructive operation:

```
┌─────────────────────────────────────────────────────────────┐
│ ⚠ PERMISSION REQUIRED                                       │
│                                                              │
│ This action will:                                           │
│ • Delete src/old_module/ (3 files)                         │
│ • Remove 12 import references                                │
│ • May break: tests/integration_test.rs                     │
│                                                              │
│ [a] Allow once  [A] Allow always  [d] Deny  [e] Edit       │
│ (Press Esc to cancel)                                       │
└─────────────────────────────────────────────────────────────┘
```

**Why:** Safety and trust. Users must always feel in control.

---

### Pattern 4: The Progressive Disclosure Accordion

Information revealed on demand:

```
┌─────────────────────────────────────────────────────────────┐
│ This function handles user authentication...                │
│                                                              │
│ [▼ Show more details]                                       │
└─────────────────────────────────────────────────────────────┘

Expanded:

┌─────────────────────────────────────────────────────────────┐
│ This function handles user authentication...                 │
│                                                              │
│ ▼ Show more details                                         │
│                                                              │
│ Line 12: bcrypt.compare() validates password hash           │
│ Line 15: JWT signed with secret from environment           │
│ Line 23: Token expires in 24 hours (configurable)          │
│                                                              │
│ Related concepts:                                           │
│ • JWT authentication flow                                    │
│ • Password hashing best practices                            │
│                                                              │
│ [▲ Show less]                                               │
└─────────────────────────────────────────────────────────────┘
```

**Why:** Cognitive load theory. Show essential first; reveal depth on demand.

---

### Pattern 5: The Context Awareness Indicator

Always visible: what does the AI see?

```
┌─────────────────────────────────────────────────────────────┐
│ CONTEXT                           [Configure Context ▼]      │
├─────────────────────────────────────────────────────────────┤
│ ✓ src/auth/jwt.rs        ✓ src/config/mod.rs    ✓ Cargo.toml │
│ ✓ tests/auth_tests.rs     ✓ .env.example        ✓ README.md   │
│                                                              │
│ Currently viewing: src/handlers/user.rs (line 45)          │
│                                                              │
│ [+] Add file  [~] Refresh  [x] Clear context  [?] Help     │
└─────────────────────────────────────────────────────────────┘
```

**Why:** Context confusion is a major pain point. Users need to know what the AI sees.

---

## Implementation Priorities

### Phase 1: Foundation (MVP)

These features must ship before anything else:

1. **Diff-First Workflow** — Every change shows diff before application
2. **Keyboard Navigation** — Full vim-style navigation (j/k/h/l/gg/G)
3. **Escape as Universal Abort** — Every operation cancellable
4. **Status Bar Contract** — Always shows location, mode, available actions
5. **Command Palette** — Fuzzy search with `:`
6. **Context Preview** — Show what's sent before any API call

### Phase 2: Trust Building

These features build confidence:

7. **Secret Detection** — Scan context for API keys, passwords
8. **Confidence Indicators** — Show AI confidence on suggestions
9. **Audit Logging** — Local, exportable interaction history
10. **Offline Detection** — Clear connectivity state
11. **Error Messages That Teach** — Explain errors with actionable guidance

### Phase 3: Power Features

These delight power users:

12. **Learn Mode** — Remember user corrections, improve over time
13. **Ghost-Text Hints** — Suggestions that don't interrupt flow
14. **Context Tree** — Visual representation of AI's understanding
15. **Pipe Mode** — Runie works with Unix pipes
16. **Headless Mode** — Scriptable, CI-friendly operation

### Phase 4: Specialization

Persona-specific features:

17. **Ollama Integration** — Local model support (Security Dev)
18. **MCP Server** — Plugin architecture (Platform Engineer)
19. **Multi-Repo Context** — Project switching (OSS Maintainer)
20. **Team Configuration** — Shared defaults (Team Lead)
21. **Reproducibility Mode** — Deterministic output (Academic)

---

## Anti-Patterns to Avoid

### ❌ Never Do: Silent Auto-Application

Bad: AI applies changes without asking
Good: Always show diff, require confirmation

### ❌ Never Do: Magic Behavior

Bad: "It just does things"
Good: Every action visible, explainable, reversible

### ❌ Never Do: Modal Overload

Bad: Every action opens a modal
Good: Inline editing where possible; modals for critical confirmations only

### ❌ Never Do: One-Way Escapes

Bad: Some states require multiple `Esc` presses
Good: Single `Esc` returns to safe state from anywhere

### ❌ Never Do: Hidden State

Bad: Configuration in binary blobs, cloud sync
Good: Text-based config files in version control

### ❌ Never Do: Color-Only Information

Bad: Red means error, but no text
Good: Color + text always paired

### ❌ Never Do: Surprise Rate Limits

Bad: Tool stops working with no warning
Good: Visible rate limit indicators, graceful degradation

### ❌ Never Do: Memory Overload

Bad: "Remember these 50 shortcuts"
Good: Progressive disclosure, recognition over recall

---

## Success Metrics

### Quantitative

| Metric | Target | Measurement |
|--------|--------|-------------|
| Time to first successful task | <2 min | Session start → first useful output |
| Keyboard shortcut retention | >80% | User can recall shortcuts after 1 week |
| Diff review rate | >95% | % of changes reviewed before accepting |
| Error message helpfulness | >4/5 | User rating of error messages |
| Offline functionality | >60% | Tasks completable without network |

### Qualitative

| Dimension | Questions to Ask |
|-----------|-----------------|
| Trust | "Would you trust Runie with production code?" |
| Speed | "Does Runie keep up with your typing?" |
| Control | "Do you always feel in control?" |
| Learning | "What did you learn today?" |
| Comparison | "How does Runie compare to [other tools]?" |

---

## Research Foundation

This synthesis is grounded in:

| Research | Key Insights Applied |
|----------|---------------------|
| Coding Agents UX | 66% "almost right" frustration, 46% distrust, trust declining |
| Unix Philosophy | Do one thing well, composability, transparency, exit codes |
| TUI Best Practices | Keyboard-first, modal design, status bar contracts, help systems |
| Cognitive Load UX | Progressive disclosure, 4-7 chunk memory, context switching costs |

---

## Conclusion

The path to Runie delighting all 10 personas is clear:

1. **Be Transparent** — Show reasoning, show context, show limitations
2. **Be Fast** — Local operations <100ms, progress for network
3. **Be Controllable** — Escape aborts everything, diff-first before changes
4. **Be Smart** — Zero-config defaults that work, context-aware suggestions
5. **Be Extensible** — Plugin architecture, composable with existing tools

When Runie embodies these principles, it becomes invisible — a tool that developers forget they're using because it just works.

---

*Document Version: 1.0*
*Created: 2026-07-15*
*Branch: cognitive*
*Research Basis: 10 persona analyses, 4 research documents*
