# Name keyring token preview and build lint lookback

## Status

**done**

## Description

`runie-core/src/auth/keyring.rs:35` hardcodes `8` for token-preview length. `runie-core/build.rs:453` hardcodes `10` for the lookback window when searching for `#[allow(...)]` above `tokio::spawn`.

## Acceptance criteria

1. **Unit tests** — Both literals are named constants; error messages and lint behavior unchanged.
2. **E2E tests** — Keyring mismatch error and orphan-spawn lint still work.
3. **Live tmux tests** — Not applicable; test/utility code.

## Tests

### Unit tests
- Constant values match old behavior.

### E2E tests
- Build script lint catches an orphan spawn with `#[allow(...)]` lookback.

### Live tmux tests
- N/A.
