# Permission Rulesets + Read-Only Tool Classification

**Status**: todo
**Milestone**: R3
**Category**: Safety & Trust
**Priority**: P1

**Depends on**: tool-registry-trait
**Blocks**: (enables `r2-safety-commands.md`)

## Description

Runie currently has a `TrustManager` and a global read-only mode. Research from
Goose (`AllowOnce/AlwaysAllow/AlwaysDeny`), thClaws (`PermissionMode` +
`ApprovalSink`), OpenCode (wildcard rulesets), and OpenHarness (read-only vs
mutating classification) shows that a richer ruleset model is needed.

This task adds wildcard permission rules evaluated last-match, an `ApprovalSink`
trait for UI/test/headless modes, and read-only tool auto-approval.

## Acceptance Criteria

- [ ] `crates/runie-core/src/permissions.rs` defines:
  ```rust
  pub enum PermissionAction { Allow, Ask, Deny }
  pub struct PermissionRule {
      pub tool_pattern: String, // glob, e.g. "read_*" or "bash"
      pub path_pattern: Option<String>, // glob for file paths
      pub action: PermissionAction,
  }
  pub struct PermissionSet { rules: Vec<PermissionRule> }
  impl PermissionSet {
      pub fn evaluate(&self, tool: &str, path: Option<&str>) -> PermissionAction;
  }
  ```
- [ ] `ApprovalSink` trait:
  ```rust
  #[async_trait]
  pub trait ApprovalSink: Send + Sync {
      async fn ask(&self, tool: &str, input: &Value) -> PermissionAction;
  }
  ```
  Implementations:
  - `TuiApprovalSink` — opens modal dialog
  - `AutoAllowSink` — always returns `Allow`
  - `ScriptedSink` — for tests
- [ ] Read-only tools (`is_read_only() == true`) are auto-allowed when the
  global read-only mode is on.
- [ ] Hard-coded denylist for sensitive paths (`~/.ssh/*`, `~/.aws/credentials`,
  `**/.env`).
- [ ] Permission decisions are persisted as durable `PermissionGranted` /
  `PermissionDenied` events.
- [ ] `cargo build --workspace` succeeds.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `wildcard_rule_matches_tool` — `bash` matches `bash`, `b*`, `*`.
- [ ] `path_rule_matches_file` — `read_file` on `src/main.rs` matches
  `src/**`.
- [ ] `last_rule_wins` — later rule overrides earlier.
- [ ] `sensitive_path_denied` — `~/.ssh/id_rsa` is denied regardless of rules.
- [ ] `read_only_tool_auto_allowed_in_read_only_mode` — no prompt.

### Layer 2 — Event Handling
- [ ] `tool_asked_emits_permission_event` — bus receives
  `PermissionRequested`.
- [ ] `permission_granted_is_persisted` — durable event written.

### Layer 3 — Rendering
- [ ] `permission_modal_renders_tool_and_input` — TUI shows tool name and
  arguments.

## Notes

**Files touched:**
- `crates/runie-core/src/permissions.rs` (rewrite)
- `crates/runie-core/src/trust.rs` (merge or replace)
- `crates/runie-agent/src/turn.rs` (check permissions before tool call)
- `crates/runie-tui/src/popups.rs` (permission modal)

**Out of scope:**
- Time-based permission expiration.
- Per-project permission inheritance (can be layered later).
