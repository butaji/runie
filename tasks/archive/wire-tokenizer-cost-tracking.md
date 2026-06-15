# Wire Tokenizer and Cost Metadata Into Token Counting

**Status**: done
**Milestone**: R3
**Category**: Core / State
**Priority**: P1

## Description

The provider/model registry now carried tokenizer and cost metadata, but production code still used `chars/4` for token estimation and `TokenTracker` was never initialized from `ModelMeta`.

## Acceptance Criteria

- [x] `estimate_tokens_for_model` selects the tokenizer based on the active model.
- [x] `TokenTracker` is initialized with the selected model’s prompt/completion costs.
- [x] Fallback to `chars/4` remains for unknown models.
- [x] Costs displayed in the UI reflect actual usage.

## Tests

### Layer 1 — State/Logic
- [x] `token_tracker_uses_registry_costs`.
- [x] `estimate_tokens_selects_model_tokenizer`.
- [x] `unknown_model_falls_back_to_approximation`.
