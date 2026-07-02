# Centralize `reqwest` client and URL normalization

## Status

`todo`

## Description

`model_client.rs`, `openai/mod.rs`, and `lib.rs` each build `reqwest::Client`s and normalize URLs/keys. Centralize these in one `runie-provider::http` module.

## Acceptance criteria

1. **Unit tests** — All provider creation paths use the centralized helper; trailing slashes and key trimming are consistent.
2. **E2E tests** — Replay turns still construct providers correctly.
3. **Live tmux tests** — Configure a provider with a trailing-slash URL and odd key whitespace; confirm it works.

## Tests

### Unit tests
- URL normalization and key trimming cases.

### E2E tests
- Factory builds provider from fixture config.

### Live tmux tests
- Edit provider URL/key in tmux settings.
