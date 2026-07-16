# Persona: The Vim Power User

> "I touch-type 120 WPM. My fingers know vim better than my own hands. If your tool can't keep up, I'll pipe it into `sed` and move on."

---

## 1. Persona Profile

### Background

**Name:** Marcus Chen  
**Age:** 34  
**Occupation:** Staff Engineer at a distributed systems company  
**Location:** San Francisco (remote)  
**Terminal Experience:** 18 years, vim for 12 years  

Marcus is a staff engineer who has been coding since he was 12. He discovered vim in college and never looked back. Today, he runs his entire workflow from tmux—a split tmux session with 6 panes is his baseline workspace. He uses vim motions everywhere: in his editor, his shell, his file manager (ranger), his git client (lazygit), and now he expects the same from his AI coding assistant.

### Expertise Level

**Advanced Power User** — Marcus represents the top 5% of terminal users. His characteristics:

- **Home row mastery**: Can navigate and edit without looking at his hands
- **Modal thinking**: Lives in NORMAL mode, enters INSERT only to type, returns to NORMAL immediately
- **Muscle memory**: Has trained vim keybindings into procedural memory over thousands of hours
- **Composability obsession**: Thinks in pipes and filters, not in monolithic applications
- **Configuration as craft**: His `~/.vimrc` is 800 lines of carefully tuned configuration

### Work Style

Marcus's typical day flows through terminal-first workflows. He wakes at 6am, checks email in mutt, reviews CI failures in vim, then enters deep work sessions with 90-minute vim sessions. He pairs via tmate, reviews diffs in fugitive, and debugs with lazygit. His tools are an extension of his craft—every configuration choice is intentional.

### Tools in His Stack

| Category | Tool | Why He Chose It |
|----------|------|-----------------|
| Terminal | tmux | Sessions, panes, persistent workflows |
| Editor | Neovim | Lua config, async, LSP native |
| Shell | zsh + fish keybindings | Fast, programmable |
| Git | lazygit | Keyboard-driven, visual git workflow |
| Files | ranger | Three-pane, vim motions, preview |
| Search | ripgrep + fzf | Fast, composable |
| Email | mutt | Terminal-native, no distractions |
| IRC | irssi | Always-on, scriptable |

---

## 2. Goals and Motivations

### Primary Goals

1. **Maintain flow state** — Marcus's most precious resource is uninterrupted focus. He measures his productivity in "flow hours," not lines of code.

2. **Never leave the keyboard** — Reaching for a mouse or navigating with arrow keys breaks his rhythm. Every context switch costs him 5-10 minutes to recover.

3. **Understand deeply** — Unlike developers who just want code generated, Marcus wants to understand what the AI suggests. He learns by seeing reasoning, not just accepting outputs.

4. **Stay in control** — He distrusts tools that "magically" do things. He wants to see what will happen before it happens and retain veto power.

5. **Build composable workflows** — His ideal tool works with his existing stack. Output should be text. State should be in files. Everything should be scriptable.

### Motivations

**EFFICIENCY (Primary)** — "My time is expensive. Every second spent fighting my tools is a second not solving real problems. If I can type `/refactor this function` instead of opening a menu, I'll save hours."

**UNDERSTANDING (Secondary)** — "I'm not here to delegate work. I'm here to be more effective. The AI should explain its reasoning so I learn, not just dump code I don't understand."

**CRAFTSMANSHIP (Tertiary)** — "My tools are an extension of my craft. A well-tuned terminal setup is as personal as a luthier's guitar. I want to shape how Runie works, not just accept defaults."

---

## 3. Pain Points with Current Tools

### Pain Point #1: Mouse Dependency

**The Frustration:** Most AI coding tools (Cursor, Copilot Chat, Claude in browser) require mouse interaction for basic operations. Marcus has to click to accept/reject suggestions, use dropdown menus for options, and navigate file trees with a mouse.

**Quote:** *"Every time I reach for the mouse, I lose 30 seconds of flow. A GUI-based AI tool is like being asked to write with my non-dominant hand."*

**Impact:** 47 context switches per hour (measured)

### Pain Point #2: Black-Box Behavior

**The Frustration:** When Claude Code or similar tools make changes, Marcus can't see what's happening. Files change silently. Commands execute without preview. Errors appear without explanation.

**Quote:** *"I need to know what the AI will do BEFORE it does it. I don't want to review a diff—I want to see the intent, understand the reasoning, and then approve."*

**Impact:** Constant verification burden, eroding trust

### Pain Point #3: Disrespect for Modal Interfaces

**The Frustration:** Many terminal AI tools don't understand modal interfaces. They don't respond to vim-style navigation, steal focus when Marcus is typing, don't integrate with his tmux workflow, and can't be scripted or piped.

**Quote:** *"I live in tmux. My AI assistant needs to be a citizen of my terminal, not a popup window that interrupts everything."*

**Impact:** Cognitive overhead of managing yet another tool paradigm

### Pain Point #4: Context Loss

**The Frustration:** Claude Code and similar tools often forget project structure after context window fills, repeat previous mistakes in new sessions, can't read `.gitignore` or project conventions, and suggest code that conflicts with existing patterns.

**Quote:** *"I've spent 20 minutes explaining my codebase to this thing, and it still doesn't understand that we're a monorepo. I shouldn't have to repeat myself."*

**Impact:** Wasted time re-establishing context

### Pain Point #5: Verbose Mode is Useless

**The Frustration:** When Marcus enables verbose/debug mode, he gets a firehose of technical output with no clear signal from noise—API calls, timing data, engineering telemetry—instead of the AI's reasoning process.

**Quote:** *"Verbose mode should show me the reasoning process, not your internal debugging. Give me a transcript of the AI's thinking, not your engineering telemetry."*

**Impact:** Unable to debug when things go wrong

---

## 4. What Would Delight This User

### Delight #1: Full Keyboard Control

Marcus wants every action accessible via keyboard with vim-style navigation in lists and panels. Escape should abort any operation. Single-key shortcuts should handle common actions. A command palette with fuzzy search completes the picture.

**Design Pattern:**
- `j/k` or arrow keys for navigation
- `Enter` to select/confirm
- `Space` to toggle selection
- `Esc` to cancel/back
- `/` to search
- `?` for help
- `q` to quit/close panel
- `:` for command mode

### Delight #2: Transparent Reasoning

Before accepting changes, Marcus wants to see the AI's thinking. He expects a structured reasoning view showing:
- What the AI observed
- Why it made certain suggestions
- What alternatives it considered
- How the suggestion aligns with project patterns

### Delight #3: Pipe-Compatible Output

Marcus wants to use Runie with Unix pipes:
```bash
# Pipe AI output to his pager
runie "explain this regex" | less -r

# Feed code from vim directly
:w !runie --lint

# Capture session for documentation
runie --session-log ~/logs/$(date +%Y%m%d).md

# Use in scripts without TTY
echo "refactor UserService" | runie --non-interactive
```

**Exit codes as contracts:**
- `0` = success
- `1` = error (with message on stderr)
- `2` = user cancelled
- `3` = partial success (some files modified)

### Delight #4: Seamless Tmux Integration

Marcus wants Runie in a dedicated tmux pane that:
- Persists across sessions
- Responds to standard tmux keybindings
- Doesn't steal focus from vim
- Can be toggled with a tmux keybinding

### Delight #5: Respect for His Config

Configuration should be via text files:
```toml
# ~/.config/runie/config.toml
provider = "anthropic"
model = "claude-opus-4-5"
keybindings = "vim"
ui.theme = "dark"
defaults.max_tokens = 4096
```

NOT hidden state, binary blobs, cloud sync that breaks offline, or GUI settings dialogs.

---

## 5. Specific UI/UX Recommendations for Runie

### Recommendation 1: Vim-Style Navigation as Default

| Key | Action | Context |
|-----|--------|---------|
| `j` / `Down` | Move down | Lists, navigation |
| `k` / `Up` | Move up | Lists, navigation |
| `h` / `Left` | Go left / parent | Hierarchy navigation |
| `l` / `Right` | Go right / child | Hierarchy navigation |
| `gg` | Jump to top | Lists |
| `G` | Jump to bottom | Lists |
| `w` / `b` | Word forward/back | Text input |
| `0` / `$` | Line start/end | Text input |
| `/` | Search forward | Search mode |
| `?` | Search backward | Search mode |
| `n` / `N` | Next/prev match | Search results |

**Critical:** `j`/`k` must work in addition to arrow keys. Not instead of.

### Recommendation 2: Three-Mode Interface

Three modes inspired by vim:

**NORMAL MODE (default)** — Status: Ready, Keys trigger actions, `Esc` returns to NORMAL from any mode

**INSERT MODE** — Status: typing, Keys insert text, `Esc` returns to NORMAL

**COMMAND MODE (:)** — Status: :cmd, Keys type commands, `Enter` executes, `Esc` returns to NORMAL

Mode indicator should always be visible in status bar.

### Recommendation 3: Status Bar Contract

Status bar must answer four questions:

1. **Where am I?** — Current panel/mode: `[Chat]`, `[Files]`, `[Settings]`, `[Onboarding]`

2. **What's selected?** — Current item: `Selected: src/services/user.rs (42 lines)`

3. **What's happening?** — Background process: `Thinking... | Model: claude-opus-4-5`

4. **What can I do?** — Mode hints: `Up/Down Navigate | Enter Select | Esc Back | ? Help`

### Recommendation 4: Diff-First for All Changes

Never auto-apply changes. Always show a diff first:

```
REVIEW CHANGES (3 files)

diff --git a/src/services/user.rs b/src/services/user.rs
--- a/src/services/user.rs
+++ b/src/services/user.rs
@@ -15,10 +15,12 @@ pub fn get_user(id: UserId) -> Result<User> {
-    let cached = cache.get(&id);
-    if let Some(user) = cached {
-        return Ok(user.clone());
-    }
+    // Check cache first for performance
+    if let Some(cached) = cache.get(&id) {
+        return Ok(cached.clone());
+    }
     }
+    
+    // NEW: Database lookup
     let user = db.query("SELECT * FROM users...")?;

[y] Accept all  [n] Reject all  [s] Stage selectively  [e] Edit
```

### Recommendation 5: Command Palette with `:`

Trigger: `:` in NORMAL mode opens command palette with fuzzy matching:

```
> ▌

> :sw
► :switch-model
  :switch-theme
  :settings
  :session new
  :session list
  :config edit
  :log show
  :quit
```

### Recommendation 6: Escape as Universal Abort

**Non-negotiable:** `Esc` must abort ANY operation and return to NORMAL mode.

| State | Action |
|-------|--------|
| In INSERT mode | Return to NORMAL mode |
| In COMMAND mode | Return to NORMAL mode |
| Search active | Clear search, return to NORMAL |
| Modal open | Close modal, return to NORMAL |
| AI thinking | Cancel request, return to NORMAL |
| Diff view | Discard changes, return to NORMAL |

**Never require multiple Esc presses, trap users in states, or close the application on Esc.**

---

## 6. Default Behaviors That Would Impress Them

### Default Behavior 1: Zero-Config Out of the Box

Marcus should be able to install Runie, run `runie`, and start coding immediately. No onboarding wizard, no "select your provider" modal.

**What impresses:** Intelligent auto-detection
```
Detected: Neovim config at ~/.config/nvim
Detected: Cargo project at .
Detected: Git repository
Using: context from current directory
Ready.
```

### Default Behavior 2: Context-Aware Defaults

From his Cargo project, Runie should:
- Read `Cargo.toml` to understand dependencies
- Read `.gitignore` to know what's excluded
- Read existing code patterns for style matching
- Not suggest changes to `target/` or generated files

**What impresses:** "It knows my project better than I do on day one."

### Default Behavior 3: Non-Intrusive Suggestions

AI suggestions appear as ghost text, but don't interrupt. Grayed out, doesn't steal focus. Fades when he types something different. No popup. No animation.

### Default Behavior 4: Visible Reasoning

When Marcus asks "why did you suggest this?", he should see structured reasoning:
```
REASONING FOR SUGGESTION

I noticed this function has 4 responsibilities:

1. Validation    (lines 10-15)
2. Transformation (lines 17-23)
3. Database I/O   (lines 25-31)
4. Caching        (lines 33-40)

This violates Single Responsibility Principle.

I suggest separating into:
- validate_input()
- transform_data()
- fetch_from_db()
- get_with_cache()

This follows the pattern used in src/utils/mod.rs (lines 5-20)
```

### Default Behavior 5: Permission Prompts for Destructive Actions

```
PERMISSION REQUIRED

This action will modify:
- src/services/user.rs (2 changes)
- src/services/user_tests.rs (1 change)

[a] Allow once    [A] Allow always    [d] Deny    [e] Edit

(Press Esc to cancel)
```

---

## 7. Keyboard Shortcut Expectations

### Core Navigation Shortcuts

| Shortcut | Action | Rationale |
|----------|--------|-----------|
| `j` / `Down` | Next item | vim convention |
| `k` / `Up` | Previous item | vim convention |
| `h` / `Left` | Parent / back | vim convention |
| `l` / `Right` | Child / forward | vim convention |
| `gg` | First item | vim convention |
| `G` | Last item | vim convention |
| `/` | Search | vim convention |
| `?` | Search backward | vim convention |
| `n` | Next match | vim convention |
| `N` | Previous match | vim convention |
| `0` | First column | vim convention |
| `$` | Last column | vim convention |
| `Ctrl+d` | Page down | vim convention |
| `Ctrl+u` | Page up | vim convention |

### Action Shortcuts

| Shortcut | Action | Rationale |
|----------|--------|-----------|
| `Enter` | Select / Confirm | Universal |
| `Space` | Toggle selection | vim visual mode |
| `Esc` | Cancel / Back | Universal abort |
| `:` | Command mode | vim convention |
| `q` | Quit panel | Many TUIs |
| `?` | Help | Universal |
| `Ctrl+c` | Interrupt / Copy | Universal |
| `Ctrl+v` | Paste | Universal |
| `Ctrl+z` | Undo | Universal |
| `Ctrl+r` | Redo | Universal |

### Runie-Specific Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+a` | Toggle AI sidebar |
| `Ctrl+l` | Clear current input |
| `Ctrl+r` | Reload context |
| `Ctrl+s` | Save session |
| `Ctrl+p` | Previous message |
| `Ctrl+n` | Next message |
| `gd` | Go to definition |
| `gr` | Find references |
| `,` (leader) | Custom macros |

### Model Switching

| Shortcut | Action |
|----------|--------|
| `mm` | Open model switcher |
| `mp` | Open provider switcher |
| `ms` | Show model status |

---

## 8. Mental Model and Workflow Patterns

### Marcus's Mental Model

Marcus thinks in pipes and filters:

```
INPUT FILTER → PROCESS PIPE → OUTPUT FILTER

vim (edit)    → sed/awk (transform) → less (view)
ripgrep (search) → fzf (filter) → vim (edit)
git log (log) → lazygit (review) → vim (diff) (compare)
```

His ideal AI workflow follows the same pattern:
1. **DISCOVER**: Ask questions, get explanations
2. **COLLABORATE**: Suggest changes, preview diffs
3. **VERIFY**: Run tests, confirm understanding

### His Workflow Pattern with AI

```
1. DISCOVER
   :!runie --explain "why does this function return None?"
   
   REASONING:
   - The function returns None when...
   - The user doesn't exist in the database
   - The query times out
   - Invalid input parameters

2. COLLABORATE
   :!runie --suggest "refactor this to use Result<T>"
   
   DIFF PREVIEW (not applied):
   - fn get_user() -> Option<User>
   + fn get_user() -> Result<User, UserError>
   
   [y] Apply  [n] Discard  [e] Edit  [?] Help

3. VERIFY
   :!runie --test "write tests for the refactored function"
   
   Generated tests:
   - test_user_not_found_returns_err
   - test_valid_user_returns_ok
   - test_timeout_returns_err
   
   Would you like me to apply these? [y/n]
```

### Session Persistence Model

Session stored as plain files (Unix philosophy):

```
~/.local/state/runie/sessions/
├── 2026-07-15-morning.yaml    # Morning session
├── 2026-07-15-afternoon.yaml  # Afternoon session
└── current.yaml               # Symlink to active
```

Benefits: Version control friendly (diff sessions), searchable with ripgrep, editable with vim, portable across machines, backup with standard tools.

---

## 9. How Runie Can Exceed Expectations (Wow Factors)

### Wow Factor #1: Hints Mode

Runie watches Marcus type and shows ghost-text hints, but NEVER applies anything without explicit command.

```
fn process_data(input: Vec<String>) -> Vec<String> {
    let results: Vec<String> = Vec::new();
    for item in input.iter() {
        results.push(░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░
                    ▲ Ghost text appears here (grayed out):
                    │  item.trim().to_uppercase()
                    │
                    │  Marcus can:
                    │  - Press Tab to accept
                    │  - Keep typing to dismiss
                    │  - Press Ctrl+h to see reasoning
```

**Why it impresses:** Seamlessly integrated, doesn't interrupt flow, transparent about reasoning.

### Wow Factor #2: Pipe Mode

Marcus can pipe any text to Runie for analysis:

```bash
# Explain a regex
echo '\d{4}-\d{2}-\d{2}' | runie --explain-regex

# Refactor pasted code
pbpaste | runie --suggest-improvements

# Generate commit message
git diff --staged | runie --commit-message

# Explain error
cargo test 2>&1 | runie --explain-error

# Ask about code structure
cat src/lib.rs | runie --analyze
```

**Why it impresses:** Composable with existing tools, respects Unix philosophy.

### Wow Factor #3: Interactive Diff

Instead of showing a unified diff, Runie shows an interactive, navigable diff with vim-style motions.

```
Keybindings in diff view:
n         → Next change
N         → Previous change
dp        → Diff put (accept this change)
dn        → Diff next (skip this change)
de        → Discard this change
da        → Accept all changes
dr        → Discard all changes
:e        → Edit current hunk
```

**Why it impresses:** Familiar vim motions, granular control, no surprises.

### Wow Factor #4: Context Tree

A visual representation of what Runie knows about the current context:

```
Project: api-server (Cargo)
├── Dependencies
│   ├── tokio (async runtime)
│   ├── serde (serialization)
│   └── tracing (logging)
├── Files (12 loaded)
│   ├── src/main.rs
│   ├── src/handlers/user.rs
│   └── src/models/mod.rs
├── Patterns (learned)
│   ├── Error handling: Result<T, Error> pattern
│   ├── Async: .await on all async calls
│   └── Logging: tracing::info! for significant events
└── Git (last 5 commits)
    ├── abc1234: Add user authentication
    └── def5678: Refactor database layer

[r] Refresh  [f] Focus  [d] Dump context  [?] Help
```

**Why it impresses:** Transparency about context window, ability to verify what the AI knows.

### Wow Factor #5: Learn Mode

Marcus can teach Runie about project-specific patterns by confirming or correcting suggestions.

```
Runie suggests:
Suggested: use anyhow::Error;
Project pattern: use thiserror::Error (custom errors)

[u] Use suggestion   [p] Use project pattern   [?] Help

Marcus presses 'p' to use project pattern.

✓ Learned: For this project, prefer thiserror over anyhow
(Added to ~/.config/runie/project-rules/api-server.toml)
```

**Why it impresses:** Improves over time, learns his preferences, becomes more helpful.

### Wow Factor #6: Silent Mode

A mode that runs entirely in the background, with no UI, outputting only to files.

```bash
# Run silently, write session to file
runie --silent --session-log ~/logs/runie-session.md << 'EOF'
Refactor the user service to use connection pooling
EOF

# Check the result later
cat ~/logs/runie-session.md
```

**Why it impresses:** For headless CI use, scripting, and batch operations.

---

## Summary: What Runie Must Do

### MUST HAVE (without these, Marcus won't use Runie)

- Full vim-style navigation (j/k/h/l/gg/G)
- Escape to abort ANY operation
- Command palette with :
- Text-based config files (TOML/YAML)
- File-based session storage
- Pipe-compatible output
- Diff-first for all changes
- No mouse required for ANY operation

### SHOULD HAVE (will significantly increase satisfaction)

- Transparent reasoning visible before action
- Ghost-text hints that don't interrupt
- Context tree to show AI's understanding
- Learn mode that improves over time
- Silent/headless mode for scripting
- Tmux-friendly (doesn't steal focus)

### NICE TO HAVE (delight factors)

- Interactive diff with vim motions
- Exit codes as proper Unix contracts
- Silent mode for CI/automation
- Context-aware auto-detection
- Zero-config defaults that "just work"

---

## References

- [TUI Design Best Practices](../research/tui_best_practices.md) — Keyboard-driven interface principles, modal design
- [Unix Philosophy](../research/unix_philosophy.md) — Composability, simplicity, text-based interfaces
- [Cognitive Load UX](../research/cognitive_load_ux.md) — Minimizing extraneous load, transparency
- [Coding Agents UX](../research/coding_agents_ux.md) — Trust, verification, agency calibration
- [lazygit](https://github.com/jesseduffield/lazygit) — Panel-based TUI pattern
- [ranger](https://github.com/ranger/ranger) — Three-column vim-style file manager
- [fzf](https://github.com/junegunn/fzf) — Fuzzy finding with keyboard navigation

---

*Document version: 1.0*  
*Created: 2026-07-15*  
*Research basis: User interviews, terminal workflow analysis, vim community best practices*
