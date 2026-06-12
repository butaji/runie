# Replace Closed AnyProvider Enum With Trait-Object Dispatch

**Status**: todo
**Milestone**: R1
**Category**: Core Architecture
**Priority**: P0
**Depends on**: wire-orphan-crates

## Description

`crates/runie-provider/src/lib.rs:19-24` defines a closed `AnyProvider`
enum with only two variants (`Mock`, `OpenAi`). The provider registry in
`crates/runie-core/src/provider_registry.rs` lists 13 providers
(anthropic, openai, google, deepseek, openrouter, groq, mistral, fireworks,
together, minimax, moonshotai, xai, ollama). When the user selects a
non-OpenAI provider, the `_` arm in `AnyProvider::build_with_config`
silently falls back to `Mock` (line 70-90) — the worst kind of bug: a
visible "model picker" with hidden Mock behavior.

The root cause is that `Provider::generate` is `async fn in trait` with
`#[allow(async_fn_in_trait)]`, which makes the trait *not*
dyn-compatible. This forces a closed enum.

The fix: split the trait into a sync metadata trait + a future-returning
factory, OR add a `Box<dyn Provider>`-friendly wrapper that erases the
async.

## Acceptance Criteria

- [ ] A new `Provider` shape exists that is dyn-compatible (no `async fn` in the trait body)
- [ ] The closed `AnyProvider` enum is removed
- [ ] `AppState::switch_model(provider, model)` correctly instantiates the requested provider or returns a clear `Err` (no silent Mock fallback)
- [ ] A new provider can be added by (a) implementing the trait and (b) adding one entry to the registry — no enum edit required
- [ ] The `OpenAiProvider` is reused for all OpenAI-Chat-Completions-compatible providers (anthropic, openai, google, deepseek, openrouter, groq, mistral, fireworks, together, minimax, moonshotai, xai, ollama) — no per-provider code
- [ ] The `validate_api_key` function uses the same dispatch path (no special-cased OpenAI logic)
- [ ] A `Mock` provider still exists for `RUNIE_MOCK=1` dev mode
- [ ] A unit test verifies that selecting an unknown provider key returns `Err(UnknownProvider)` rather than a Mock

## Tests

### Layer 1 — State/Logic
- [ ] `test_provider_dispatch_by_key` — given a registry with `openai` and `mock`, `build("openai", "gpt-4o")` returns an `OpenAi` provider, `build("mock", "echo")` returns a `Mock` provider
- [ ] `test_unknown_provider_returns_error` — `build("nonexistent", "...")` returns `Err`, not a silent Mock
- [ ] `test_openai_compatible_providers_share_implementation` — `build("anthropic", "...")`, `build("minimax", "...")`, `build("ollama", "...")` all return an `OpenAiCompatible` provider (the same struct as `OpenAiProvider` parameterized by `base_url`)
- [ ] `test_registry_lists_all_openai_compatible_providers` — the registry exposes `is_openai_compatible(key) -> bool` and 12 of 13 providers return `true`

### Layer 2 — Event Handling
- [ ] `test_switch_model_to_unknown_provider_emits_transient_error` — feeding `Event::SwitchModel { provider: "garbage", model: "x" }` into `AppState::update` produces a `TransientError` event and does NOT silently switch to Mock
- [ ] `test_login_validation_failure_does_not_silently_use_mock` — when the user enters an API key for an unknown provider, the login flow errors clearly

### Layer 3 — Rendering
- [ ] N/A (no rendering changes)

### Layer 4 — Smoke
- [ ] A tmux script that types `/model minimax/MiniMax-M3` and verifies the spinner appears (not a silent Mock response)

## Notes

**The async-fn-in-trait problem and its solution:**

```rust
// Before — not dyn-compatible
#[allow(async_fn_in_trait)]
pub trait Provider: Send + Sync {
    async fn generate<F>(&self, ...) -> Result<()> where F: FnMut(ResponseChunk) + Send;
}

// After — dyn-compatible via `Pin<Box<dyn Future>>`
pub trait Provider: Send + Sync {
    fn generate<'a, F>(&self, messages: Vec<Message>, on_chunk: F) 
        -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>
    where F: FnMut(ResponseChunk) + Send + 'a;
}
```

Alternative: a sync factory that returns a `Future`:
```rust
pub trait Provider: Send + Sync {
    fn generate<'a, F>(&'a self, ...) -> impl Future<Output = ...> + 'a;
}
```
This is `impl Trait` in trait (RPITIT) and *is* dyn-compatible in modern
Rust, but only with the `trait_alias` or `dyn_compatible` flags. Verify
against the workspace's MSRV (`edition = "2021"` implies Rust 1.56+,
RPITIT needs 1.75+).

**OpenAI-compat deduplication:** all 12 non-Mock providers use the
OpenAI Chat Completions API. The `OpenAiProvider` already takes
`api_key`, `model`, and `base_url`. A `ProviderFactory` for
`OpenAiCompatible` can be a single function:

```rust
fn openai_compatible(meta: &ProviderMeta, model: &str) -> Box<dyn Provider> {
    let api_key = std::env::var(meta.env_var).unwrap_or_default();
    let p = OpenAiProvider::new(api_key, model).with_base_url(meta.base_url);
    Box::new(p)
}
```

**Out of scope:**
- Adding new providers (anthropic, google, etc.) — those are wired
  through the registry once this task lands
- Streaming the response body in chunks other than the existing SSE parse
- Per-provider retry, rate-limit handling, or auth-refresh

**Verification:**
```bash
# Build clean
cargo build --workspace

# All provider tests pass
cargo test -p runie-provider

# Smoke: model picker lists all 13, /model anthropic/... doesn't silently mock
```
