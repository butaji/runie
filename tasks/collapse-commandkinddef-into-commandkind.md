# Collapse `CommandKindDef` into `CommandKind`

## Status

`todo`

## Description

`commands/dsl/spec.rs` and `declarative/types.rs` define overlapping `CommandKind` shapes. Deserializing YAML goes through `CommandKindDef` then `CommandKind`. Collapse them and derive `CommandCategory` labels.

## Acceptance criteria

1. **Unit tests** — YAML commands deserialize directly into `CommandKind`; category labels are derived.
2. **E2E tests** — Declarative command loading still works.
3. **Live tmux tests** — A YAML-defined command appears and runs in tmux.

## Tests

### Unit tests
- YAML -> `CommandKind` deserialization.

### E2E tests
- Load declarative commands and execute via replay.

### Live tmux tests
- Add a YAML command and invoke it from the palette.
