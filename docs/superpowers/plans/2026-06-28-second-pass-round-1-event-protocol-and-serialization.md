# Round 1 — Event Protocol & Serialization

## Findings

### 1. Hand-written event taxonomies

- `crates/runie-core/src/event/mod.rs:355-577` — `Event::kind()` is a ~220-line manual `match`.
- `crates/runie-core/src/event/mod.rs:580-798` — `Event::category()` repeats the same pattern for `EventCategory`.
- `crates/runie-core/src/event/mod.rs:1039-1141` — `EVENT_NAMES` duplicates every zero-argument variant as a `(name, ctor)` pair.

These are SSOT violations: adding a variant requires touching the enum, `kind()`, `category()`, and `EVENT_NAMES`. `strum::VariantNames`/`IntoStaticStr` plus a category attribute can derive all of this.

### 2. `Intent` duplicates `Event` intent/control variants

- `crates/runie-core/src/event/intent.rs:50-432` — `Intent` enum mirrors many `Event` variants.
- `crates/runie-core/src/event/mod.rs:803-958` — `Event::into_intent()` maps every variant by hand.

Consider a single `#[derive(Intent)]` proc macro, or at least shared struct payloads so `RunLoadCommand { name }` is one type.

### 3. Durable and headless event types duplicate canonical types

- `crates/runie-core/src/event/durable.rs:333-371` — `DurableCoreEvent` re-declares `MessageSent`, `ToolCalled`, `ToolResult`, etc.
- `crates/runie-core/src/event/headless.rs:19-60` — `HeadlessEvent` duplicates `ProviderEvent` stream vocabulary.

Both should be derived from the canonical `Event`/`ProviderEvent` vocabularies with `#[serde(skip)]` and thin wrappers.

### 4. Builder / validation boilerplate

- `crates/runie-core/src/proto/message/mod.rs:267-382` — `ChatMessageBuilder` is hand-written although `derive_builder` is already used elsewhere.
- `crates/runie-core/src/proto/message/mod.rs:71-89` — `Role` derives `strum::Display`/`EnumString` but also hand-implements `as_str()`/`parse()`.
- `crates/runie-core/src/provider_event.rs:95-261` — `ModelError` carries custom trait impls just because `anyhow::Error` is not `Clone`/`Serialize`. Storing `message: String` would allow deriving everything.
- `crates/runie-core/src/sanitize.rs:14-32` — `sanitize_messages` delegates to a second copy of validation logic in `proto/message/mod.rs`.

## Recommended changes

1. Replace `Event::kind()`, `Event::category()`, and `EVENT_NAMES` with `strum` derives or a single category attribute.
2. Collapse `Intent`/`Event::into_intent()` into shared payloads or a derive macro.
3. Store durable events as canonical `Event`/`ChatMessage`/`ToolCall` types with `#[serde(skip)]` transient fields.
4. Derive `ChatMessageBuilder` and remove manual `Role` serialization helpers.
5. Store `message: String` in `ModelError::Other` and derive `Clone`/`Serialize`/`Deserialize`.
6. Consolidate message validation in one module.

## Task mapping

| Finding | Task file | Status |
|---------|-----------|--------|
| Derive event taxonomies | `tasks/derive-event-taxonomies-with-strum-or-proc-macro.md` | **new** |
| Collapse `Intent` into shared payloads | `tasks/collapse-intent-into-shared-event-payloads.md` | **new** |
| Durable/headless events from canonical types | `tasks/collapse-durablecoreevent-into-event-enum.md` | existing `todo` |
| Derive `ChatMessageBuilder` | `tasks/derive-chatmessage-builder-with-derive-builder.md` | **new** |
| Simplify `ModelError` serde | `tasks/simplify-modelerror-serde-by-storing-message.md` | **new** |
| Consolidate message validation | `tasks/consolidate-message-validation-in-one-module.md` | **new** |
