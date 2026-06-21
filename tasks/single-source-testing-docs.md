# Single-source testing-strategy documentation

**Status**: todo
**Milestone**: R4
**Category**: Configuration
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

The 4-layer testing strategy (L1 state, L2 event handling, L3 TestBackend, L4 provider replay) is written out in full in `AGENTS.md` and `docs/Architecture.md`, summarized in `docs/UI_UX.md`, and echoed in `README.md`. The config TOML example also appears in both `README.md` and `docs/Architecture.md`. These sections drift out of sync.

## Acceptance Criteria

- [ ] The full testing-strategy description lives only in `AGENTS.md`.
- [ ] `Architecture.md`, `UI_UX.md`, and `README.md` link to `AGENTS.md` instead of duplicating the full text.
- [ ] The config example lives in one documentation file (suggest `docs/Configuration.md`) and is linked elsewhere.
- [ ] No significant duplicated documentation sections remain.

## Tests

### Layer 1 — State/Logic
- [ ] N/A.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `AGENTS.md`
- `docs/Architecture.md`
- `docs/UI_UX.md`
- `README.md`
- Possibly new `docs/Configuration.md`

## Notes

This is a documentation-only task. Keep anchors stable so external links do not break.
