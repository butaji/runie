# Completion audit

This document separates the finite acceptance boundary from optional parity
extensions. A finite item is complete only when its source, event/replay tests,
and TUI evidence exist. “Richer”, provider-specific, and platform-specific
work is queued separately and must not make the finite boundary ambiguous.

## Verified finite boundary

The twenty rows in [`completion-matrix.md`](completion-matrix.md) are covered
by the corresponding `harness-findings.md` entries, workspace tests, and the
live TUI smoke matrix. The latest live run exercised 124 TUI cases with
`passed=124 failed=0`; all workspace test binaries passed. Formatting, clippy,
structural lint, and
`git diff --check` are green.

## Queued extensions

These are real follow-up improvements, not hidden acceptance failures:

1. Provider/session transport parity: add captured, secret-free wire fixtures
   for remaining provider payload and session-lane variants.
2. Tool/job UX: add typed output-card navigation and richer queued/running
   lifecycle controls, with failure and cancellation replay traces.
3. Context controls: add explicit policy-setting events only after the
   renderer-neutral settings data contract is defined.
4. Session/Git UX: add picker interactions and approval-gated recovery
   mutations without allowing direct cross-actor state changes.
5. Media/diagnostics: add only provider formats and interactive controls that
   have bounded data contracts plus deterministic fixtures.

Each extension follows the same proof loop: failing event/replay test, minimal
actor-owned implementation, focused tests, `just ci`, 124-case live TUI smoke,
finding update, commit, and push.

## Why the previous status was misleading

`harness-findings.md` uses `partial` for both finite rows with complete
acceptance evidence and genuinely unfinished extensions. The finite matrix is
the authoritative acceptance boundary; the findings ledger should retain the
extension note, but must not imply that the finite row lacks its required
implementation or tests.
