# Auto-approve mode

## Objective

Add a two-level permission mode cycle: `manual` (default) and `auto` (approve regular tools automatically). Surface the current mode in the status bar.

## Agent landscape finding

codex has `/permissions`, goose has `/mode auto`, gemini has `Ctrl+Y`, kimi has `--yolo`. Runie has read-only/trust but no explicit auto-approve toggle.

## runie current state

Runie supports read-only mode and project trust. Tool approvals are always manual except for pre-approved operations.

## Required runie changes

- Add a permission mode: `manual` and `auto`.
- `auto` approves read, edit, and shell tools automatically; still requires confirmation for plan exit and other high-risk actions.
- Untrusted projects force `manual` mode.
- Add `/auto` slash command or include mode in `/settings` to toggle.
- Show current mode in the status bar.

## Test scenarios

1. **Auto mode approves read tool**
   - Keys: enable auto mode, send a prompt that triggers a read tool.
   - Assert: tool runs without permission dialog.

2. **Manual mode shows permission dialog**
   - Keys: ensure manual mode, send same prompt.
   - Assert: permission dialog appears.

3. **Untrusted project stays manual**
   - Keys: start in untrusted project, try to enable auto.
   - Assert: mode remains manual; inline error shown.

## Edge / negative cases

- Mode persists for the session but not across restarts by default.
- Switching to auto shows a confirmation dialog explaining the risk.

## Dependencies

- `tool_permissions`
- `status-command`

## Acceptance checklist

- [x] All scenarios pass with `AppTest::mock()` or replay fixtures.
      (`tests/auto_approve_mode.rs`: palette item, toggle + badge, manual
      dialog, auto skips dialog, `/auto off`, untrusted refusal, restart reset.)
- [x] Edge cases are covered. (Untrusted guard, session-only mode; the
      risk-confirmation dialog is intentionally out of scope.)
- [x] No `sleep()` in resulting Rust tests.

## Status

Implemented in runie: `/auto` toggles a session-scoped permission mode
(`PermissionActor` holds the mode; gate construction in
`runie-agent/src/actor/handlers.rs::policies_for_mode` prepends the
`AutoApprove` policy in Auto mode). Status bar shows `⚡ Auto` while enabled.
Untrusted projects refuse with an inline message. Mode never persists to
config, so restarts return to manual.
