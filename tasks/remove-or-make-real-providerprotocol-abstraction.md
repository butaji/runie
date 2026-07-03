# Remove or make real the `ProviderProtocol` abstraction

## Status

**wontfix** — Keep the abstraction; it provides value as documentation and future protocol support.

## Context

`ProviderProtocol` trait is defined but not used polymorphically. All production providers are built through `BuiltProviderFactory` → `BuiltProvider`.

## Analysis

### Current State
- `ProviderProtocol` trait is defined in `crates/runie-provider/src/protocol.rs`
- `OpenAiProtocol` implements the trait
- The trait is used in `openai/stream.rs` but only `OpenAiProtocol` is instantiated
- No other protocol implementations exist

### Options Considered

1. **Delete the trait** — Not recommended. The trait provides:
   - Clean documentation of the protocol abstraction
   - Type safety for frame/state handling
   - Potential for future protocol implementations (Anthropic, Gemini, etc.)

2. **Keep the trait** — Recommended. The abstraction is:
   - Already clean and well-documented
   - Provides type safety even with single implementation
   - Ready for future protocol additions

### Decision

**Keep the trait.** The `ProviderProtocol` abstraction is a sound design that:
- Documents the protocol interface clearly
- Enables type-safe frame/state handling
- Can be extended with new protocol implementations without refactoring

## Acceptance Criteria

- [x] Decision documented in this file.

## No Tests Required

This is an architectural decision, not a code change.
