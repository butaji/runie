# Split large core files into focused modules

## Status

`todo`

## Description

The following production files exceed the advertised 500-line limit and mix responsibilities:

- `crates/runie-core/src/proto/message/mod.rs` — 769 lines
- `crates/runie-core/src/session/sqlite_store.rs` — 674 lines
- `crates/runie-core/src/actors/fff_indexer/mod.rs` — 669 lines
- `crates/runie-core/src/provider/provider_trait.rs` — 650 lines
- `crates/runie-core/src/event/durable.rs` — 648 lines
- `crates/runie-core/src/actors/permission/ractor_permission.rs` — 601 lines
- `crates/runie-core/src/actors/io/ractor_io.rs` — 587 lines
- `crates/runie-core/src/model/state/domain_ops.rs` — 410 lines

## Acceptance criteria

- Each file is split into focused modules.
- No production file exceeds 500 lines.
- Public API remains unchanged.

## Tests

### Layer 1 — State/Logic
- Unit tests for split modules pass.

### Layer 2 — Event Handling
- Event dispatch through split modules passes.
