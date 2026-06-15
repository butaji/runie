# Fix Provider/Model Registry Drift and Names

**Status**: done
**Milestone**: R3
**Category**: Configuration
**Priority**: P1

## Description

The unified provider/model registry still had drift:
- OpenRouter mirrors used different names/punctuation and missing capability flags vs. canonical providers.
- Several model identifiers were non-standard or invented (`claude-sonnet-4-6`, `gpt-5`, `o4-mini`, `deepseek-v4-flash`, `grok-4.3`).
- `context_window_for()` in `runie-tui/src/status_bar.rs` duplicated context-window data and listed phantom providers.

## Acceptance Criteria

- [x] OpenRouter mirror models derive from canonical entries or match them exactly.
- [x] Model names align with real provider API identifiers; speculative models removed or clearly marked.
- [x] `ProviderMeta`/`ModelMeta` expose `context_window`.
- [x] `status_bar.rs` uses registry data instead of its own lookup.
- [x] `ProviderApiType` dead abstraction addressed (removed).

## Tests

### Layer 1 — State/Logic
- [x] `openrouter_model_matches_canonical`.
- [x] `context_window_comes_from_registry`.
- [x] `status_bar_context_window_matches_registry`.
- [x] `status_bar_context_window_falls_back_to_default`.
