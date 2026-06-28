# Bundle default resources with checksum manifest

**Status**: done
**Milestone**: R4
**Category**: Core / State
**Priority**: P2

**Depends on**: built-in-subagent-types
**Blocks**: none

## Summary

Ship built-in agents, roles, personas, and skills as resource files embedded in the binary. A manifest with SHA-256 checksums validates bundled assets at build time and runtime.

## Implementation

The feature is fully implemented:

1. **Resources directory**: `crates/runie-core/resources/` with:
   - `agents/` - subagent type markdown files
   - `models/` - model metadata YAML files

2. **Manifest**: `crates/runie-core/resources/agents/manifest.json` with SHA-256 checksums:
   ```json
   {
     "version": 1,
     "description": "Bundled built-in subagent types...",
     "files": {
       "explore.md": "sha256:...",
       "plan.md": "sha256:...",
       "verify.md": "sha256:...",
       "check-work.md": "sha256:..."
     }
   }
   ```

3. **Build-time validation**: `crates/runie-core/build.rs::validate_agent_manifest()` computes SHA-256 hashes at build time and validates against the manifest.

4. **Runtime loading**: `crates/runie-core/src/subagents/manifest.rs` loads the embedded manifest and provides `check_file()` for validation.

5. **Override precedence**: User overrides in `~/.runie/` take precedence over bundled defaults (handled by `subagents/mod.rs`).

## Acceptance Criteria

- [x] Default resources live under `crates/runie-core/resources/`.
- [x] Build script computes checksums and embeds `manifest.json`.
- [x] Runtime verifies bundled resources against the manifest on load.
- [x] User overrides in `~/.runie/` take precedence over bundled defaults.
- [x] `cargo check --workspace` is green.

## Files

- `crates/runie-core/resources/agents/manifest.json` - checksum manifest
- `crates/runie-core/build.rs` - validates manifest at build time
- `crates/runie-core/src/subagents/manifest.rs` - runtime manifest loading
