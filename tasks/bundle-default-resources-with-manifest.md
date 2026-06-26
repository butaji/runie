# Bundle default resources with checksum manifest

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Priority**: P2

**Depends on**: built-in-subagent-types
**Blocks**: none

## Summary

Ship built-in agents, roles, personas, and skills as resource files embedded in the binary. A manifest with SHA-256 checksums validates bundled assets at build time and runtime.

## Layout

```text
resources/
  agents/
    explore.md
    plan.md
    general-purpose.md
  roles/
    implementer.toml
    reviewer.toml
  personas/
    implementer.toml
  skills/
    check-work/SKILL.md
  manifest.json
```

Manifest example:

```json
{
  "version": "r1",
  "checksums": {
    "agents/explore.md": "sha256:...",
    "roles/implementer.toml": "sha256:..."
  }
}
```

## Acceptance Criteria

- Default resources live under `crates/runie-core/resources/`.
- Build script computes checksums and embeds `manifest.json`.
- Runtime verifies bundled resources against the manifest on load.
- User overrides in `~/.runie/` take precedence over bundled defaults.
- `cargo check --workspace` is green.

## Tests

- **Layer 1**: Manifest generation and checksum verification.
- **Layer 2**: Load overrides correctly shadow bundled resources.
