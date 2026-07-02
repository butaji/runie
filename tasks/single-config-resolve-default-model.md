# Single `Config::resolve_default_model`

## Status

`todo`

## Description

Default model/provider resolution exists in `config_impl.rs`, `session.rs`, and `domain_ops.rs`. Provide a single `Config::resolve_default_model()` used everywhere.

## Acceptance criteria

1. **Unit tests** — All fallback branches are covered by the single resolver.
2. **E2E tests** — `ConfigState::default` and bootstrap produce the same model selection.
3. **Live tmux tests** — Launch tmux with no explicit model and verify a sensible default is selected.

## Tests

### Unit tests
- Resolver handles mock/test/normal modes.

### E2E tests
- Bootstrap event produces expected default model.

### Live tmux tests
- Clear config and launch; check default provider/model.
