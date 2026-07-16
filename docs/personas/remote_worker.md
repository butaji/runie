# Persona: The Remote Worker

**Persona Name:** Alex Chen  
**Role:** Senior Backend Engineer at a Distributed Tech Company  
**Work Location:** Home office in Austin, TX (UTC-5), working with team across 5 time zones  
**Age Range:** 32-45  
**Tech Stack:** Rust, Go, PostgreSQL, Kubernetes, AWS  

---

## 1. Persona Profile

### Background

Alex has been working remotely for 6 years, transitioning from a traditional office role at a Fortune 500 company. They now work for a 200-person distributed startup with engineers across San Francisco, London, Bangalore, and Sydney. The team operates asynchronously, with core overlap hours from 10 AM-2 PM Pacific, but critical work often happens outside those windows.

Before remote work became standard, Alex spent 3 years working from coffee shops, co-working spaces, and occasionally hotel rooms while traveling. This taught them to work effectively from anywhere with unreliable internet. Now, they split time between a dedicated home office and periodic remote work while visiting family in other cities.

### Expertise Level

**Advanced Power User**

- 12+ years of software engineering experience
- Terminal-native: spends 80%+ of work time in terminal/CLI environments
- Vim user for 8 years; has customized dotfiles extensively
- Comfortable with SSH tunnels, port forwarding, and remote pair programming
- Uses tmux daily for session management and window multiplexing
- Writes custom shell scripts to automate repetitive tasks
- Has built internal CLI tools for the team

### Work Style

Alex's typical workday:

```
06:00 - Wake, quick email sweep on phone
07:00 - Deep work block: complex feature development (2-3 hours)
09:00 - Async standup in Slack, review PRs
10:00 - Core overlap meeting (video, but camera often off)
11:00 - Continue feature work, context-switching to code review
12:00 - Lunch walk (no computer)
13:00 - Afternoon deep work block
15:00 - Async collaboration: comments, documentation, planning
17:00 - End of core hours, but often continues until 18:00
```

Key characteristics:
- **Asynchronous-first:** Prefers to leave work for others to pick up rather than interrupt
- **Terminal-centric:** Rarely leaves the terminal during focused work
- **Timezone-aware:** Schedules meetings sparingly, writes detailed async updates
- **Documentation-focused:** Writes thorough PR descriptions and technical specs
- **Self-sufficient:** Minimal reliance on synchronous help; prefers to research independently

---

## 2. Goals and Motivations

### Primary Goals

1. **Maintain deep focus during work blocks**
   - Protect 2-3 hour uninterrupted windows for complex problem-solving
   - Minimize context switches that cost 23+ minutes to recover from

2. **Collaborate effectively across timezones**
   - Leave clear, actionable context for colleagues in different time zones
   - Review and provide feedback on code without requiring real-time communication
   - Reduce the number of meetings needed for alignment

3. **Remain productive regardless of connectivity**
   - Continue working during spotty WiFi or while traveling
   - Never be blocked by network issues when the work is locally possible
   - Queue work for sync when connectivity returns

4. **Ship code efficiently**
   - Minimize friction between thought and working code
   - Get quick feedback on approach without waiting for human review
   - Automate repetitive patterns without leaving the terminal

5. **Build confidence in AI-assisted work**
   - Understand what the AI did and why
   - Verify suggestions without excessive mental overhead
   - Maintain problem-solving skills rather than becoming dependent

### Motivations

- **Autonomy:** Remote work's appeal is self-direction; tools that feel controlling are frustrating
- **Flow state:** Deep focus produces the best work; anything that breaks flow is a problem
- **Ownership:** Takes pride in understanding the full context of solutions, not just accepting them
- **Efficiency:** Would rather invest time in smart defaults than repeatedly configure settings
- **Reliability:** Prefers tools that consistently work over those with occasional brilliant moments

---

## 3. Pain Points with Current Tools

### Connectivity and Latency Issues

| Pain Point | Impact | Current Workaround |
|------------|--------|-------------------|
| AI tools become unusable on slow connections (500ms+ latency) | Forces context switch to less capable tools | Maintains local Vim setup as fallback |
| Cloud-based AI services timeout during spotty connections | Lost work, wasted API tokens | Copies prompts to clipboard for retry |
| Rate limits hit silently, causing confusion | "Why did it stop working?" debugging | Keeps track manually (unreliable) |
| Session state lost when connection drops | Full conversation context gone | Writes prompts to temp files |

### SSH Workflow Frustrations

- **Tab switching fatigue:** Constantly jumping between local editor and remote server via SSH
- **Clipboard friction:** Can't easily copy AI suggestions from local tool to remote server
- **Bandwidth waste:** Full desktop GUI tools over VNC/remote desktop are sluggish
- **Terminal emulation issues:** Some AI tools don't render correctly over SSH

### Asynchronous Collaboration Barriers

- **No context preservation:** When asking for help asynchronously, must re-explain everything
- **Review latency:** Waiting for human review blocks progress on dependent work
- **Timezone grinding:** "I had to wait until they woke up" is a constant frustration

### Terminal-First Tool Limitations

- Many AI coding tools are browser-based or IDE-centric
- CLI tools that exist often have poor terminal UX (interactive prompts that break scripts)
- Tools that work well locally don't sync state across machines

### Cognitive Load Issues

- **Verification burden:** 46% of developers distrust AI accuracy; Alex spends significant time double-checking
- **"Almost right" solutions:** When AI suggestions are 90% correct, fixing the remaining 10% takes longer than writing from scratch
- **Context loss:** AI tools that don't understand the codebase produce irrelevant suggestions
- **Mental model mismatch:** Unexplained AI behavior creates uncertainty about what to expect

---

## 4. What Would Delight This User

### Instant Response, Always

> "Speed is happiness." — TUI Best Practices

Alex is delighted when:
- Tool responds in <100ms for local operations
- No spinning loaders for things that could be instant
- Progress indicators for genuinely async operations (API calls, long tasks)
- Clear differentiation between "thinking" and "waiting for network"

### Works Offline, Syncs Later

- Full functionality without internet connection
- Queue operations for when connectivity returns
- No "must be online" prompts blocking legitimate use cases
- Graceful degradation that doesn't break workflows

### Terminal-Native Everything

- True keyboard-driven interface (no mouse required)
- Composable with existing tools via pipes/stdin/stdout
- Respects terminal conventions (Esc to cancel, standard shortcuts)
- Clear visual hierarchy using ASCII and color semantics

### Transparent and Predictable

- Visible diffs before changes are applied
- Clear error messages that explain what went wrong and how to fix it
- No silent failures or surprise rate limits
- Consistent behavior across sessions

### Context-Aware Intelligence

- Understands the codebase structure, not just the current file
- Suggests code consistent with existing patterns
- Explains what it did and why when requested
- Respects `.gitignore` and project conventions

### Respects Their Workflow

- Never auto-completes or auto-applies without permission
- Pauses when user starts typing
- Remembers conversation context across sessions
- Doesn't force them to use a specific editor or toolchain

---

## 5. Specific UI/UX Recommendations for Runie

### Latency-Sensitive Design

Based on cognitive load research showing context switching costs 23+ minutes to recover from, Runie should:

| Recommendation | Implementation | Rationale |
|----------------|----------------|----------|
| **Local-first operations** | File operations, syntax parsing, context building happen locally | No network dependency for core functionality |
| **Progressive loading** | Show available UI immediately, load AI responses progressively | Perceived performance matters more than absolute |
| **Debounced input** | Don't fire API calls on every keystroke; wait 300-500ms of pause | Reduces API load and avoids "typing in tar" feeling |
| **Queue for offline** | Allow queuing requests when offline, execute when connected | Maintains productivity across connectivity changes |
| **Connection indicator** | Always show current connectivity state in status bar | No guessing about network status |

### SSH Workflow Optimizations

1. **Minimal bandwidth rendering**
   - Use ASCII characters, avoid complex Unicode that may not render
   - Support 16-color fallback for older terminals
   - Render incrementally, don't re-draw entire screen

2. **Detachable sessions**
   - tmux integration for persistent sessions over SSH drops
   - State survives connection loss
   - Reconnect without losing work

3. **Clipboard integration**
   - Support system clipboard over SSH (via OSC 52)
   - Easy transfer of suggestions between contexts
   - `Alt+Enter` to copy AI response to clipboard

### Command Palette Design

Following TUI best practices for command palettes:

```
┌──────────────────────────────────────────────────────┐
│ > ▌                                                   │
├──────────────────────────────────────────────────────┤
│ ► /model         Switch active model                  │
│   /context       Manage context files                 │
│   /session       View/create sessions                │
│   :settings      Open settings                        │
│   :help          Show help                           │
└──────────────────────────────────────────────────────┘
```

- `:` prefix for commands (vim convention)
- `/` prefix for search/navigation
- Fuzzy matching for flexible input
- Tab completion for commands
- Escape closes palette, returns to previous state

### Status Bar Contract

Per cognitive load research, the status bar should answer:

1. **Where am I?** → Current mode, panel, context
2. **What's selected?** → Current item, count selected
3. **What's happening?** → Loading state, connectivity, sync status
4. **What can I do?** → Mode-specific available actions

Example status bar:
```
[chat] │ Model: claude-sonnet-4 │ ● Connected │ Queued: 0 │ ? Help
```

### Error Handling Patterns

Following Unix philosophy "fail loudly and as soon as possible":

| Error Type | Display | User Action |
|------------|---------|-------------|
| Network timeout | `[Timeout] Retrying in 5s... (Ctrl+C to cancel)` | Non-blocking retry |
| Rate limit | `[Rate limited] Resets at 14:32 (3m 22s)` | Clear countdown |
| Auth failure | `[Auth error] Check API key in settings` | Link to settings |
| Context overflow | `[Context full] 45K/200K tokens. /compact or /clear` | Actionable next step |

### Keyboard Shortcuts

Follow vim-inspired conventions that Alex already knows:

| Key | Action |
|-----|--------|
| `j/k` | Navigate down/up |
| `h/l` | Collapse/expand, left/right panels |
| `Enter` | Select, confirm |
| `Esc` | Cancel, back out (always works) |
| `Space` | Toggle selection |
| `/` | Search within view |
| `?` | Show context help |
| `:` | Command palette |
| `Ctrl+C` | Cancel current operation |
| `Ctrl+Q` | Quit |
| `Ctrl+W` | Close current panel |

---

## 6. Default Behaviors That Would Impress Them

### Smart Defaults That "Just Work"

Based on research showing 30-50% faster task completion with progressive interfaces:

1. **Context auto-discovery**
   - Automatically includes relevant files based on git diff
   - Adds recent commits' files for context
   - Includes test files alongside implementation

2. **Intelligent model selection**
   - Chooses appropriate model based on task complexity
   - Fast responses for simple queries, stronger models for complex refactoring
   - Allows override but defaults sensibly

3. **Permission boundaries**
   - Read-only by default for first-time commands
   - Explicit approval required for file modifications
   - Graduated autonomy based on command type

### Diff-First Approach

Following research finding that visible diffs before accepting increases trust:

```
┌─────────────────────────────────────────────────────────────┐
│  AI Suggestion: Add error handling to parse_config()       │
├─────────────────────────────────────────────────────────────┤
│  ─────────────────────────────────────────────────────────  │
│  25,26d25                                                 │
│  <                                                         │
│  <     // Parse config file                                │
│  27a27,31                                                 │
│  >     // Parse config file                                 │
│  >     data, err := os.ReadFile(path)                      │
│  >     if err != nil {                                     │
│  >         return fmt.Errorf("config read: %w", err)      │
│  >     }                                                   │
│  ─────────────────────────────────────────────────────────  │
│                                                             │
│  [Tab] Apply  │  [e] Edit  │  [d] Discard  │  [?] Explain │
└─────────────────────────────────────────────────────────────┘
```

### Respects Terminal Conventions

- Standard ANSI escape sequences for colors/formatting
- Proper terminal detection (TERM environment variable)
- Respects `$NO_COLOR` environment variable
- Supports both dark and light terminal backgrounds

### Session Persistence

- Sessions persist across app restarts
- Named sessions for different contexts (feature work, code review, debugging)
- Quick switch between sessions with fuzzy search
- Export/import sessions for sharing or backup

---

## 7. Latency and Connectivity Considerations

### The 100ms Rule

Research from TUI best practices shows that response time directly impacts user experience:

| Response Time | User Perception | Behavior |
|--------------|----------------|----------|
| <100ms | Instant | No waiting, flow maintained |
| 100-300ms | Noticeable | Mild awareness of delay |
| 300-1000ms | Waiting | Mental context maintained |
| 1-3s | Frustrating | Context switching begins |
| >3s | Abandoning | User注意力分散 |

Runie should prioritize:
1. **Local operations <50ms** (context building, file parsing)
2. **User feedback <100ms** (input acknowledgment, cursor movement)
3. **AI response streaming** with first token <500ms

### Connection State Management

```
┌─────────────────────────────────────────────────────────────┐
│  Connection States                                          │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ● Connected ─────── Normal operation                       │
│  ○ Reconnecting ──── Attempting restore (exponential back) │
│  ○ Offline ───────── Full offline mode active               │
│  ○ Queued ────────── N requests waiting for connection      │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Graceful Degradation

When connectivity is poor:

1. **Queue non-critical requests** (comments, refactoring suggestions)
2. **Prioritize critical paths** (code generation for active file)
3. **Reduce request frequency** (debounce more aggressively)
4. **Show degradation clearly** ("Running in limited mode")

### Bandwidth Optimization

For SSH and low-bandwidth scenarios:

- **Incremental rendering** — update only changed portions of screen
- **Efficient encoding** — avoid sending full screen state on updates
- **Compression** — compress large context payloads
- **Progressive enhancement** — basic text first, formatting when bandwidth allows

---

## 8. Offline Capability Requirements

### Must-Work-Offline Features

Based on Unix philosophy principles (programs should work together, handle streams):

| Feature | Offline Behavior | Sync Behavior |
|---------|-----------------|---------------|
| Codebase context building | Full local operation | N/A |
| Syntax highlighting | Full local operation | N/A |
| File diff viewing | Full local operation | N/A |
| Session management | Full local operation | Export/import |
| Command history | Full local operation | Export/import |
| Context file management | Full local operation | Sync on reconnect |
| Query composition | Full local operation | N/A |
| Query execution | **Blocked** | Queue and execute on reconnect |
| Response streaming | **Blocked** | N/A |

### Offline Data Requirements

- **Cached context** — Previously loaded codebase context available offline
- **Local embeddings** — Semantic search works with cached data
- **Session state** — All session data persisted locally
- **Configuration** — Full config available offline (no cloud dependency)

### Sync Strategy

```
┌─────────────────────────────────────────────────────────────┐
│  Offline → Online Transition                                │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  1. Connection restored                                    │
│  2. Show notification: "Reconnected. Syncing..."          │
│  3. Process queued requests (with user confirmation)        │
│  4. Sync session state                                     │
│  5. Update context with any missed changes                 │
│  6. Clear "offline" indicator                              │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Data Integrity

- **No data loss** — All user data persists locally regardless of connectivity
- **Conflict resolution** — Clear UI when local and remote conflict
- **Backup before sync** — Keep local copy before overwriting
- **Atomic operations** — Don't leave partial state on failure

---

## 9. How Runie Can Exceed Expectations (Wow Factors)

### 1. Intelligent Context Management

**The Wow:** Runie understands the codebase better than Alex does.

- Automatically identifies related files when working on a feature
- Remembers which files Alex has been working on across sessions
- Suggests context improvements ("This function references `auth.rs` but it's not in context")
- Builds semantic index for codebase-wide questions

### 2. Async-First Collaboration

**The Wow:** "It wrote the PR description for me."

- Generate detailed PR descriptions from diff and commit messages
- Create review notes highlighting potential issues
- Draft async standup updates explaining recent work
- Generate context summaries for handoff to teammates in other timezones

### 3. Terminal-Native Intelligence

**The Wow:** "It feels like having a senior engineer in my tmux session."

- Inline explanations of complex code patterns
- Contextual man page generation
- Debug session capture and sharing
- Integration with existing CLI tools (jq, git, sed, awk)

### 4. Proactive Assistance

**The Wow:** Runie anticipates needs before being asked.

- Flags potential issues in code being reviewed
- Suggests test cases based on recent changes
- Identifies documentation gaps
- Notices when context might be stale

### 5. Composable Architecture

**The Wow:** Runie plays well with others.

```bash
# Pipe output to other tools
runie explain-error ./error.log | jq '.suggestions[]'

# Use with existing workflows
git diff | runie generate-tests

# Export for sharing
runie session export team-session.json
```

Following Unix philosophy: "Write programs that do one thing well... Expect the output of every program to become the input to another."

### 6. Trust-Building Transparency

**The Wow:** "I understand exactly what it did and why."

- Every suggestion shows relevant context that informed it
- "Why this suggestion?" available with one keystroke
- Clear confidence indicators when uncertain
- Explicit limitations stated upfront

### 7. Flow State Protection

**The Wow:** "I forgot it was there until I needed it."

- Non-intrusive presence in terminal
- Resumes seamlessly after interruptions
- Never auto-generates without user trigger
- Subtle notifications that don't break focus

### 8. Global Accessibility

**The Wow:** "Works perfectly from a remote server in Singapore."

- Full functionality over SSH with 300ms latency
- Minimal bandwidth mode for metered connections
- Adapts to terminal limitations gracefully
- Timezone-aware scheduling for async operations

---

## Summary: What Makes Runie the Remote Worker's Tool

| Need | How Runie Meets It |
|------|-------------------|
| Deep focus | Instant local responses, no blocking operations |
| Async collaboration | Session sharing, PR automation, handoff tools |
| Connectivity independence | Full offline capability, graceful degradation |
| Terminal-native workflow | Keyboard-first, tmux-compatible, composable |
| Trust and transparency | Diff-first, explainable suggestions, clear errors |
| Efficiency | Smart defaults, context awareness, minimal friction |

---

*Document version: 1.0*  
*Research base: coding_agents_ux.md, unix_philosophy.md, tui_best_practices.md, cognitive_load_ux.md*  
*Target audience: Runie product team, UX designers, developers*
