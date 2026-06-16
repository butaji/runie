# Fix CI and Scripts Referencing runie-term

**Status**: todo
**Milestone**: R3
**Category**: Configuration
**Priority**: P0

**Depends on**: delete-runie-term-archive
**Blocks**: (none)

## Description

CI and audit scripts still run `cargo build -p runie-term` and `cargo test -p runie-term --test e2e`, but `runie-term` is not a workspace member and the code now lives in `runie-tui`.

## Acceptance Criteria

- [ ] `.github/workflows/ci.yml` builds/tests `runie-tui` instead of `runie-term`.
- [ ] `scripts/ui-deep-audit.sh` references `runie-tui`.
- [ ] `scripts/ux-audit.sh` references `runie-tui`.
- [ ] Any `--test e2e` target exists in `runie-tui` or is removed.
- [ ] CI green.

## Tests

### Layer 1 — State/Logic
- [ ] `ci_yaml_parses` — `.github/workflows/ci.yml` is valid YAML and mentions only workspace members.

### Layer 4 — Smoke
- [ ] `audit_scripts_run` — `scripts/ui-deep-audit.sh` and `scripts/ux-audit.sh` execute without `cargo` package-not-found errors.

## Files touched

- `.github/workflows/ci.yml`
- `scripts/ui-deep-audit.sh`
- `scripts/ux-audit.sh`

## Notes

Do not re-add `runie-term` to workspace members; it is intentionally archived.
