# Archived ADRs

The ADRs in this directory were written for the original actor-based
architecture, in which `runie-core` had an `EventBus` and dedicated actors
for config, queue, session, etc. None of that code is in the runtime
anymore — see [`../../SPEC.md`](../../SPEC.md) for what actually shipped.

Kept for historical reference: understanding why certain APIs are shaped
the way they are, and what was considered and rejected.
