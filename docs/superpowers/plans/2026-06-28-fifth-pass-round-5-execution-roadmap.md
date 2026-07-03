# Round 5 — Pre-Implementation Roadmap

## Stopping rule

This is the **final architecture review round**. The backlog now contains enough actionable tasks across five passes. Further horizontal review will produce diminishing returns. The next work should be implementation.

## Phase 1 — Fix the dual state-machine (week 1)

These are the highest-impact fixes; they eliminate the root cause of drift and feedback loops.

1. `make-turnactor-sole-owner-of-turn-state`
2. `remove-dual-agent-event-application-in-dispatch`
3. `remove-direct-turn-lifecycle-mutations-outside-turnactor`
4. `treat-agentstate-as-pure-turnstate-projection`

## Phase 2 — Make the protocol safe and idempotent (week 2)

1. `remove-unsafe-zeroed-reply-port-in-deliverqueued`
2. `remove-clone-impl-for-messages-with-reply-ports`
3. `serialize-projection-update-after-actor-commit`
4. `add-idempotency-keys-to-turn-events`
5. `make-projection-handlers-idempotent`

## Phase 3 — Add missing facts and remove derived events (week 3)

1. `add-missing-queue-fact-events`
2. `move-derived-values-out-of-events`
3. `remove-derived-values-from-durable-events`
4. `replace-wholestate-sessionchanged-with-fine-grained-events`

## Phase 4 — Crash recovery and contract tests (week 4)

1. `add-turn-journal-phases-for-crash-recovery`
2. `add-contract-tests-for-turnactor-and-sessionstore`

## Keystone decisions before Phase 1

- Confirm `TurnActor` owns turn lifecycle (product/UX impact: none).
- Decide whether to keep `duration_secs` in persisted events or compute it on replay.

## Task mapping

| Finding | Task file | Status |
|---------|-----------|--------|
| Execute roadmap | `tasks/execute-fifth-pass-ssot-event-protocol-roadmap.md` | **new** |
