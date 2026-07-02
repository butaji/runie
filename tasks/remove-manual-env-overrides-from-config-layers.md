# Remove manual env overrides from config layers

## Status

`todo`

## Description

`config/layers.rs` uses `figment` but then manually re-implements env overrides for `RUNIE_PROVIDER`, `RUNIE_MODEL`, `RUNIE_THEME`. Rely on `figment::providers::Env` or document why it is insufficient.

## Acceptance criteria

1. **Unit tests** — Env vars still override config values through Figment.
2. **E2E tests** — Loading config with env overrides produces the same result.
3. **Live tmux tests** — Set `RUNIE_PROVIDER`/`RUNIE_MODEL` and launch tmux; verify the overrides apply.

## Tests

### Unit tests
- Figment env override precedence.

### E2E tests
- Config load with env overrides.

### Live tmux tests
- Launch with env overrides and check provider/model.
