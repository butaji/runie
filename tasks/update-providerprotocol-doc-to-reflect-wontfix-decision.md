# Update `ProviderProtocol` doc to reflect `wontfix` decision

## Status

`done`

## Description

`tasks/remove-or-make-real-providerprotocol-abstraction.md` is `wontfix` (the abstraction is kept because it provides value as documentation and future protocol support), but `docs/superpowers/plans/2026-06-28-second-pass-round-2-provider-and-retry.md` still lists it as a recommendation to delete. Updated the doc to reflect the decision.

## Changes made

- Updated `docs/superpowers/plans/2026-06-28-second-pass-round-2-provider-and-retry.md`:
  - Changed section 3 from "dead weight" recommendation to "Decision: Keep (wontfix)"
  - Updated the "Recommended changes" section to strike through the delete recommendation

## Acceptance criteria

1. **Unit tests** — N/A; doc task.
2. **E2E tests** — N/A; doc task.
3. **Live tmux tests** — N/A; doc task.

## Completion Validation

- [x] **Doc updated** — `docs/superpowers/plans/2026-06-28-second-pass-round-2-provider-and-retry.md` reflects the `wontfix` decision.
