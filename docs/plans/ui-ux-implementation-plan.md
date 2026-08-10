# Runie UI/UX Implementation Plan

*Implementation roadmap for the "cognitive" worktree*

---

## Overview

This plan translates the persona synthesis into concrete implementation tasks. It prioritizes by impact (universal principles first) and groups related changes for efficient development.

---

## Phase 1: Foundation (MVP)

These features form the minimum viable experience that all personas require.

### 1.1 Diff-First Workflow

**Files to modify:**
- `crates/runie-core/src/commands/diff.rs` (new)
- `crates/runie-core/src/actors/turn.rs`
- `crates/runie-tui/src/components/diff_view.rs` (new)
- `crates/runie-tui/src/handlers/diff_handler.rs` (new)

**Implementation:**

```rust
// crates/runie-core/src/commands/diff.rs
pub struct DiffCommand {
    pub files: Vec<FileChange>,
    pub context: DiffContext,
}

impl DiffCommand {
    pub fn preview(&self) -> DiffPreview {
        // Generate unified diff format
        // Calculate token estimates
        // Return structured preview
    }

    pub fn accept_all(&mut self) -> Result<Vec<AppliedChange>> {
        // Apply all changes atomically
        // Return result for each file
    }

    pub fn accept_hunk(&mut self, hunk: HunkId) -> Result<AppliedChange> {
        // Apply single hunk
    }

    pub fn reject_hunk(&mut self, hunk: HunkId) {
        // Mark hunk as rejected
    }
}
```

**Key behaviors:**
- Every file modification triggers diff preview
- User must explicitly accept/reject changes
- Supports hunk-level granularity
- Undo capability for all changes

---

### 1.2 Keyboard Navigation System

**Files to modify:**
- `crates/runie-tui/src/input/mod.rs`
- `crates/runie-tui/src/input/vim_keys.rs` (new)
- `crates/runie-tui/src/handlers/navigation.rs` (new)

**Implementation:**

```rust
// crates/runie-tui/src/input/vim_keys.rs
#[derive(Debug, Clone, Copy)]
pub enum VimKey {
    Down, Up, Left, Right,
    First, Last,        // gg, G
    Next, Prev,        // n, N
    Select, Cancel,     // Enter, Esc
    Search, SearchBack, // /, ?
    Command,           // :
    Help, Quit,        // ?, q
}

impl VimKey {
    pub fn from_key_event(key: &KeyEvent) -> Option<VimKey> {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => Some(VimKey::Down),
            KeyCode::Char('k') | KeyCode::Up => Some(VimKey::Up),
            KeyCode::Char('h') | KeyCode::Left => Some(VimKey::Left),
            KeyCode::Char('l') | KeyCode::Right => Some(VimKey::Right),
            KeyCode::Char('g') => Some(VimKey::First), // gg
            KeyCode::Char('G') => Some(VimKey::Last),
            KeyCode::Char('/') => Some(VimKey::Search),
            KeyCode::Char('?') => Some(VimKey::SearchBack),
            KeyCode::Char(':') => Some(VimKey::Command),
            KeyCode::Char('?') | KeyCode::F(1) => Some(VimKey::Help),
            KeyCode::Char('q') => Some(VimKey::Quit),
            KeyCode::Enter => Some(VimKey::Select),
            KeyCode::Esc => Some(VimKey::Cancel),
            _ => None,
        }
    }
}
```

**Key bindings (vim convention):**
```
j/k, arrows     Navigate lists
h/l             Parent/child navigation
gg/G            First/last item
/ ?             Search forward/backward
n/N             Next/previous match
Enter           Select/confirm
Space           Toggle selection
Esc             Cancel/back (universal abort)
:               Command palette
?               Help
q               Quit panel
```

---

### 1.3 Status Bar Contract

**Files to modify:**
- `crates/runie-tui/src/components/status_bar.rs` (new)
- `crates/runie-tui/src/state/mod.rs`
- `crates/runie-tui/src/render.rs`

**Implementation:**

```rust
// crates/runie-tui/src/components/status_bar.rs
pub struct StatusBar {
    pub mode: Mode,
    pub location: String,
    pub selection: Option<Selection>,
    pub activity: Activity,
    pub hints: Vec<Hint>,
    pub privacy: PrivacyLevel,
    pub connectivity: Connectivity,
}

impl StatusBar {
    pub fn render(&self, f: &mut Frame) {
        // Always show four things:
        // 1. Where am I? → self.location
        // 2. What's selected? → self.selection
        // 3. What's happening? → self.activity
        // 4. What can I do? → self.hints
    }
}

// Example status bar output:
┌─────────────────────────────────────────────────────────────┐
│ [Chat] │ Model: claude-sonnet-4 │ Privacy: Standard        │
│ Context: 2,847 tokens │ Last sync: 2m ago │ [?] Help      │
└─────────────────────────────────────────────────────────────┘
```

---

### 1.4 Command Palette

**Files to modify:**
- `crates/runie-tui/src/components/command_palette.rs` (new)
- `crates/runie-tui/src/handlers/palette_handler.rs` (new)
- `crates/runie-core/src/commands/registry.rs`

**Implementation:**

```rust
// crates/runie-tui/src/components/command_palette.rs
pub struct CommandPalette {
    pub query: String,
    pub results: Vec<CommandMatch>,
    pub selected: usize,
    fuzzy: FuzzySearch,
}

impl CommandPalette {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            results: Vec::new(),
            selected: 0,
            fuzzy: FuzzySearch::new(),
        }
    }

    pub fn filter(&mut self, query: &str) {
        self.query = query.to_string();
        self.results = self.fuzzy
            .search(query, CommandRegistry::all())
            .take(10)
            .collect();
        self.selected = 0;
    }

    pub fn execute(&self) -> Option<Command> {
        self.results.get(self.selected).map(|m| m.command.clone())
    }
}

// Prefix conventions:
// : → Commands
// / → Search
// @ → File references
// # → Content search
// > → Actions
```

**Commands to register:**
```
:switch-model         Switch AI model
:switch-provider      Switch provider
:session new          Create new session
:session list         List sessions
:session fork         Fork current session
:config edit          Open config file
:config show          Show current config
:privacy-set          Set privacy level
:context-scan         Scan for secrets
:context-preview      Preview context
:help                 Show help
:quit                 Quit Runie
```

---

### 1.5 Universal Escape Handler

**Files to modify:**
- `crates/runie-tui/src/input/mod.rs`
- `crates/runie-tui/src/state/mod.rs`
- `crates/runie-tui/src/handlers/global_handler.rs` (new)

**Implementation:**

```rust
// crates/runie-tui/src/handlers/global_handler.rs
pub fn handle_escape(state: &mut AppState) -> EventResult {
    match state.current_mode() {
        Mode::Insert => {
            state.set_mode(Mode::Normal);
            EventResult::Consumed
        }
        Mode::Command => {
            state.close_command_palette();
            state.set_mode(Mode::Normal);
            EventResult::Consumed
        }
        Mode::Search => {
            state.clear_search();
            state.set_mode(Mode::Normal);
            EventResult::Consumed
        }
        Mode::Diff => {
            state.discard_changes();
            state.set_mode(Mode::Normal);
            EventResult::Consumed
        }
        Mode::Modal(open_modal) => {
            state.close_modal();
            state.set_mode(Mode::Normal);
            EventResult::Consumed
        }
        Mode::Thinking => {
            state.cancel_thinking();
            state.set_mode(Mode::Normal);
            EventResult::Consumed
        }
        Mode::Normal => {
            // Don't consume - let parent handler decide
            EventResult::Propagate
        }
    }
}
```

**Key invariant:** Single `Esc` always returns to safe state.

---

## Phase 2: Trust Building

### 2.1 Context Preview

**Files to modify:**
- `crates/runie-core/src/context/mod.rs`
- `crates/runie-core/src/context/preview.rs` (new)
- `crates/runie-tui/src/components/context_preview.rs` (new)

**Implementation:**

```rust
// crates/runie-core/src/context/preview.rs
pub struct ContextPreview {
    pub files: Vec<FileInContext>,
    pub tokens_estimate: usize,
    pub detected_secrets: Vec<SecretPattern>,
    pub provider: Provider,
    pub model: Model,
}

pub struct FileInContext {
    pub path: PathBuf,
    pub tokens: usize,
    pub reason: InclusionReason,
    pub has_secrets: bool,
}

impl ContextPreview {
    pub fn generate(context: &Context) -> Self {
        // Calculate token estimates
        // Detect secret patterns
        // Determine inclusion reasons
    }

    pub fn render_terminal(&self) -> String {
        // Generate terminal-friendly output
    }
}
```

**UI:**
```
┌─────────────────────────────────────────────────────────────┐
│ Context Preview                                    [Edit]   │
├─────────────────────────────────────────────────────────────┤
│ About to send 2,847 tokens to Claude (claude-3-5-sonnet) │
│                                                              │
│ Files (3):                                                  │
│ ✓ src/auth/jwt.rs (423 tokens) [explicit selection]        │
│ ✓ src/config/mod.rs (189 tokens) [dependency]              │
│ ✓ Cargo.toml (67 tokens) [project config]                   │
│                                                              │
│ ⚠ No secrets detected                                       │
│                                                              │
│ [Scan for Secrets] [Edit Context] [Send]                    │
└─────────────────────────────────────────────────────────────┘
```

---

### 2.2 Secret Detection

**Files to modify:**
- `crates/runie-core/src/security/secret_detector.rs` (new)
- `crates/runie-core/src/context/mod.rs`

**Implementation:**

```rust
// crates/runie-core/src/security/secret_detector.rs
pub struct SecretDetector {
    patterns: Vec<SecretPattern>,
}

impl SecretDetector {
    pub fn new() -> Self {
        Self {
            patterns: vec![
                // API Keys
                Pattern::new(r"sk-[a-zA-Z0-9]{20,}", "OpenAI API Key"),
                Pattern::new(r"sk-ant-[a-zA-Z0-9]{20,}", "Anthropic API Key"),
                Pattern::new(r"AKIA[A-Z0-9]{16}", "AWS Access Key"),
                // JWTs
                Pattern::new(r"eyJ[a-zA-Z0-9_-]*\.eyJ[a-zA-Z0-9_-]*\.[a-zA-Z0-9_-]*", "JWT Token"),
                // Passwords
                Pattern::new(r"password\s*=\s*['\"][^'\"]{8,}['\"]", "Hardcoded Password"),
                // etc.
            ],
        }
    }

    pub fn scan(&self, content: &str) -> Vec<SecretMatch> {
        self.patterns.iter()
            .filter_map(|p| p.find(content))
            .collect()
    }
}
```

---

### 2.3 Confidence Indicators

**Files to modify:**
- `crates/runie-core/src/llm/confidence.rs` (new)
- `crates/runie-core/src/turn/mod.rs`
- `crates/runie-tui/src/components/confidence_badge.rs` (new)

**Implementation:**

```rust
// crates/runie-core/src/llm/confidence.rs
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Confidence {
    High,
    Medium,
    Low,
}

impl Confidence {
    pub fn from_score(score: f32) -> Self {
        match score {
            s if s >= 0.8 => Confidence::High,
            s if s >= 0.5 => Confidence::Medium,
            _ => Confidence::Low,
        }
    }

    pub fn color(&self) -> Color {
        match self {
            Confidence::High => Color::Green,
            Confidence::Medium => Color::Yellow,
            Confidence::Low => Color::Red,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Confidence::High => "HIGH CONFIDENCE",
            Confidence::Medium => "MEDIUM CONFIDENCE",
            Confidence::Low => "LOW CONFIDENCE",
        }
    }
}
```

**UI:**
```
┌─────────────────────────────────────────────────────────────┐
│ HIGH CONFIDENCE (green)                                      │
│ "This is idiomatic Rust. Well-tested pattern used in       │
│ 50,000+ crates."                                           │
└─────────────────────────────────────────────────────────────┘
```

---

### 2.4 Audit Logging

**Files to modify:**
- `crates/runie-core/src/audit/log.rs` (new)
- `crates/runie-core/src/audit/export.rs` (new)
- `crates/runie-core/src/actors/turn.rs`

**Implementation:**

```rust
// crates/runie-core/src/audit/log.rs
#[derive(Debug, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp: DateTime<Utc>,
    pub interaction_id: String,
    pub event_type: EventType,
    pub provider: String,
    pub model: String,
    pub tokens_in: usize,
    pub tokens_out: usize,
    pub context_files: Vec<FileRef>,
    pub privacy_level: PrivacyLevel,
    pub warnings: Vec<String>,
    pub duration_ms: u64,
    pub status: Status,
}

impl AuditLog {
    pub fn new(path: PathBuf) -> Self {
        // Initialize log file
    }

    pub fn record(&mut self, entry: AuditEntry) -> Result<()> {
        // Write to file
    }

    pub fn export_csv(&self) -> Result<String> { /* ... */ }
    pub fn export_json(&self) -> Result<String> { /* ... */ }
}
```

---

## Phase 3: Power Features

### 3.1 Learn Mode

**Files to modify:**
- `crates/runie-core/src/learning/user_feedback.rs` (new)
- `crates/runie-core/src/learning/preference_store.rs` (new)
- `crates/runie-core/src/turn/mod.rs`

**Implementation:**

```rust
// crates/runie-core/src/learning/preference_store.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    pub project_rules: HashMap<ProjectId, ProjectRule>,
    pub global_rules: Vec<GlobalRule>,
    pub corrections: Vec<Correction>,
}

pub struct Correction {
    pub timestamp: DateTime<Utc>,
    pub suggestion: String,
    pub user_choice: String,
    pub reason: Option<String>,
}

impl PreferenceStore {
    pub fn learn(&mut self, correction: Correction) {
        // Update preference model
        // Persist to disk
    }

    pub fn apply_preferences(&self, suggestion: &mut Suggestion) {
        // Apply learned preferences to suggestion
    }
}
```

---

### 3.2 Ghost-Text Hints

**Files to modify:**
- `crates/runie-tui/src/components/input.rs`
- `crates/runie-tui/src/components/ghost_text.rs` (new)

**Implementation:**

```rust
// crates/runie-tui/src/components/ghost_text.rs
pub struct GhostText {
    pub suggestion: String,
    pub start_pos: usize,
    pub confidence: Confidence,
}

impl GhostText {
    pub fn render(&self, f: &mut Frame, area: Rect) {
        // Render suggestion in muted color
        // Underline current position
    }
}

// Input component integration:
impl InputComponent {
    fn update_ghost_text(&mut self, suggestion: Suggestion) {
        if self.input.starts_with(&suggestion.prefix) {
            self.ghost_text = Some(GhostText::new(suggestion));
        } else {
            self.ghost_text = None;
        }
    }
}
```

---

### 3.3 Context Tree

**Files to modify:**
- `crates/runie-tui/src/components/context_tree.rs` (new)
- `crates/runie-core/src/context/mod.rs`

**Implementation:**

```rust
// crates/runie-tui/src/components/context_tree.rs
pub struct ContextTree {
    pub project: ProjectInfo,
    pub dependencies: Vec<Dependency>,
    pub files: Vec<FileInfo>,
    pub patterns: Vec<Pattern>,
    pub git: GitInfo,
}

impl ContextTree {
    pub fn render(&self, f: &mut Frame) {
        // Render collapsible tree
        // Show what Runie knows about the context
    }
}
```

**UI:**
```
┌─────────────────────────────────────────────────────────────┐
│ Project: api-server (Cargo)                                 │
│ ├── Dependencies                                           │
│ │   ├── tokio (async runtime)                             │
│ │   ├── serde (serialization)                             │
│ │   └── tracing (logging)                                 │
│ ├── Files (12 loaded)                                      │
│ │   ├── src/main.rs                                       │
│ │   ├── src/handlers/user.rs                              │
│ │   └── src/models/mod.rs                                 │
│ ├── Patterns (learned)                                      │
│ │   ├── Error handling: Result<T, Error> pattern          │
│ │   ├── Async: .await on all async calls                 │
│ │   └── Logging: tracing::info! for significant events   │
│ └── Git (last 5 commits)                                   │
│     ├── abc1234: Add user authentication                  │
│     └── def5678: Refactor database layer                  │
│                                                              │
│ [r] Refresh  [f] Focus  [d] Dump context  [?] Help       │
└─────────────────────────────────────────────────────────────┘
```

---

### 3.4 Pipe Mode

**Files to modify:**
- `crates/runie-cli/src/commands/pipe.rs` (new)
- `crates/runie-core/src/io/stdin.rs`

**Implementation:**

```bash
# Examples:
echo '\d{4}-\d{2}-\d{2}' | runie --explain-regex
pbpaste | runie --suggest-improvements
git diff --staged | runie --commit-message
cargo test 2>&1 | runie --explain-error
```

---

### 3.5 Headless Mode

**Files to modify:**
- `crates/runie-cli/src/commands/headless.rs` (new)
- `crates/runie-core/src/actors/turn.rs`

**Implementation:**

```bash
# Non-interactive mode
runie --non-interactive "optimize deploy.yml"

# JSON output for scripting
runie --json "suggest improvements" | jq '.suggestions[]'

# Exit codes as contracts
# 0 = success
# 1 = error
# 2 = user cancelled
# 3 = partial success
```

---

## Phase 4: Specialization

### 4.1 Ollama Integration

**Files to modify:**
- `crates/runie-provider/src/ollama.rs` (new)
- `crates/runie-core/src/config/provider.rs`

**Implementation:**

```rust
// crates/runie-provider/src/ollama.rs
pub struct OllamaProvider {
    base_url: Url,
    client: Client,
}

impl OllamaProvider {
    pub async fn complete(&self, request: Request) -> Result<Response> {
        // Call local Ollama API
        // Handle streaming
    }

    pub fn models(&self) -> Vec<Model> {
        // List available Ollama models
    }
}
```

---

### 4.2 MCP Server

**Files to modify:**
- `crates/runie-core/src/mcp/server.rs` (new)
- `crates/runie-core/src/mcp/client.rs` (new)

**Implementation:**

```rust
// crates/runie-core/src/mcp/server.rs
pub struct MCPServer {
    tools: Vec<Tool>,
    handlers: HashMap<String, ToolHandler>,
}

impl MCPServer {
    pub async fn handle_request(&self, req: Request) -> Result<Response> {
        match req.method.as_str() {
            "tools/list" => self.list_tools(),
            "tools/call" => self.call_tool(req.params).await,
            _ => Err(Error::method_not_found()),
        }
    }
}
```

---

### 4.3 Multi-Repo Context

**Files to modify:**
- `crates/runie-core/src/session/multi_repo.rs` (new)
- `crates/runie-tui/src/components/repo_switcher.rs` (new)

**Implementation:**

```rust
// crates/runie-core/src/session/multi_repo.rs
pub struct MultiRepoContext {
    pub repos: Vec<RepoContext>,
    pub active: RepoId,
}

impl MultiRepoContext {
    pub fn switch_to(&mut self, repo_id: RepoId) {
        // Save current state
        // Load new repo context
    }

    pub fn recent_projects(&self) -> Vec<RecentProject> {
        // Return projects with activity info
    }
}
```

---

## Testing Plan

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_vim_key_parsing() {
        assert_eq!(VimKey::from_str("j"), Some(VimKey::Down));
        assert_eq!(VimKey::from_str("gg"), Some(VimKey::First));
        assert_eq!(VimKey::from_str("G"), Some(VimKey::Last));
    }

    #[test]
    fn test_secret_detection() {
        let detector = SecretDetector::new();
        let result = detector.scan("const API_KEY = 'sk-1234567890abcdef'");
        assert!(!result.is_empty());
        assert_eq!(result[0].pattern_name, "OpenAI API Key");
    }

    #[test]
    fn test_confidence_from_score() {
        assert_eq!(Confidence::from_score(0.9), Confidence::High);
        assert_eq!(Confidence::from_score(0.6), Confidence::Medium);
        assert_eq!(Confidence::from_score(0.3), Confidence::Low);
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_diff_first_workflow() -> Result<()> {
    AppTest::mock()
        .start().await?
        .type_text("refactor this function").await?
        .press(ENTER).await?
        .expect_text("REVIEW CHANGES").await?
        .press(ENTER)  // Accept all
        .await?
        .expect_text("Changes applied").await?;
    Ok(())
}

#[tokio::test]
async fn test_escape_returns_to_normal() -> Result<()> {
    AppTest::mock()
        .start().await?
        .press(COLON)  // Open command palette
        .await?
        .expect_text(":").await?
        .press(ESC)    // Should return to normal
        .await?
        .ensure_normal_mode().await?;
    Ok(())
}
```

---

## Rollout Strategy

### 1. Feature Flags

```rust
// Config options for gradual rollout
[features]
diff_first = true
ghost_text = false  // Enable after diff_first stable
context_tree = false
```

### 2. Migration Path

For users upgrading:
1. Show changelog of new behaviors
2. Allow opting into legacy behavior temporarily
3. Deprecate legacy after 2 releases

---

## Success Criteria

### Phase 1 (Foundation)
- [ ] Diff preview shown for all file changes
- [ ] Full vim-style navigation working
- [ ] Single Esc returns to safe state from any mode
- [ ] Status bar shows all 4 required elements
- [ ] Command palette fuzzy search functional

### Phase 2 (Trust)
- [ ] Context preview shown before every API call
- [ ] Secret detection finds 90%+ of common secrets
- [ ] Confidence indicators visible on suggestions
- [ ] Audit log records all interactions
- [ ] Export to CSV/JSON functional

### Phase 3 (Power)
- [ ] Learn mode improves suggestions over time
- [ ] Ghost-text doesn't interfere with typing
- [ ] Context tree accurately shows AI's understanding
- [ ] Pipe mode works with standard Unix tools
- [ ] Headless mode functional for CI

### Phase 4 (Specialization)
- [ ] Ollama integration works with local models
- [ ] MCP server exposes Runie tools
- [ ] Multi-repo context switching preserves state

---

## Open Questions

1. **Granularity of diff acceptance** — Hunk-level, line-level, or file-level?
2. **Ghost-text timing** — Appear after pause? Immediately? User-configurable?
3. **Learn mode storage** — Per-project or global? Encrypted?
4. **Audit log retention** — 30 days? 90 days? Configurable?
5. **Context preview scope** — Always shown? Only for large contexts? Configurable?

---

*Document Version: 1.0*
*Created: 2026-07-15*
*Branch: cognitive*
*Based on: persona_synthesis.md*
