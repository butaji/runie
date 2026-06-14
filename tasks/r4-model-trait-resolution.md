# Model Trait Resolution

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Priority**: P0

**Depends on**: r3-model-capability-flags, r4-orchestrator-domain-types
**Blocks**: r4-orchestrator-actor, r4-one-shot-orchestrator-llm

## Description

Build a resolver that maps an abstract **model trait** (e.g. `Reasoning`,
`Code`, `Fast`, `Vision`, `LongContext`) to a concrete configured model. The
Orchestrator uses this to pick the best model for each subtask and synthesis.

## Acceptance Criteria

- [ ] `ModelTrait` enum exists with at least `General`, `Reasoning`, `Code`,
  `Fast`, `Vision`, `LongContext`.
- [ ] Each configured `ModelProfile` declares a `HashSet<ModelTrait>`.
- [ ] `ModelResolver` takes a `Vec<ModelProfile>` and a trait request and returns
  the best match.
- [ ] Matching is deterministic: exact trait match wins; otherwise the model with
  the most matching traits wins; ties broken by an optional global priority list.
- [ ] If no model matches, return an error with a helpful message.
- [ ] Provider preference flags are ignored at this layer (provider selection is
  downstream).

## Tests

### Layer 1 — State / Logic

```rust
#[test]
fn exact_trait_wins() {
    let profiles = vec![
        profile("gpt-4o", &[General, Vision]),
        profile("o3-mini", &[Reasoning]),
    ];
    let resolver = ModelResolver::new(profiles);
    assert_eq!(resolver.resolve(Reasoning).unwrap().id, "o3-mini");
}

#[test]
fn most_matching_traits_wins_on_partial_match() {
    let profiles = vec![
        profile("a", &[General]),
        profile("b", &[General, Code]),
    ];
    let resolver = ModelResolver::new(profiles);
    assert_eq!(resolver.resolve(Code).unwrap().id, "b");
}

#[test]
fn priority_list_breaks_ties() {
    let profiles = vec![
        profile("first", &[Code]),
        profile("second", &[Code]),
    ];
    let resolver = ModelResolver::new(profiles).with_priority(&["first", "second"]);
    assert_eq!(resolver.resolve(Code).unwrap().id, "first");
}
```

## Files touched

- `crates/runie-core/src/model.rs` or new `crates/runie-core/src/trait_resolver.rs`
- `crates/runie-core/src/lib.rs`

## Out of scope

- Cost/routing optimization.
- Streaming provider selection logic.
