# Safety Commands

**Status**: todo
**Milestone**: R2
**Category**: Safety

## Description

Safety-related commands: read-only mode toggle and trust system.

## Architecture

```rust
// Command factories

fn cmd_readonly(_args: &str) -> Option<Event> {
    Some(Event::ToggleReadOnly)
}

fn cmd_trust(_args: &str) -> Option<Event> {
    Some(Event::TrustProject)
}

fn cmd_untrust(_args: &str) -> Option<Event> {
    Some(Event::UntrustProject)
}
```

### Read-Only Mode

```rust
pub struct AppState {
    // ... existing fields ...
    pub read_only: bool,
}

fn update_toggle_read_only(state: &mut AppState) -> String {
    state.read_only = !state.read_only;
    let status = if state.read_only { "enabled" } else { "disabled" };
    format!("Read-only mode {}", status)
}
```

When `read_only` is true, only these tools are sent to the LLM:
- `read_file`, `grep`, `find`, `ls`

### Trust System

```rust
pub struct TrustManager {
    decisions: HashMap<PathBuf, TrustDecision>,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum TrustDecision {
    Trusted,
    Untrusted,
}

impl TrustManager {
    pub fn load() -> Self;
    pub fn save(&self) -> Result<()>;
    pub fn decision_for(&self, path: &Path) -> Option<TrustDecision>;
    pub fn set(&mut self, path: &Path, decision: TrustDecision);
}
```

First-run flow:
```
1. runie starts in /path/to/project
2. TrustManager checks if project is known
3. If not known: show prompt "Trust this project? (y/n)"
4. User choice saved to ~/.runie/trust.json
5. Untrusted → read_only = true by default
```

## Acceptance Criteria

- [ ] `/readonly` toggles read-only mode
- [ ] Read-only restricts tool schemas sent to LLM
- [ ] Status bar shows `🔒 RO` when active
- [ ] `/trust` marks current project as trusted
- [ ] `/untrust` marks current project as untrusted
- [ ] First-run prompt for unknown projects
- [ ] Trust decisions persisted in `~/.runie/trust.json`
- [ ] Untrusted projects default to read-only

## Files

| File | Description |
|------|-------------|
| `crates/runie-core/src/model.rs` | `read_only: bool` |
| `crates/runie-core/src/trust.rs` | `TrustManager`, `TrustDecision` |
| `crates/runie-core/src/commands/handlers.rs` | Factory functions |
| `crates/runie-core/src/update/mod.rs` | Toggle handler |
| `crates/runie-agent/src/lib.rs` | Filter tools by read_only |
| `crates/runie-tui/src/ui.rs` | Show `🔒 RO` badge |

## Tests

### Layer 1 — State/Logic
- [ ] `toggle_flips_read_only` — bool inverts
- [ ] `read_only_filters_tools` — write/edit/bash excluded from schemas
- [ ] `trust_manager_loads` — JSON roundtrip
- [ ] `trust_sets_decision` — decision updated
- [ ] `untrusted_defaults_readonly` — untrusted → read_only true

### Layer 2 — Event Handling
- [ ] `slash_readonly_toggles` — event updates state
- [ ] `slash_trust_sets` — event updates TrustManager
