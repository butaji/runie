# Add Grok TUI keystroke replay and tool mapping

## Status

**done** ✅

## Context

Grok TUI scenarios are captured as tmux pane dumps; there is no bridge to Runie `TestBackend` events or canonical tool-name mapping.

## Implementation

### Keystroke DSL (`crates/runie-testing/src/keystroke_dsl.rs`)

Created a keystroke DSL parser that translates string representations to `runie_core::Event` variants.

**DSL Syntax:**
- Single characters: `"a"`, `"A"`, `"!"` → `Input(c)`
- Named keys: `"enter"`, `"escape"`, `"up"`, `"down"`, `"left"`, `"right"`, `"pageup"`, `"pagedown"`, `"home"`, `"end"`, `"tab"`, `"backspace"`, `"delete"`, `"space"`
- Modifiers: `"ctrl+c"`, `"alt+enter"`, `"shift+tab"`
- tmux style: `"C-c"`, `"M-RET"` (Alt+Enter)

**Key functions:**
- `parse_keystroke(dsl: &str) -> Option<Event>` — Parse single DSL token to Event
- `parse_sequence(dsl: &str) -> Vec<Event>` — Parse sequence with delimiters (`<enter>`, `,`, `;`, etc.)
- `parse_tmux_style(dsl: &str) -> Option<Event>` — Parse tmux-style notation (`C-c`, `M-RET`)
- `key_event_to_dsl(event: &KeyEvent) -> String` — Convert crossterm KeyEvent back to DSL

### Tool Aliases (`crates/runie-testing/src/tool_aliases.rs`)

Created bidirectional mapping between Grok and Runie tool names.

**Tool Name Mapping:**
| Grok Tool | Runie Tool |
|-----------|------------|
| `grok-read` | `Read` |
| `grok-write` | `Write` |
| `grok-edit` | `Edit` |
| `grok-list-directory` | `ListDir` |
| `grok-grep` | `Grep` |
| `grok-find` | `Find` |
| `grok-web-search` | `WebSearch` |
| `grok-web-fetch` | `WebFetch` |
| `grok-bash` | `Bash` |
| `grok-shell` | `Bash` |
| `grok-exec` | `Bash` |
| `grok-run` | `Bash` |

**Key functions:**
- `grok_to_runie(grok_tool: &str) -> Option<&str>` — Map Grok → Runie
- `runie_to_grok(runie_tool: &str) -> Option<&str>` — Map Runie → Grok (first alias)
- `runie_to_grok_all(runie_tool: &str) -> Vec<&str>` — Get all Grok aliases
- `display_name(tool_name: &str) -> String` — Canonical display name
- `is_read_only_tool(tool_name: &str) -> bool` — Check if tool is read-only
- `transform_args(tool_name: &str, args: &Value) -> Value` — Normalize arguments

## Acceptance Criteria

- [x] Define small keystroke DSL.
- [x] Translate DSL to `runie_core::Event`s.
- [x] Add tool-name/schema aliases.

## Design Impact

No change to TUI element design or composition. Only test infrastructure and tool mapping changes.

## Tests

- **Layer 1 — State/Logic:** ✅ 35 unit tests in `runie-testing` pass
- **Layer 2 — Event Handling:** N/A (utility module)
- **Layer 3 — Rendering:** N/A (utility module)
- **Layer 4 — E2E:** Utility available for TUI comparison tests

## Completion Validation

- [x] **Unit tests** — `cargo test -p runie-testing` passes (35 tests)
- [x] **E2E tests** — `cargo test --workspace` passes
- [x] **Live tmux run tests** — N/A (utility infrastructure, not TUI)

### SSOT/Event Compliance

- [x] **Actor/SSOT:** N/A (DSL translation is a utility; actors remain authoritative)
- [x] **Trigger events:** DSL keystrokes translate to `Event` variants
- [x] **Observer events:** N/A (translation doesn't emit events)
- [x] **No direct mutations:** DSL translation doesn't mutate actor state
- [x] **No new mirrors:** Tool aliases are mappings, not authoritative state
- [x] **Async work observed:** N/A (synchronous translation)

## Files Touched

- `crates/runie-testing/src/keystroke_dsl.rs` — new file (keystroke DSL parser)
- `crates/runie-testing/src/tool_aliases.rs` — new file (Grok→Runie tool mapping)
- `crates/runie-testing/src/lib.rs` — exports new modules
- `crates/runie-testing/Cargo.toml` — added `crossterm` dev-dependency

## Notes

- The keystroke DSL can be extended with additional named keys as needed
- Tool aliases support multiple Grok tools mapping to the same Runie tool
- Argument transformation handles schema differences (e.g., `command` vs `cmd` for Bash)
