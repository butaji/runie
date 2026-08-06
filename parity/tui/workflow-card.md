# Workflow card

The workflow card is the TUI projection of the Pi-core workflow lifecycle.
Its state is owned by the scrollback actor and is reduced only from
`WorkflowStarted`, `WorkflowProgress`, and `WorkflowFinished` events.

Grok-visible variants covered by the Runie YAML runner are running, done,
failed, and cancelled. Each card preserves its `run_id`, name, objective,
phase trail, active-agent count, terminal status, and elapsed duration. The
card is replaced in place by `run_id`, so progress does not create duplicate
transcript entries.

Fixtures: `visual-workflow-lifecycle.yaml` and
`visual-workflow-terminal-states.yaml`.
