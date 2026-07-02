# Remove or make real the `ProviderProtocol` abstraction

## Status

`todo`

## Description

`ProviderProtocol` trait is defined but not used polymorphically; all production providers are forced through `build_openai_provider`. Either delete the trait or make `OpenAiProvider` generic over it.

## Acceptance criteria

1. **Unit tests** — The trait is either removed or exercised by at least two protocol implementations.
2. **E2E tests** — Provider factory builds and replays work unchanged.
3. **Live tmux tests** — Switch between OpenAI-compatible providers in tmux and confirm both work.

## Tests

### Unit tests
- Factory builds providers without dead abstraction.

### E2E tests
- Replay turn through refactored factory.

### Live tmux tests
- Change provider in settings and run a turn.
